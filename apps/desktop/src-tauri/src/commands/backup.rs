use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};
use crypt_core::backup::{BackupHeader, write_backup};
use serde::Serialize;
use std::{io::Write, path::Path};
#[derive(Debug, Serialize)]
pub struct BackupExport {
    pub path: String,
    pub bytes: u64,
    pub header: BackupHeader,
}
pub fn export(st: &AppState, path: &Path, pin: Option<&str>) -> ApiResult<BackupExport> {
    let id = crate::commands::lifecycle::header_vault_id(st)
        .ok_or_else(|| ApiError::new("NotAVault", "Хранилище не найдено"))?;
    st.unlocker().confirm_presence(&id, pin)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64;
    let (header, bytes) = {
        let guard = st.vault_guard();
        let _vault = guard
            .as_ref()
            .ok_or_else(|| ApiError::new("Locked", "Хранилище заблокировано"))?;
        if path.starts_with(&st.vault_dir) {
            return Err(ApiError::new(
                "Validation",
                "Сохраните бэкап вне папки хранилища",
            ));
        }
        let mut name = path.as_os_str().to_os_string();
        name.push(".tmp");
        let temp = std::path::PathBuf::from(name);
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts.open(&temp).map_err(crypt_core::CryptError::from)?;
        let result = (|| -> crypt_core::Result<_> {
            let h = write_backup(&st.vault_dir, &mut file, now)?;
            file.flush()?;
            file.sync_all()?;
            let bytes = file.metadata()?.len();
            std::fs::rename(&temp, path)?;
            Ok((h, bytes))
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result?
    };
    let _update = st.settings_update.lock().unwrap_or_else(|e| e.into_inner());
    let mut settings = st.settings.lock().unwrap_or_else(|e| e.into_inner());
    let mut next = settings.clone();
    next.last_backup_at = Some(now);
    next.save(&st.settings_path)
        .map_err(crypt_core::CryptError::from)?;
    *settings = next;
    Ok(BackupExport {
        path: path.to_string_lossy().into_owned(),
        bytes,
        header,
    })
}
#[tauri::command]
pub async fn export_backup(
    st: tauri::State<'_, std::sync::Arc<AppState>>,
    path: String,
    pin: Option<String>,
) -> ApiResult<BackupExport> {
    let state = st.inner().clone();
    let pin = pin.map(zeroize::Zeroizing::new);
    tauri::async_runtime::spawn_blocking(move || {
        export(&state, Path::new(&path), pin.as_deref().map(String::as_str))
    })
    .await
    .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}

fn read_backup_file(path: &Path) -> ApiResult<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(crypt_core::CryptError::from)?;
    if !file
        .metadata()
        .map_err(crypt_core::CryptError::from)?
        .is_file()
    {
        return Err(ApiError::new("BackupFormat", "Это не файл бэкапа Crypt"));
    }
    let mut bytes = Vec::new();
    file.take(crypt_core::backup::MAX_BACKUP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(crypt_core::CryptError::from)?;
    if bytes.len() as u64 > crypt_core::backup::MAX_BACKUP_BYTES {
        return Err(ApiError::new("BackupFormat", "Файл бэкапа больше 2 ГБ"));
    }
    Ok(bytes)
}
pub fn restore_file(
    st: &AppState,
    path: &Path,
    words: Vec<String>,
) -> ApiResult<crate::commands::lifecycle::VaultStatus> {
    let words = zeroize::Zeroizing::new(words);
    let phrase = zeroize::Zeroizing::new(words.join(" "));
    let seed = crypt_core::SeedPhrase::parse(&phrase)?;
    let guard = st.vault_guard();
    if guard.is_some() || st.vault_dir.exists() {
        return Err(crypt_core::CryptError::VaultExists.into());
    }
    let bytes = read_backup_file(path)?;
    crypt_core::backup::restore_backup(&bytes, &seed, &st.vault_dir)?;
    let vault = crypt_core::Vault::open_with_seed(&st.vault_dir, &seed)?;
    drop(guard);
    Ok(crate::commands::lifecycle::install(st, vault))
}
#[derive(Serialize)]
pub struct BackupInspection {
    #[serde(flatten)]
    pub header: BackupHeader,
    pub bytes: u64,
}
pub fn inspect_file(path: &Path) -> ApiResult<BackupInspection> {
    let mut file = std::fs::File::open(path).map_err(crypt_core::CryptError::from)?;
    let meta = file.metadata().map_err(crypt_core::CryptError::from)?;
    if !meta.is_file() || meta.len() > crypt_core::backup::MAX_BACKUP_BYTES {
        return Err(ApiError::new(
            "BackupFormat",
            "Файл не подходит для восстановления",
        ));
    }
    Ok(BackupInspection {
        header: crypt_core::backup::read_backup_header(&mut file)?,
        bytes: meta.len(),
    })
}
#[tauri::command]
pub async fn inspect_backup(path: String) -> ApiResult<BackupInspection> {
    tauri::async_runtime::spawn_blocking(move || inspect_file(Path::new(&path)))
        .await
        .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}
#[tauri::command]
pub async fn restore_backup(
    st: tauri::State<'_, std::sync::Arc<AppState>>,
    path: String,
    words: Vec<String>,
) -> ApiResult<crate::commands::lifecycle::VaultStatus> {
    let state = st.inner().clone();
    tauri::async_runtime::spawn_blocking(move || restore_file(&state, Path::new(&path), words))
        .await
        .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::lifecycle::{create_vault, generate_seed},
        unlock::{
            authenticator::{AuthError, FakeAuthenticator},
            keystore::MemoryKeyStore,
        },
    };
    fn state(auth: FakeAuthenticator) -> (tempfile::TempDir, AppState) {
        let d = tempfile::tempdir().unwrap();
        let st = AppState::new(
            d.path().join("vault"),
            d.path().join("settings.json"),
            Box::new(MemoryKeyStore::default()),
            Box::new(auth),
        );
        create_vault(&st, generate_seed()).unwrap();
        (d, st)
    }
    #[test]
    fn export_backup_writes_file_and_updates_last_backup() {
        let (d, st) = state(FakeAuthenticator::ok());
        let path = d.path().join("backup.cryptbackup");
        let result = export(&st, &path, None).unwrap();
        assert!(result.bytes > 0);
        let mut file = std::fs::File::open(path).unwrap();
        assert_eq!(
            crypt_core::backup::read_backup_header(&mut file).unwrap(),
            result.header
        );
        assert!(st.settings.lock().unwrap().last_backup_at.is_some());
    }
    #[test]
    fn export_requires_presence() {
        let (d, st) = state(FakeAuthenticator::err(AuthError::Cancelled));
        let path = d.path().join("backup.cryptbackup");
        assert!(export(&st, &path, None).is_err());
        assert!(!path.exists());
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;
    use crate::{
        commands::lifecycle::{VaultStatus, create_vault, generate_seed, status},
        unlock::{authenticator::FakeAuthenticator, keystore::MemoryKeyStore},
    };
    fn state(root: &Path) -> AppState {
        AppState::new(
            root.join("vault"),
            root.join("settings.json"),
            Box::new(MemoryKeyStore::default()),
            Box::new(FakeAuthenticator::ok()),
        )
    }
    fn backup(d: &Path) -> (std::path::PathBuf, Vec<String>, crypt_core::RecordId) {
        let source = state(&d.join("source"));
        let words = generate_seed();
        create_vault(&source, words.clone()).unwrap();
        let id = source
            .vault_guard()
            .as_mut()
            .unwrap()
            .create_record(crypt_core::NewRecord {
                title: "Restored".into(),
                ..Default::default()
            })
            .unwrap();
        let path = d.join("copy.cryptbackup");
        export(&source, &path, None).unwrap();
        (path, words, id)
    }
    #[test]
    fn restore_when_vault_exists_fails() {
        let d = tempfile::tempdir().unwrap();
        let (path, words, _) = backup(d.path());
        let st = state(&d.path().join("target"));
        create_vault(&st, generate_seed()).unwrap();
        assert_eq!(
            restore_file(&st, &path, words).unwrap_err().code,
            "VaultExists"
        );
    }
    #[test]
    fn restore_opens_vault() {
        let d = tempfile::tempdir().unwrap();
        let (path, words, id) = backup(d.path());
        let st = state(&d.path().join("target"));
        assert!(matches!(
            restore_file(&st, &path, words).unwrap(),
            VaultStatus::Unlocked { .. }
        ));
        assert_eq!(
            st.vault_guard().as_ref().unwrap().get(&id).unwrap().title,
            "Restored"
        );
    }
    #[test]
    fn restore_wrong_seed_keeps_novault() {
        let d = tempfile::tempdir().unwrap();
        let (path, _, _) = backup(d.path());
        let st = state(&d.path().join("target"));
        assert_eq!(
            restore_file(&st, &path, generate_seed()).unwrap_err().code,
            "BackupSeedMismatch"
        );
        assert!(matches!(status(&st), VaultStatus::NoVault));
        assert!(!st.vault_dir.exists());
    }
}

#[cfg(test)]
mod inspection_tests {
    use super::*;
    #[test]
    fn inspection_reports_actual_file_size_without_unpacking() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("vault");
        let seed = crypt_core::SeedPhrase::generate();
        crypt_core::Vault::create(&root, &seed).unwrap();
        let path = d.path().join("test.cryptbackup");
        let mut file = std::fs::File::create(&path).unwrap();
        let expected = write_backup(&root, &mut file, 123).unwrap();
        drop(file);
        let info = inspect_file(&path).unwrap();
        assert_eq!(info.bytes, std::fs::metadata(&path).unwrap().len());
        assert_eq!(info.header, expected);
    }
}
