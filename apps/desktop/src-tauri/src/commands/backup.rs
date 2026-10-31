use crate::{state::AppState, error::{ApiError, ApiResult}};
use crypt_core::backup::{BackupHeader, write_backup};
use serde::Serialize;
use std::{path::Path, io::Write};
#[derive(Debug, Serialize)]
pub struct BackupExport { pub path: String, pub bytes: u64, pub header: BackupHeader }
pub fn export(st: &AppState, path: &Path, pin: Option<&str>) -> ApiResult<BackupExport> {
    let id = crate::commands::lifecycle::header_vault_id(st).ok_or_else(|| ApiError::new("NotAVault", "Хранилище не найдено"))?;
    st.unlocker().confirm_presence(&id, pin)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis().min(i64::MAX as u128) as i64;
    let (header, bytes) = {
        let guard = st.vault_guard();
        let _vault = guard.as_ref().ok_or_else(|| ApiError::new("Locked", "Хранилище заблокировано"))?;
        if path.starts_with(&st.vault_dir) { return Err(ApiError::new("Validation", "Сохраните бэкап вне папки хранилища")); }
        let mut name = path.as_os_str().to_os_string(); name.push(".tmp"); let temp = std::path::PathBuf::from(name);
        let mut opts = std::fs::OpenOptions::new(); opts.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; opts.mode(0o600); }
        let mut file = opts.open(&temp).map_err(crypt_core::CryptError::from)?;
        let result = (|| -> crypt_core::Result<_> {
            let h = write_backup(&st.vault_dir,&mut file,now)?; file.flush()?; file.sync_all()?;
            let bytes = file.metadata()?.len(); std::fs::rename(&temp,path)?; Ok((h,bytes))
        })();
        if result.is_err() { let _ = std::fs::remove_file(&temp); }
        result?
    };
    let _update = st.settings_update.lock().unwrap_or_else(|e|e.into_inner());
    let mut settings = st.settings.lock().unwrap_or_else(|e|e.into_inner());
    let mut next = settings.clone(); next.last_backup_at = Some(now);
    next.save(&st.settings_path).map_err(crypt_core::CryptError::from)?; *settings = next;
    Ok(BackupExport { path:path.to_string_lossy().into_owned(), bytes, header })
}
#[tauri::command]
pub async fn export_backup(st: tauri::State<'_, std::sync::Arc<AppState>>, path: String, pin: Option<String>) -> ApiResult<BackupExport> {
    let state = st.inner().clone(); let pin = pin.map(zeroize::Zeroizing::new);
    tauri::async_runtime::spawn_blocking(move || export(&state,Path::new(&path),pin.as_deref().map(String::as_str))).await.map_err(|_|ApiError::new("Internal","Внутренняя ошибка"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::lifecycle::{create_vault, generate_seed}, unlock::{keystore::MemoryKeyStore, authenticator::{FakeAuthenticator, AuthError}}};
    fn state(auth: FakeAuthenticator) -> (tempfile::TempDir, AppState) {
        let d = tempfile::tempdir().unwrap();
        let st = AppState::new(d.path().join("vault"),d.path().join("settings.json"),Box::new(MemoryKeyStore::default()),Box::new(auth));
        create_vault(&st,generate_seed()).unwrap(); (d,st)
    }
    #[test]
    fn export_backup_writes_file_and_updates_last_backup() {
        let (d,st) = state(FakeAuthenticator::ok());
        let path = d.path().join("backup.cryptbackup");
        let result = export(&st,&path,None).unwrap();
        assert!(result.bytes > 0);
        let mut file = std::fs::File::open(path).unwrap();
        assert_eq!(crypt_core::backup::read_backup_header(&mut file).unwrap(),result.header);
        assert!(st.settings.lock().unwrap().last_backup_at.is_some());
    }
    #[test]
    fn export_requires_presence() {
        let (d,st) = state(FakeAuthenticator::err(AuthError::Cancelled));
        let path = d.path().join("backup.cryptbackup");
        assert!(export(&st,&path,None).is_err()); assert!(!path.exists());
    }
}
