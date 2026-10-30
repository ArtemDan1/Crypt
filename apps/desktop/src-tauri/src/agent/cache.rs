use crate::state::AppState;
use crate::unlock::keystore::KeyStore;
use crypt_core::ssh::AgentIdentity;
pub fn refresh(st: &AppState) {
    {
        let guard = st.vault_guard();
        if let Some(v) = guard.as_ref()
            && let Ok(bytes) = serde_json::to_vec(&v.agent_identities())
        {
            let _ = st.keys.set(&format!("agent-keys:{}", v.vault_id()), &bytes);
        }
    }
    st.emit("agent://changed", ());
}
pub fn load(keys: &dyn KeyStore, vault_id: &str) -> Vec<AgentIdentity> {
    keys.get(&format!("agent-keys:{vault_id}"))
        .ok()
        .flatten()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
