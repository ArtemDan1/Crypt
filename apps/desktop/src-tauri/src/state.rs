use crate::unlock::{Unlocker, authenticator::Authenticator, keystore::KeyStore};
use crypt_core::Vault;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub autolock_minutes: u32,
    pub clipboard_clear_seconds: u32,
    pub agent_enabled: bool,
    pub agent_trust_minutes: u32,
    pub launch_at_login: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            autolock_minutes: 15,
            clipboard_clear_seconds: 30,
            agent_enabled: true,
            agent_trust_minutes: 0,
            launch_at_login: false,
        }
    }
}
impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)
    }
}

pub struct AppState {
    pub recent: crate::recent::RecentSecrets,
    pub app: Mutex<Option<tauri::AppHandle>>,
    pub agent: crate::agent::server::AgentHandle,
    pub agent_error: Mutex<Option<String>>,
    pub settings_update: Mutex<()>,
    pub clipboard: std::sync::Arc<dyn crate::clipboard::Clipboard>,
    pub vault: Mutex<Option<Vault>>,
    pub vault_dir: PathBuf,
    pub ssh_dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub keys: Box<dyn KeyStore>,
    pub auth: Box<dyn Authenticator>,
    /// Set while a graceful lock waits for the frontend to flush pending saves.
    pub flush_waiter: Mutex<Option<mpsc::Sender<()>>>,
}
impl AppState {
    pub fn new(
        vault_dir: PathBuf,
        settings_path: PathBuf,
        keys: Box<dyn KeyStore>,
        auth: Box<dyn Authenticator>,
    ) -> Self {
        let settings = Settings::load(&settings_path);
        Self {
            recent: Default::default(),
            app: Mutex::new(None),
            agent: Default::default(),
            agent_error: Mutex::new(None),
            settings_update: Mutex::new(()),
            clipboard: std::sync::Arc::new(crate::clipboard::MacClipboard),
            vault: Mutex::new(None),
            ssh_dir: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".ssh"),
            vault_dir,
            settings: Mutex::new(settings),
            settings_path,
            keys,
            auth,
            flush_waiter: Mutex::new(None),
        }
    }
    pub fn unlocker(&self) -> Unlocker<'_> {
        Unlocker::new(self.keys.as_ref(), self.auth.as_ref())
    }
    pub fn vault_guard(&self) -> std::sync::MutexGuard<'_, Option<Vault>> {
        self.vault.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl AppState {
    pub fn emit<T: serde::Serialize + Clone>(&self, event: &str, value: T) {
        use tauri::Emitter;
        if let Some(app) = self.app.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = app.emit(event, value);
        }
    }
    pub fn socket_path(&self) -> PathBuf {
        self.settings_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("agent.sock")
    }
}
