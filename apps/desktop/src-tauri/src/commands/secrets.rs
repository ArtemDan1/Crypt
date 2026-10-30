#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::lifecycle::{create_vault, generate_seed, lock, set_settings},
        error::ApiError,
        state::{AppState, Settings},
    };
    fn state() -> (tempfile::TempDir, AppState, RecordId) {
        let d = tempfile::tempdir().unwrap();
        let st = AppState::new(
            d.path().join("vault"),
            d.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        create_vault(&st, generate_seed()).unwrap();
        let id = with_vault(&st, |v| {
            v.create_record(crypt_core::NewRecord {
                title: "login".into(),
                secret_kind: Some(SecretKind::Login),
                ..Default::default()
            })
        })
        .unwrap();
        (d, st, id)
    }
    #[test]
    fn overview_hides_sensitive_values() {
        let (_d, st, id) = state();
        for (name, value) in [
            ("username", "anton"),
            ("password", "p@ss"),
            ("url", "https://x"),
            ("totp_seed", "GEZDGNBVGY3TQOJQ"),
        ] {
            set_value(&st, &id, name, Some(value)).unwrap();
        }
        let o = overview(&st, &id).unwrap();
        assert_eq!(o.filled, ["password", "totp_seed"]);
        assert!(o.has_totp);
        let json = serde_json::to_string(&o).unwrap();
        assert!(json.contains("anton"));
        assert!(json.contains("https://x"));
        assert!(!json.contains("p@ss"));
        assert!(!json.contains("GEZD"));
        assert!(reveal(&st, &id, "totp_seed").is_err());
    }
    #[test]
    fn private_key_only_via_set_ssh_key() {
        let (_d, st, id) = state();
        assert_eq!(
            set_value(&st, &id, "private_key", Some("bad"))
                .unwrap_err()
                .code,
            "Validation"
        );
        let ssh = with_vault(&st, |v| {
            v.create_record(crypt_core::NewRecord {
                secret_kind: Some(SecretKind::SshKey),
                ..Default::default()
            })
        })
        .unwrap();
        assert!(set_value(&st, &ssh, "public_key", Some("fake")).is_err());
        let (info, pem) = crypt_core::ssh::generate_ed25519("test").unwrap();
        let got = with_vault(&st, |v| v.set_ssh_private_key(&ssh, &pem, None)).unwrap();
        assert_eq!(got.fingerprint, info.fingerprint);
    }
    #[test]
    fn locked_vault_rejects_secret_commands() {
        let (_d, st, id) = state();
        lock(&st);
        assert_eq!(overview(&st, &id).unwrap_err().code, "Locked");
        assert_eq!(reveal(&st, &id, "password").unwrap_err().code, "Locked");
        assert_eq!(copy_value(&st, &id, "password").unwrap_err().code, "Locked");
    }
    #[test]
    fn settings_validate_new_fields() {
        let (_d, st, _) = state();
        let s: Settings = serde_json::from_str(r#"{"autolockMinutes":5}"#).unwrap();
        assert_eq!(s.clipboard_clear_seconds, 30);
        assert_eq!(s.agent_trust_minutes, 0);
        assert!(
            set_settings(
                &st,
                Settings {
                    clipboard_clear_seconds: 20,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            set_settings(
                &st,
                Settings {
                    agent_trust_minutes: 7,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn api_errors_never_contain_values() {
        let (_d, st, id) = state();
        set_value(&st, &id, "password", Some("private123")).unwrap();
        let e = reveal(&st, &id, "missing").unwrap_err();
        assert!(!e.message.contains("private123"));
        let e: ApiError = crypt_core::CryptError::WrongPassphrase.into();
        assert_eq!(e.message, "Неверный пароль ключа");
    }
    #[test]
    fn revision_metadata_and_reveal_are_separate() {
        let (_d, st, id) = state();
        set_value(&st, &id, "password", Some("old secret")).unwrap();
        set_value(&st, &id, "password", Some("new secret")).unwrap();
        let h = history(&st, &id, "password").unwrap();
        assert_eq!(h.len(), 1);
        assert!(!serde_json::to_string(&h).unwrap().contains("secret"));
        assert_eq!(revision(&st, &id, "password", 0).unwrap(), "old secret");
        assert!(revision(&st, &id, "password", 1).is_err());
    }
}

use crate::{
    clipboard::copy_with_clear, commands::library::with_vault, error::ApiResult, state::AppState,
};
use crypt_core::{RecordId, SecretKind, r#gen::PasswordOptions, ssh::SshKeyInfo, totp::TotpCode};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SecretOverview {
    pub kind: SecretKind,
    pub public: BTreeMap<String, String>,
    pub filled: Vec<String>,
    pub history_counts: BTreeMap<String, usize>,
    pub ssh: Option<SshKeyInfo>,
    pub agent_enabled: bool,
    pub has_totp: bool,
}
pub fn is_public(kind: SecretKind, name: &str) -> bool {
    match kind {
        SecretKind::Login => matches!(name, "username" | "url"),
        SecretKind::Token => name == "url",
        SecretKind::SshKey => matches!(name, "public_key" | "comment"),
        SecretKind::Card => matches!(name, "holder" | "expiry"),
        SecretKind::Note => false,
    }
}
pub fn overview(st: &AppState, id: &RecordId) -> ApiResult<SecretOverview> {
    with_vault(st, |v| {
        let kind = v
            .get(id)?
            .secret_kind
            .ok_or_else(|| crypt_core::CryptError::Validation("not a secret".into()))?;
        let mut public = BTreeMap::new();
        let mut filled = Vec::new();
        let mut history_counts = BTreeMap::new();
        let mut has_totp = false;
        for name in kind.value_names() {
            let value = v.secret_value(id, name)?;
            if is_public(kind, name) {
                public.insert(
                    (*name).into(),
                    value.map(|s| s.to_string()).unwrap_or_default(),
                );
            } else if value.is_some_and(|s| !s.is_empty()) {
                filled.push((*name).into());
                if *name == "totp_seed" {
                    has_totp = true;
                }
            }
            history_counts.insert((*name).into(), v.secret_history(id, name)?.len());
        }
        Ok(SecretOverview {
            kind,
            public,
            filled,
            history_counts,
            ssh: if kind == SecretKind::SshKey {
                v.ssh_key_info(id)?
            } else {
                None
            },
            agent_enabled: v.agent_enabled(id)?,
            has_totp,
        })
    })
}
fn allow_value(name: &str) -> crypt_core::Result<()> {
    if name == "totp_seed" {
        Err(crypt_core::CryptError::Validation(
            "ключ TOTP доступен только для генерации кода".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn reveal(st: &AppState, id: &RecordId, name: &str) -> ApiResult<String> {
    with_vault(st, |v| {
        allow_value(name)?;
        Ok(v.secret_value(id, name)?
            .map(|s| s.to_string())
            .unwrap_or_default())
    })
}
pub fn copy_value(st: &AppState, id: &RecordId, name: &str) -> ApiResult<()> {
    with_vault(st, |v| {
        allow_value(name)?;
        let value = v.secret_value(id, name)?.unwrap_or_default();
        copy_with_clear(st.clipboard.clone(), &value, clear_after(st));
        Ok(())
    })
}
fn clear_after(st: &AppState) -> Duration {
    Duration::from_secs(
        st.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clipboard_clear_seconds
            .into(),
    )
}
pub fn set_value(st: &AppState, id: &RecordId, name: &str, value: Option<&str>) -> ApiResult<()> {
    with_vault(st, |v| {
        if name == "private_key"
            || (name == "public_key" && v.get(id)?.secret_kind == Some(SecretKind::SshKey))
        {
            return Err(crypt_core::CryptError::Validation(
                "используйте set_ssh_key".into(),
            ));
        }
        if name == "totp_seed"
            && let Some(s) = value.filter(|s| !s.trim().is_empty())
        {
            crypt_core::totp::parse_totp(s)?;
        }
        v.set_secret_value(id, name, value)
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionMeta {
    pub index: usize,
    pub replaced_at: i64,
}
pub fn history(st: &AppState, id: &RecordId, name: &str) -> ApiResult<Vec<RevisionMeta>> {
    with_vault(st, |v| {
        Ok(v.secret_history(id, name)?
            .iter()
            .enumerate()
            .map(|(index, r)| RevisionMeta {
                index,
                replaced_at: r.replaced_at,
            })
            .collect())
    })
}
pub fn revision(st: &AppState, id: &RecordId, name: &str, index: usize) -> ApiResult<String> {
    with_vault(st, |v| {
        allow_value(name)?;
        v.secret_history(id, name)?
            .get(index)
            .map(|r| r.value.to_string())
            .ok_or_else(|| crypt_core::CryptError::Validation("версия не найдена".into()))
    })
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub mod cmd {
    use super::*;
    use tauri::State;
    #[tauri::command]
    pub async fn secret_overview(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
    ) -> ApiResult<SecretOverview> {
        overview(&st, &id)
    }
    #[tauri::command]
    pub async fn reveal_secret(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        name: String,
    ) -> ApiResult<String> {
        reveal(&st, &id, &name)
    }
    #[tauri::command]
    pub async fn copy_secret(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        name: String,
    ) -> ApiResult<()> {
        copy_value(&st, &id, &name)
    }
    #[tauri::command]
    pub async fn set_secret(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        name: String,
        value: Option<String>,
    ) -> ApiResult<()> {
        let value = value.map(Zeroizing::new);
        set_value(&st, &id, &name, value.as_deref().map(String::as_str))
    }
    #[tauri::command]
    pub async fn secret_history(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        name: String,
    ) -> ApiResult<Vec<RevisionMeta>> {
        history(&st, &id, &name)
    }
    #[tauri::command]
    pub async fn reveal_secret_revision(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        name: String,
        index: usize,
    ) -> ApiResult<String> {
        revision(&st, &id, &name, index)
    }
    #[tauri::command]
    pub async fn copy_secret_revision(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        name: String,
        index: usize,
    ) -> ApiResult<()> {
        let value = Zeroizing::new(revision(&st, &id, &name, index)?);
        copy_with_clear(st.clipboard.clone(), &value, clear_after(&st));
        Ok(())
    }
    #[tauri::command]
    pub async fn generate_password(
        st: State<'_, std::sync::Arc<AppState>>,
        options: PasswordOptions,
    ) -> ApiResult<String> {
        with_vault(&st, |_| {
            Ok(crypt_core::r#gen::generate_password(&options)?.to_string())
        })
    }
    #[tauri::command]
    pub async fn totp_now(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
    ) -> ApiResult<Option<TotpCode>> {
        with_vault(&st, |v| v.totp_code(&id, now()))
    }
    #[tauri::command]
    pub async fn copy_totp(st: State<'_, std::sync::Arc<AppState>>, id: RecordId) -> ApiResult<()> {
        with_vault(&st, |v| {
            if let Some(code) = v.totp_code(&id, now())? {
                copy_with_clear(st.clipboard.clone(), &code.code, clear_after(&st));
            }
            Ok(())
        })
    }
    #[tauri::command]
    pub async fn set_ssh_key(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        pem: String,
        passphrase: Option<String>,
    ) -> ApiResult<SshKeyInfo> {
        let pem = Zeroizing::new(pem);
        let password = passphrase.map(Zeroizing::new);
        {
            let result = with_vault(&st, |v| {
                v.set_ssh_private_key(&id, &pem, password.as_deref().map(String::as_str))
            })?;
            crate::agent::cache::refresh(&st);
            Ok(result)
        }
    }
    #[tauri::command]
    pub async fn generate_ssh_key(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
    ) -> ApiResult<SshKeyInfo> {
        {
            let result = with_vault(&st, |v| {
                let title = v.get(&id)?.title;
                let (_, key) = crypt_core::ssh::generate_ed25519(&format!("{title}@crypt"))?;
                v.set_ssh_private_key(&id, &key, None)
            })?;
            crate::agent::cache::refresh(&st);
            Ok(result)
        }
    }
    #[tauri::command]
    pub async fn set_agent_enabled(
        st: State<'_, std::sync::Arc<AppState>>,
        id: RecordId,
        enabled: bool,
    ) -> ApiResult<()> {
        {
            with_vault(&st, |v| v.set_agent_enabled(&id, enabled))?;
            st.agent.policy.clear_trust();
            crate::agent::cache::refresh(&st);
            Ok(())
        }
    }
}
