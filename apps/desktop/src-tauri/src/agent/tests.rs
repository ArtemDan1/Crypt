use super::{policy::*, protocol::*, server::*};
use crate::{
    commands::lifecycle::{create_vault, generate_seed, lock},
    state::AppState,
    unlock::authenticator::{AuthError, Authenticator},
};
use crypt_core::{NewRecord, RecordId, SecretKind};
use signature::Verifier;
use std::{
    io::Write,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
struct Auth {
    calls: AtomicUsize,
    active: AtomicUsize,
    max: AtomicUsize,
    cancel: AtomicBool,
    delay: Duration,
}
impl Auth {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            max: AtomicUsize::new(0),
            cancel: AtomicBool::new(false),
            delay: Duration::from_millis(100),
        }
    }
}
impl Authenticator for Arc<Auth> {
    fn available(&self) -> bool {
        true
    }
    fn authenticate(&self, _: &str) -> Result<(), AuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let n = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max.fetch_max(n, Ordering::SeqCst);
        std::thread::sleep(self.delay);
        self.active.fetch_sub(1, Ordering::SeqCst);
        if self.cancel.load(Ordering::SeqCst) {
            Err(AuthError::Cancelled)
        } else {
            Ok(())
        }
    }
}
struct FakeClock {
    base: Instant,
    offset: Mutex<Duration>,
}
impl FakeClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            offset: Mutex::new(Duration::ZERO),
        }
    }
    fn advance(&self) {
        *self.offset.lock().unwrap() += Duration::from_secs(360);
    }
}
impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }
}
fn setup(
    bio: bool,
) -> (
    tempfile::TempDir,
    Arc<AppState>,
    RecordId,
    Vec<u8>,
    Arc<Auth>,
    Arc<FakeClock>,
) {
    let d = tempfile::tempdir().unwrap();
    let auth = Arc::new(Auth::new());
    let clock = Arc::new(FakeClock::new());
    let mut st = AppState::new(
        d.path().join("vault"),
        d.path().join("settings.json"),
        Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
        Box::new(auth.clone()),
    );
    st.agent = AgentHandle::new(clock.clone());
    create_vault(&st, generate_seed()).unwrap();
    let (id, blob) = {
        let mut g = st.vault_guard();
        let v = g.as_mut().unwrap();
        let id = v
            .create_record(NewRecord {
                title: "agent test".into(),
                secret_kind: Some(SecretKind::SshKey),
                ..Default::default()
            })
            .unwrap();
        let (info, pem) = crypt_core::ssh::generate_ed25519("test").unwrap();
        v.set_ssh_private_key(&id, &pem, None).unwrap();
        if bio {
            st.keys
                .set(
                    &format!("data-key:{}", v.vault_id()),
                    v.data_key().as_bytes(),
                )
                .unwrap();
        }
        (
            id,
            crypt_core::ssh::public_key_blob(&info.public_key).unwrap(),
        )
    };
    super::cache::refresh(&st);
    (d, Arc::new(st), id, blob, auth, clock)
}
fn peer() -> PeerInfo {
    PeerInfo {
        uid: unsafe { libc::geteuid() },
        pid: None,
        process: Some("test ssh".into()),
    }
}
fn request(blob: &[u8]) -> Request {
    Request::Sign {
        key_blob: blob.to_vec(),
        data: b"challenge".to_vec(),
        flags: 0,
    }
}
fn signed(st: &AppState, blob: &[u8]) {
    let r = st.agent.policy.handle(st, request(blob), &peer());
    match r {
        Response::Signature(sig) => {
            let pubkey = ssh_key::PublicKey::from_bytes(blob).unwrap();
            pubkey
                .key_data()
                .verify(
                    b"challenge",
                    &ssh_key::Signature::try_from(sig.as_slice()).unwrap(),
                )
                .unwrap();
        }
        _ => panic!("signature refused"),
    }
}
#[test]
fn identities_unlocked_and_from_cache_when_locked() {
    let (_d, st, id, _blob, _, _) = setup(true);
    assert!(
        matches!(st.agent.policy.handle(&st,Request::Identities,&peer()),Response::Identities(k) if k.len()==1)
    );
    lock(&st);
    assert!(
        matches!(st.agent.policy.handle(&st,Request::Identities,&peer()),Response::Identities(k) if k.len()==1)
    );
    let v = st
        .unlocker()
        .unlock_biometric(
            &st.vault_dir,
            &crate::commands::lifecycle::header_vault_id(&st).unwrap(),
        )
        .unwrap();
    crate::commands::lifecycle::install(&st, v);
    st.vault_guard()
        .as_mut()
        .unwrap()
        .set_agent_enabled(&id, false)
        .unwrap();
    super::cache::refresh(&st);
    lock(&st);
    assert!(
        matches!(st.agent.policy.handle(&st,Request::Identities,&peer()),Response::Identities(k) if k.is_empty())
    );
}
#[test]
fn each_sign_prompts_without_trust() {
    let (_d, st, _, blob, auth, _) = setup(true);
    signed(&st, &blob);
    signed(&st, &blob);
    assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
}
#[test]
fn trust_window_skips_prompt_until_expiry_and_lock() {
    let (_d, st, _, blob, auth, clock) = setup(true);
    st.settings.lock().unwrap().agent_trust_minutes = 5;
    signed(&st, &blob);
    signed(&st, &blob);
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
    clock.advance();
    signed(&st, &blob);
    assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
    lock(&st);
    signed(&st, &blob);
    assert_eq!(auth.calls.load(Ordering::SeqCst), 3);
}
#[test]
fn cancelled_touch_id_fails() {
    let (_d, st, _, blob, auth, _) = setup(true);
    auth.cancel.store(true, Ordering::SeqCst);
    assert!(matches!(
        st.agent.policy.handle(&st, request(&blob), &peer()),
        Response::Failure
    ));
}
#[test]
fn unknown_key_fails_without_prompt() {
    let (_d, st, _, _, auth, _) = setup(true);
    assert!(matches!(
        st.agent.policy.handle(&st, request(b"unknown"), &peer()),
        Response::Failure
    ));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn concurrent_signs_prompt_one_at_a_time() {
    for trust in [0, 5] {
        let (_d, st, _, blob, auth, _) = setup(true);
        st.settings.lock().unwrap().agent_trust_minutes = trust;
        std::thread::scope(|s| {
            for _ in 0..2 {
                s.spawn(|| signed(&st, &blob));
            }
        });
        assert_eq!(auth.max.load(Ordering::SeqCst), 1);
        assert_eq!(
            auth.calls.load(Ordering::SeqCst),
            if trust == 0 { 2 } else { 1 }
        );
    }
}
#[test]
fn locked_sign_uses_biometric_and_relocks() {
    let (_d, st, _, blob, auth, _) = setup(true);
    lock(&st);
    signed(&st, &blob);
    assert!(st.vault_guard().is_none());
    assert_eq!(auth.calls.load(Ordering::SeqCst), 1);
}
#[test]
fn locked_sign_without_biometric_fails_fast() {
    let (_d, st, _, blob, auth, _) = setup(false);
    lock(&st);
    let start = Instant::now();
    assert!(matches!(
        st.agent.policy.handle(&st, request(&blob), &peer()),
        Response::Failure
    ));
    assert!(start.elapsed() < Duration::from_millis(50));
    assert_eq!(auth.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn server_roundtrip_over_socket() {
    let (d, st, _, blob, _, _) = setup(true);
    let socket = d.path().join("agent.sock");
    st.agent.start(st.clone(), socket.clone()).unwrap();
    assert_eq!(
        std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        st.agent
            .start(st.clone(), socket.clone())
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::AddrInUse
    );
    let mut client = UnixStream::connect(&socket).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    client.write_all(&[0, 0, 0, 1, 11]).unwrap();
    let r = read_frame(&mut client).unwrap().unwrap();
    assert_eq!(&r[..5], &[12, 0, 0, 0, 1]);
    let mut body = vec![13];
    for s in [&blob[..], b"challenge"] {
        body.extend_from_slice(&(s.len() as u32).to_be_bytes());
        body.extend_from_slice(s);
    }
    body.extend_from_slice(&0u32.to_be_bytes());
    client
        .write_all(&(body.len() as u32).to_be_bytes())
        .unwrap();
    client.write_all(&body).unwrap();
    let r = read_frame(&mut client).unwrap().unwrap();
    assert_eq!(r[0], 14);
    ssh_key::PublicKey::from_bytes(&blob)
        .unwrap()
        .key_data()
        .verify(
            b"challenge",
            &ssh_key::Signature::try_from(&r[5..]).unwrap(),
        )
        .unwrap();
    st.agent.stop();
    assert!(!socket.exists());
    assert!(!st.agent.running());
}

#[test]
fn startup_socket_conflict_keeps_vault_accessible() {
    let (_d, st, _, _, _, _) = setup(false);
    std::fs::write(st.socket_path(), "unrelated file").unwrap();
    assert!(super::set_running(st.clone(), true).is_err());
    assert!(!st.agent.running());
    assert!(st.vault_guard().is_some());
    assert_eq!(
        std::fs::read_to_string(st.socket_path()).unwrap(),
        "unrelated file"
    );
    assert!(st.agent_error.lock().unwrap().is_some());
}

#[test]
fn inherited_nonblocking_stream_stays_open_between_requests() {
    let (_d, st, _, _, _, _) = setup(false);
    let (server, mut client) = UnixStream::pair().unwrap();
    server.set_nonblocking(true).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    client.write_all(&[0, 0, 0, 1, 11]).unwrap();
    let weak = Arc::downgrade(&st);
    let worker = std::thread::spawn(move || {
        super::server::connection(server, weak, Arc::new(AtomicBool::new(true)))
    });
    assert_eq!(read_frame(&mut client).unwrap().unwrap()[0], 12);
    std::thread::sleep(Duration::from_millis(50));
    client.write_all(&[0, 0, 0, 1, 11]).unwrap();
    assert_eq!(read_frame(&mut client).unwrap().unwrap()[0], 12);
    drop(client);
    worker.join().unwrap();
}
