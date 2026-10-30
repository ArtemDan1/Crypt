//! Import (KeePass, Obsidian) and Markdown export. Each import first takes a vault
//! snapshot so it can be rolled back from Settings.
use crate::commands::library::with_vault;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crypt_core::{ExportReport, KeepassReport, MarkdownImportReport, Vault};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use zeroize::Zeroizing;

const MAX_IMPORT_FILE: u64 = 200 * 1024 * 1024;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Imported<T> {
    pub report: T,
    pub snapshot: String,
}

fn read_limited(path: &str) -> ApiResult<Zeroizing<Vec<u8>>> {
    let meta = std::fs::metadata(path).map_err(|_| ApiError::new("ImportFormat", "Файл не найден"))?;
    if !meta.is_file() || meta.len() > MAX_IMPORT_FILE {
        return Err(ApiError::new("ImportFormat", "Файл не подходит для импорта"));
    }
    Ok(Zeroizing::new(
        std::fs::read(path).map_err(|_| ApiError::new("ImportFormat", "Не удалось прочитать файл"))?,
    ))
}
fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}
fn with_snapshot<T>(
    v: &mut Vault,
    label: String,
    f: impl FnOnce(&mut Vault) -> crypt_core::Result<T>,
) -> crypt_core::Result<Imported<T>> {
    let snapshot = v.create_snapshot(&label)?;
    match f(v) {
        Ok(report) => Ok(Imported { report, snapshot: snapshot.name }),
        Err(e) => {
            // Nothing (or only part) was imported: drop the snapshot only when nothing was.
            let total = v.list(&Default::default()).len()
                + v.list(&crypt_core::ListFilter { trashed: true, ..Default::default() }).len();
            if total == snapshot.records {
                let _ = v.delete_snapshot(&snapshot.id);
            }
            Err(e)
        }
    }
}

pub fn import_keepass_file(
    st: &AppState,
    path: &str,
    password: Option<&str>,
    keyfile: Option<&str>,
) -> ApiResult<Imported<KeepassReport>> {
    let data = read_limited(path)?;
    let key = keyfile.map(read_limited).transpose()?;
    with_vault(st, |v| {
        with_snapshot(v, format!("Перед импортом {}", file_name(path)), |v| {
            v.import_keepass(&data, password, key.as_deref().map(|k| k.as_slice()))
        })
    })
}

#[tauri::command]
pub async fn import_keepass(
    st: State<'_, Arc<AppState>>,
    path: String,
    password: Option<String>,
    keyfile: Option<String>,
) -> ApiResult<Imported<KeepassReport>> {
    let password = password.map(Zeroizing::new);
    import_keepass_file(&st, &path, password.as_deref().map(String::as_str), keyfile.as_deref())
}

#[tauri::command]
pub async fn import_markdown(
    st: State<'_, Arc<AppState>>,
    path: String,
) -> ApiResult<Imported<MarkdownImportReport>> {
    with_vault(&st, |v| {
        with_snapshot(v, format!("Перед импортом {}", file_name(&path)), |v| {
            v.import_markdown_dir(Path::new(&path))
        })
    })
}

#[tauri::command]
pub async fn export_markdown(
    st: State<'_, Arc<AppState>>,
    path: String,
    include_secrets: bool,
) -> ApiResult<ExportReport> {
    with_vault(&st, |v| v.export_markdown(Path::new(&path), include_secrets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::lifecycle::{create_vault, generate_seed};

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(
            dir.path().join("vault"),
            dir.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        create_vault(&st, generate_seed()).unwrap();
        (dir, st)
    }

    #[test]
    fn failed_import_leaves_no_snapshot_and_success_keeps_one() {
        let (dir, st) = state();
        let junk = dir.path().join("junk.kdbx");
        std::fs::write(&junk, b"not a database").unwrap();
        let err = import_keepass_file(&st, junk.to_str().unwrap(), Some("pw"), None).unwrap_err();
        assert_eq!(err.code, "ImportFormat");
        assert!(with_vault(&st, |v| Ok(v.snapshots())).unwrap().is_empty());

        let notes = dir.path().join("Заметки");
        std::fs::create_dir(&notes).unwrap();
        std::fs::write(notes.join("a.md"), "hello").unwrap();
        let out = with_vault(&st, |v| {
            with_snapshot(v, "Перед импортом Заметки".into(), |v| v.import_markdown_dir(&notes))
        })
        .unwrap();
        assert_eq!(out.report.notes, 1);
        assert_eq!(out.snapshot, "Перед импортом Заметки");
        assert_eq!(with_vault(&st, |v| Ok(v.snapshots())).unwrap().len(), 1);
    }

    #[test]
    fn missing_file_is_a_clear_error() {
        let (_d, st) = state();
        let err = import_keepass_file(&st, "/nonexistent/x.kdbx", Some("pw"), None).unwrap_err();
        assert_eq!(err.code, "ImportFormat");
    }
}
