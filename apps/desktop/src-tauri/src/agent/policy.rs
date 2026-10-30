use super::{
    cache,
    protocol::{Request, Response},
};
use crate::{commands::lifecycle::header_vault_id, state::AppState};
use crypt_core::{
    RecordId, SecretKey, Vault,
    ssh::{AgentIdentity, public_key_blob},
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
pub struct PeerInfo {
    pub uid: u32,
    pub pid: Option<i32>,
    pub process: Option<String>,
}
pub struct AgentPolicy {
    trust: Mutex<HashMap<RecordId, Instant>>,
    prompt: Mutex<()>,
    clock: Arc<dyn Clock>,
    epoch: AtomicU64,
}
impl AgentPolicy {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            trust: Mutex::new(HashMap::new()),
            prompt: Mutex::new(()),
            clock,
            epoch: AtomicU64::new(0),
        }
    }
    pub fn clear_trust(&self) {
        let mut trust = self.trust.lock().unwrap_or_else(|e| e.into_inner());
        self.epoch.fetch_add(1, Ordering::SeqCst);
        trust.clear();
    }
    fn trusted(&self, id: &RecordId, minutes: u32) -> bool {
        minutes > 0
            && self
                .trust
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(id)
                .is_some_and(|until| *until > self.clock.now())
    }
    pub fn handle(&self, st: &AppState, req: Request, peer: &PeerInfo) -> Response {
        if peer.uid != unsafe { libc::geteuid() }
            || !st
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .agent_enabled
        {
            return Response::Failure;
        }
        match req {
            Request::Identities => Response::Identities(
                identities(st)
                    .into_iter()
                    .filter_map(|i| {
                        public_key_blob(&i.public_key)
                            .ok()
                            .map(|blob| (blob, i.title))
                    })
                    .collect(),
            ),
            Request::Other(_) => Response::Failure,
            Request::Sign {
                key_blob,
                data,
                flags,
            } => {
                let Some(identity) = identities(st)
                    .into_iter()
                    .find(|i| public_key_blob(&i.public_key).is_ok_and(|b| b == key_blob))
                else {
                    return Response::Failure;
                };
                let _prompt = self.prompt.lock().unwrap_or_else(|e| e.into_inner());
                let epoch = self.epoch.load(Ordering::SeqCst);
                let minutes = st
                    .settings
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .agent_trust_minutes;
                let locked = st.vault_guard().is_none();
                let vault_id = header_vault_id(st);
                if locked
                    && (vault_id.as_ref().is_none_or(|id| {
                        !st.keys.contains(&format!("data-key:{id}")).unwrap_or(false)
                    }) || !st.auth.available())
                {
                    st.emit("agent://unlock-needed", ());
                    return Response::Failure;
                }
                if !self.trusted(&identity.id, minutes) {
                    let reason = format!(
                        "ssh: подписать ключом «{}» для {}",
                        identity.title,
                        peer.process.as_deref().unwrap_or("неизвестного процесса")
                    );
                    if st.auth.authenticate(&reason).is_err() {
                        if locked {
                            st.emit("agent://unlock-needed", ());
                        }
                        return Response::Failure;
                    }
                }
                if epoch != self.epoch.load(Ordering::SeqCst)
                    || !st
                        .settings
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .agent_enabled
                {
                    return Response::Failure;
                }
                let sign = |v: &Vault| {
                    if !v.agent_identities().iter().any(|i| {
                        i.id == identity.id
                            && public_key_blob(&i.public_key).is_ok_and(|b| b == key_blob)
                    }) {
                        return None;
                    }
                    v.sign_with(&identity.id, &data, flags).ok()
                };
                let signature = {
                    let guard = st.vault_guard();
                    if let Some(v) = guard.as_ref() {
                        sign(v)
                    } else {
                        vault_id
                            .and_then(|id| st.keys.get(&format!("data-key:{id}")).ok().flatten())
                            .and_then(|raw| {
                                let bytes: [u8; 32] = raw.as_slice().try_into().ok()?;
                                let v = Vault::open_with_key(
                                    &st.vault_dir,
                                    SecretKey::from_bytes(bytes),
                                )
                                .ok()?;
                                sign(&v)
                            })
                    }
                };
                let Some(signature) = signature else {
                    return Response::Failure;
                };
                if minutes > 0 {
                    let mut trust = self.trust.lock().unwrap_or_else(|e| e.into_inner());
                    if epoch == self.epoch.load(Ordering::SeqCst) {
                        trust.insert(
                            identity.id,
                            self.clock.now() + Duration::from_secs(minutes as u64 * 60),
                        );
                    }
                }
                Response::Signature(signature)
            }
        }
    }
}
pub fn identities(st: &AppState) -> Vec<AgentIdentity> {
    let guard = st.vault_guard();
    if let Some(v) = guard.as_ref() {
        v.agent_identities()
    } else {
        header_vault_id(st)
            .map(|id| cache::load(st.keys.as_ref(), &id))
            .unwrap_or_default()
    }
}
