use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, Settings};
use crypt_core::{SeedPhrase, Vault};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum VaultStatus {
    NoVault,
    #[serde(rename_all = "camelCase")]
    Locked {
        vault_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Unlocked {
        vault_id: String,
    },
}

/// `vault_id` lives in the plaintext header, so it is readable while locked.
pub fn header_vault_id(st: &AppState) -> Option<String> {
    let bytes = std::fs::read(st.vault_dir.join("vault.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("vault_id")?.as_str().map(str::to_owned)
}

pub fn status(st: &AppState) -> VaultStatus {
    if let Some(v) = st.vault_guard().as_ref() {
        return VaultStatus::Unlocked {
            vault_id: v.vault_id().to_string(),
        };
    }
    match header_vault_id(st) {
        Some(vault_id) => VaultStatus::Locked { vault_id },
        None => VaultStatus::NoVault,
    }
}

pub fn generate_seed() -> Vec<String> {
    SeedPhrase::generate()
        .words()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn parse_words(words: &[String]) -> ApiResult<SeedPhrase> {
    Ok(SeedPhrase::parse(&words.join(" "))?)
}

/// Put an opened vault into the state and report its status.
pub fn install(st: &AppState, vault: Vault) -> VaultStatus {
    let vault_id = vault.vault_id().to_string();
    *st.vault_guard() = Some(vault);
    crate::agent::cache::refresh(st);
    let result = VaultStatus::Unlocked { vault_id };
    st.emit("vault-unlocked", result.clone());
    result
}

pub fn create_vault(st: &AppState, words: Vec<String>) -> ApiResult<VaultStatus> {
    let seed = parse_words(&words)?;
    let vault = Vault::create(&st.vault_dir, &seed)?;
    Ok(install(st, vault))
}

pub fn unlock_with_seed(st: &AppState, words: Vec<String>) -> ApiResult<VaultStatus> {
    let seed = parse_words(&words)?;
    let vault = Vault::open_with_seed(&st.vault_dir, &seed)?;
    st.unlocker()
        .reset_attempts(&vault.vault_id().to_string())?;
    Ok(install(st, vault))
}

pub fn lock(st: &AppState) -> VaultStatus {
    st.agent.policy.clear_trust();
    {
        let mut guard = st.vault_guard();
        if let Some(v) = guard.take() { v.lock(); }
        st.recent.clear();
    }
    let status = status(st);
    st.emit("vault-locked", status.clone());
    st.emit("agent://changed", ());
    status
}

pub fn delete_vault(
    st: &AppState,
    confirmation: &str,
    pin: Option<&str>,
) -> ApiResult<VaultStatus> {
    if confirmation != "УДАЛИТЬ" {
        return Err(ApiError::new(
            "Validation",
            "Введите УДАЛИТЬ для подтверждения",
        ));
    }
    let mut guard = st.vault_guard();
    let vault = guard
        .as_ref()
        .ok_or_else(|| ApiError::new("Locked", "Хранилище заблокировано"))?;
    let id = vault.vault_id().to_string();
    st.unlocker().confirm_deletion(&id, pin)?;
    // Close before any mutation, retaining the mutex so writes cannot race removal.
    if let Some(v) = guard.take() {
        v.lock();
    }
    st.agent.policy.clear_trust();
    st.keys.delete(&format!("agent-keys:{id}"))?;
    st.unlocker().disable_biometric(&id)?;
    st.unlocker().remove_pin(&id)?;
    std::fs::remove_dir_all(&st.vault_dir)
        .map_err(|_| ApiError::new("Io", "Не удалось удалить хранилище"))?;
    drop(guard);
    st.emit("agent://changed", ());
    Ok(VaultStatus::NoVault)
}

pub fn get_settings(st: &AppState) -> Settings {
    st.settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn validate_settings(s: &Settings) -> ApiResult<()> {
    if ![1, 5, 15, 30, 60].contains(&s.autolock_minutes) {
        return Err(ApiError::new(
            "Validation",
            "Недопустимый интервал автоблокировки",
        ));
    }
    if ![15, 30, 60, 90].contains(&s.clipboard_clear_seconds)
        || ![0, 1, 5, 15].contains(&s.agent_trust_minutes)
    {
        return Err(ApiError::new("Validation", "Недопустимый интервал"));
    }
    Ok(())
}

pub fn set_settings(st: &AppState, s: Settings) -> ApiResult<Settings> {
    validate_settings(&s)?;
    s.save(&st.settings_path)
        .map_err(|_| ApiError::new("Io", "Не удалось сохранить настройки"))?;
    *st.settings.lock().unwrap_or_else(|e| e.into_inner()) = s.clone();
    Ok(s)
}

pub fn set_settings_with_effects(
    st: &AppState,
    settings: Settings,
    mut agent: impl FnMut(bool) -> ApiResult<()>,
    mut autostart: impl FnMut(bool) -> ApiResult<()>,
) -> ApiResult<Settings> {
    let _update = st.settings_update.lock().unwrap_or_else(|e| e.into_inner());
    let old = get_settings(st);
    validate_settings(&settings)?;
    let agent_changed = old.agent_enabled != settings.agent_enabled;
    let launch_changed = old.launch_at_login != settings.launch_at_login;
    if agent_changed && let Err(error) = agent(settings.agent_enabled) {
        let _ = agent(old.agent_enabled);
        return Err(error);
    }
    if launch_changed && let Err(error) = autostart(settings.launch_at_login) {
        let _ = autostart(old.launch_at_login);
        if agent_changed {
            let _ = agent(old.agent_enabled);
        }
        return Err(error);
    }
    let saved = match set_settings(st, settings) {
        Ok(saved) => saved,
        Err(error) => {
            if launch_changed {
                let _ = autostart(old.launch_at_login);
            }
            if agent_changed {
                let _ = agent(old.agent_enabled);
            }
            return Err(error);
        }
    };
    if old.agent_trust_minutes != saved.agent_trust_minutes {
        st.agent.policy.clear_trust();
    }
    st.emit("agent://changed", ());
    Ok(saved)
}

pub mod cmd {
    #[tauri::command]
    pub fn show(app: tauri::AppHandle) { crate::tray::show(&app); }
    #[tauri::command]
    pub fn tray_hide(app: tauri::AppHandle) {
        use tauri::Manager;
        if let Some(w) = app.get_webview_window("tray") { let _ = w.hide(); }
    }
    #[tauri::command]
    pub fn app_quit(app: tauri::AppHandle) { app.exit(0); }

    use super::*;

    #[tauri::command]
    pub async fn delete_vault(
        app: tauri::AppHandle,
        confirmation: String,
        pin: Option<String>,
    ) -> ApiResult<VaultStatus> {
        use tauri::Manager;
        tauri::async_runtime::spawn_blocking(move || {
            super::delete_vault(
                &app.state::<std::sync::Arc<AppState>>(),
                &confirmation,
                pin.as_deref(),
            )
        })
        .await
        .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
    }
    #[tauri::command]
    pub fn print_seed(window: tauri::WebviewWindow) -> ApiResult<()> {
        window
            .print()
            .map_err(|_| ApiError::new("Print", "Не удалось открыть диалог печати"))
    }
    #[tauri::command]
    pub fn status(st: State<'_, std::sync::Arc<AppState>>) -> VaultStatus {
        super::status(&st)
    }
    #[tauri::command]
    pub fn generate_seed() -> Vec<String> {
        super::generate_seed()
    }
    #[tauri::command]
    pub async fn create_vault(
        st: State<'_, std::sync::Arc<AppState>>,
        words: Vec<String>,
    ) -> ApiResult<VaultStatus> {
        super::create_vault(&st, words)
    }
    #[tauri::command]
    pub async fn unlock_with_seed(
        st: State<'_, std::sync::Arc<AppState>>,
        words: Vec<String>,
    ) -> ApiResult<VaultStatus> {
        super::unlock_with_seed(&st, words)
    }
    #[tauri::command]
    pub fn lock(st: State<'_, std::sync::Arc<AppState>>) -> VaultStatus {
        super::lock(&st)
    }
    #[tauri::command]
    pub fn vault_path(st: State<'_, std::sync::Arc<AppState>>) -> String {
        st.vault_dir.display().to_string()
    }
    #[tauri::command]
    pub fn get_settings(st: State<'_, std::sync::Arc<AppState>>) -> Settings {
        super::get_settings(&st)
    }
    #[tauri::command]
    pub fn set_settings(
        st: State<'_, std::sync::Arc<AppState>>,
        settings: Settings,
    ) -> ApiResult<Settings> {
        super::set_settings_with_effects(
            &st,
            settings,
            |enabled| crate::agent::set_running(st.inner().clone(), enabled),
            |enabled| {
                use tauri_plugin_autostart::ManagerExt;
                let handle = st.app.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if let Some(app) = handle {
                    let result = if enabled {
                        app.autolaunch().enable()
                    } else {
                        app.autolaunch().disable()
                    };
                    result.map_err(|_| ApiError::new("Autostart", "Не удалось изменить автозапуск"))
                } else {
                    Ok(())
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(
            dir.path().join("vault"),
            dir.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        (dir, st)
    }

    #[test]
    fn status_no_vault_then_unlocked_then_locked() {
        let (_d, st) = state();
        assert!(matches!(status(&st), VaultStatus::NoVault));
        let words = generate_seed();
        assert_eq!(words.len(), 24);
        let id = match create_vault(&st, words).unwrap() {
            VaultStatus::Unlocked { vault_id } => vault_id,
            other => panic!("{other:?}"),
        };
        match lock(&st) {
            VaultStatus::Locked { vault_id } => assert_eq!(vault_id, id),
            other => panic!("{other:?}"),
        }
        assert!(matches!(status(&st), VaultStatus::Locked { .. }));
    }

    #[test]
    fn unlock_with_wrong_seed_maps_to_wrong_key() {
        let (_d, st) = state();
        create_vault(&st, generate_seed()).unwrap();
        lock(&st);
        let other = generate_seed();
        let err = unlock_with_seed(&st, other.clone()).unwrap_err();
        assert_eq!(err.code, "WrongKey");
        assert!(other.iter().all(|w| !err.message.contains(w.as_str())));
    }

    #[test]
    fn unlock_with_right_seed_works() {
        let (_d, st) = state();
        let words = generate_seed();
        create_vault(&st, words.clone()).unwrap();
        lock(&st);
        assert!(matches!(
            unlock_with_seed(&st, words).unwrap(),
            VaultStatus::Unlocked { .. }
        ));
    }

    #[test]
    fn seed_unlock_resets_pin_attempts() {
        let (_d, st) = state();
        let words = generate_seed();
        create_vault(&st, words.clone()).unwrap();
        {
            let g = st.vault_guard();
            st.unlocker()
                .set_pin(g.as_ref().unwrap(), "482913")
                .unwrap();
        }
        let id = match lock(&st) {
            VaultStatus::Locked { vault_id } => vault_id,
            s => panic!("{s:?}"),
        };
        st.unlocker()
            .unlock_pin(&st.vault_dir, &id, "000000")
            .unwrap_err();
        assert_eq!(st.unlocker().methods(&id).unwrap().attempts_left, None);
        unlock_with_seed(&st, words).unwrap();
        assert_eq!(st.unlocker().methods(&id).unwrap().attempts_left, Some(5));
    }

    #[test]
    fn create_twice_is_vault_exists() {
        let (_d, st) = state();
        create_vault(&st, generate_seed()).unwrap();
        assert_eq!(
            create_vault(&st, generate_seed()).unwrap_err().code,
            "VaultExists"
        );
    }

    #[test]
    fn set_settings_rejects_odd_intervals() {
        let (_d, st) = state();
        assert_eq!(
            set_settings(
                &st,
                Settings {
                    autolock_minutes: 7,
                    ..Default::default()
                }
            )
            .unwrap_err()
            .code,
            "Validation"
        );
        assert_eq!(
            set_settings(
                &st,
                Settings {
                    autolock_minutes: 30,
                    ..Default::default()
                }
            )
            .unwrap()
            .autolock_minutes,
            30
        );
        assert_eq!(Settings::load(&st.settings_path).autolock_minutes, 30);
    }

    #[test]
    fn deletion_requires_confirmation_and_fresh_auth() {
        let (_d, st) = state();
        create_vault(&st, generate_seed()).unwrap();
        {
            let g = st.vault_guard();
            st.unlocker()
                .set_pin(g.as_ref().unwrap(), "482913")
                .unwrap();
            st.unlocker().enable_biometric(g.as_ref().unwrap()).unwrap();
        }
        let id = header_vault_id(&st).unwrap();
        std::fs::write(st.vault_dir.parent().unwrap().join("keep.txt"), "keep").unwrap();
        assert_eq!(
            delete_vault(&st, "", Some("482913")).unwrap_err().code,
            "Validation"
        );
        assert_eq!(
            delete_vault(&st, "УДАЛИТЬ", Some("000000"))
                .unwrap_err()
                .code,
            "WrongPin"
        );
        assert!(st.vault_dir.join("vault.json").exists());
        assert_eq!(
            delete_vault(&st, "УДАЛИТЬ", Some("482913")).unwrap(),
            VaultStatus::NoVault
        );
        assert!(!st.vault_dir.exists());
        assert!(st.vault_dir.parent().unwrap().join("keep.txt").exists());
        let m = st.unlocker().methods(&id).unwrap();
        assert!(!m.pin && !m.biometric);
        create_vault(&st, generate_seed()).unwrap();
        lock(&st);
        assert_eq!(
            delete_vault(&st, "УДАЛИТЬ", None).unwrap_err().code,
            "Locked"
        );
    }

    #[test]
    fn deletion_io_failure_closes_the_live_vault() {
        let (_d, st) = state();
        create_vault(&st, generate_seed()).unwrap();
        std::fs::rename(&st.vault_dir, st.vault_dir.with_extension("backup")).unwrap();
        std::fs::write(&st.vault_dir, "not a directory").unwrap();
        assert_eq!(delete_vault(&st, "УДАЛИТЬ", None).unwrap_err().code, "Io");
        assert!(st.vault_guard().is_none());
    }

    #[test]
    fn settings_default_and_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        assert_eq!(Settings::load(&p).autolock_minutes, 15);
        Settings {
            autolock_minutes: 5,
            ..Default::default()
        }
        .save(&p)
        .unwrap();
        assert_eq!(Settings::load(&p).autolock_minutes, 5);
    }
}

#[cfg(test)]
mod runtime_settings_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn failed_autostart_rolls_back_agent_and_preferences() {
        let d = tempfile::tempdir().unwrap();
        let st = AppState::new(
            d.path().join("vault"),
            d.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        let old = Settings {
            agent_enabled: false,
            ..Default::default()
        };
        set_settings(&st, old.clone()).unwrap();
        let running = AtomicBool::new(false);
        let result = set_settings_with_effects(
            &st,
            Settings {
                agent_enabled: true,
                launch_at_login: true,
                ..old.clone()
            },
            |enabled| {
                running.store(enabled, Ordering::SeqCst);
                Ok(())
            },
            |enabled| {
                if enabled {
                    Err(ApiError::new("Autostart", "failed"))
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert!(!running.load(Ordering::SeqCst));
        assert_eq!(get_settings(&st), old);
        assert_eq!(Settings::load(&st.settings_path), old);
    }
}
