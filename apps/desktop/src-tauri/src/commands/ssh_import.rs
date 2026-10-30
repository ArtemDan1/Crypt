#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::lifecycle::{create_vault, generate_seed};
    #[test]
    fn scan_skips_junk_and_import_is_per_item() {
        let d = tempfile::tempdir().unwrap();
        let ssh = d.path().join("ssh");
        std::fs::create_dir(&ssh).unwrap();
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../crates/crypt-core/tests/fixtures/ssh");
        for (from, to) in [
            ("ed25519", "id_ed25519"),
            ("ed25519.pub", "id_ed25519.pub"),
            ("ed25519_enc", "id_enc"),
            ("ed25519_enc.pub", "id_enc.pub"),
            ("rsa3072", "id_rsa"),
        ] {
            std::fs::copy(src.join(from), ssh.join(to)).unwrap();
        }
        for name in [
            "config",
            "known_hosts",
            "authorized_keys",
            "putty.ppk",
            "broken",
        ] {
            std::fs::write(ssh.join(name), "junk").unwrap();
        }
        std::fs::write(ssh.join("big"), vec![0; 70 * 1024]).unwrap();
        std::fs::create_dir(ssh.join("sub")).unwrap();
        std::fs::copy(src.join("ed25519"), ssh.join("sub/key")).unwrap();
        std::os::unix::fs::symlink("missing", ssh.join("dangling")).unwrap();
        let before: Vec<_> = std::fs::read_dir(&ssh)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                std::fs::read(e.path())
                    .ok()
                    .map(|b| (e.path(), b, e.metadata().unwrap().modified().unwrap()))
            })
            .collect();
        let mut st = AppState::new(
            d.path().join("vault"),
            d.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        st.ssh_dir = ssh.clone();
        create_vault(&st, generate_seed()).unwrap();
        let found = scan(&st).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|c| c.file_name.as_str())
                .collect::<Vec<_>>(),
            ["id_ed25519", "id_enc", "id_rsa"]
        );
        assert!(found[1].encrypted);
        assert!(found[1].info.is_some());
        let results = import(
            &st,
            found
                .iter()
                .map(|c| SshImportItem {
                    path: c.path.clone(),
                    title: c.file_name.clone(),
                    passphrase: Some("wrong".into()),
                })
                .collect(),
        )
        .unwrap();
        assert!(results[0].record_id.is_some());
        assert_eq!(results[1].error.as_ref().unwrap().code, "WrongPassphrase");
        assert!(results[2].record_id.is_some());
        let found = scan(&st).unwrap();
        assert!(found[0].already_imported);
        assert!(!found[1].already_imported);
        assert!(found[2].already_imported);
        for path in [ssh.join("../other"), ssh.join("sub/key")] {
            let r = import(
                &st,
                vec![SshImportItem {
                    path: path.display().to_string(),
                    title: "x".into(),
                    passphrase: None,
                }],
            )
            .unwrap();
            assert_eq!(r[0].error.as_ref().unwrap().code, "Validation");
        }
        for (path, bytes, mtime) in before {
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert_eq!(std::fs::metadata(path).unwrap().modified().unwrap(), mtime);
        }
        assert_eq!(
            st.vault_guard().as_ref().unwrap().agent_identities().len(),
            2
        );
    }
}

use crate::{
    commands::library::with_vault,
    error::{ApiError, ApiResult},
    state::AppState,
};
use crypt_core::{
    ListFilter, NewRecord, RecordId, SecretKind,
    ssh::{SshKeyInfo, inspect_private_key, is_encrypted_private_key, public_key_info},
};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::OpenOptionsExt;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshCandidate {
    pub path: String,
    pub file_name: String,
    pub encrypted: bool,
    pub info: Option<SshKeyInfo>,
    pub already_imported: bool,
}
#[derive(Deserialize)]
pub struct SshImportItem {
    pub path: String,
    pub title: String,
    pub passphrase: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshImportResult {
    pub path: String,
    pub record_id: Option<RecordId>,
    pub error: Option<ApiError>,
}
fn read_key(path: &Path) -> ApiResult<Zeroizing<String>> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| ApiError::new("Validation", "Не удалось прочитать ключ"))?;
    if !file
        .metadata()
        .map_err(|_| ApiError::new("Validation", "Не удалось прочитать ключ"))?
        .is_file()
    {
        return Err(ApiError::new("Validation", "Ожидался файл ключа"));
    }
    let mut text = Zeroizing::new(String::new());
    file.take(65537)
        .read_to_string(&mut text)
        .map_err(|_| ApiError::new("Validation", "Не удалось прочитать ключ"))?;
    if text.len() > 65536 {
        return Err(ApiError::new("Validation", "Файл ключа слишком большой"));
    }
    Ok(text)
}
fn fingerprints(v: &crypt_core::Vault) -> Vec<String> {
    v.list(&ListFilter {
        secrets: Some(true),
        ..Default::default()
    })
    .iter()
    .filter(|r| r.secret_kind == Some(SecretKind::SshKey))
    .filter_map(|r| v.ssh_key_info(&r.id).ok().flatten().map(|i| i.fingerprint))
    .collect()
}
fn direct(root: &Path, path: &Path) -> ApiResult<PathBuf> {
    let path = path
        .canonicalize()
        .map_err(|_| ApiError::new("Validation", "Путь должен указывать на ключ в ~/.ssh"))?;
    if path.parent() != Some(root) {
        return Err(ApiError::new(
            "Validation",
            "Путь должен указывать на ключ в ~/.ssh",
        ));
    }
    Ok(path)
}
pub fn scan(st: &AppState) -> ApiResult<Vec<SshCandidate>> {
    with_vault(st, |_| Ok(()))?;
    if !st.ssh_dir.exists() {
        return Ok(Vec::new());
    }
    let root = st
        .ssh_dir
        .canonicalize()
        .map_err(|_| ApiError::new("Io", "Не удалось прочитать ~/.ssh"))?;
    let known = with_vault(st, |v| Ok(fingerprints(v)))?;
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&root)
        .map_err(|_| ApiError::new("Io", "Не удалось прочитать ~/.ssh"))?
        .flatten()
    {
        if entry.path().extension().is_some_and(|x| x == "pub") {
            continue;
        }
        let Ok(path) = direct(&root, &entry.path()) else {
            continue;
        };
        let Ok(text) = read_key(&path) else {
            continue;
        };
        let Some(encrypted) = is_encrypted_private_key(&text) else {
            continue;
        };
        let info = if encrypted {
            read_key(&PathBuf::from(format!("{}.pub", path.display())))
                .ok()
                .and_then(|s| public_key_info(&s).ok())
        } else {
            inspect_private_key(&text, None).ok().map(|(i, _)| i)
        };
        let already_imported = info
            .as_ref()
            .is_some_and(|i| known.contains(&i.fingerprint));
        found.push(SshCandidate {
            path: path.display().to_string(),
            file_name: entry.file_name().to_string_lossy().into(),
            encrypted,
            info,
            already_imported,
        });
    }
    found.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(found)
}
pub fn import(st: &AppState, items: Vec<SshImportItem>) -> ApiResult<Vec<SshImportResult>> {
    with_vault(st, |_| Ok(()))?;
    let root = st.ssh_dir.canonicalize().ok();
    let mut results = Vec::new();
    for item in items {
        let password = item.passphrase.map(Zeroizing::new);
        let result = (|| {
            let root = root
                .as_deref()
                .ok_or_else(|| ApiError::new("Validation", "Каталог ~/.ssh не найден"))?;
            let path = direct(root, Path::new(&item.path))?;
            let pem = read_key(&path)?;
            let (info, plain) = inspect_private_key(&pem, password.as_deref().map(String::as_str))?;
            with_vault(st, |v| {
                if fingerprints(v).contains(&info.fingerprint) {
                    return Err(crypt_core::CryptError::Validation(
                        "ключ уже импортирован".into(),
                    ));
                }
                let id = v.create_record(NewRecord {
                    title: item.title.clone(),
                    secret_kind: Some(SecretKind::SshKey),
                    ..Default::default()
                })?;
                if let Err(e) = v.set_ssh_private_key(&id, &plain, None) {
                    let _ = v.delete(&id);
                    return Err(e);
                }
                Ok(id)
            })
        })();
        results.push(match result {
            Ok(id) => SshImportResult {
                path: item.path,
                record_id: Some(id),
                error: None,
            },
            Err(e) => SshImportResult {
                path: item.path,
                record_id: None,
                error: Some(e),
            },
        });
    }
    crate::agent::cache::refresh(st);
    Ok(results)
}
#[tauri::command]
pub async fn scan_ssh_dir(
    st: tauri::State<'_, std::sync::Arc<AppState>>,
) -> ApiResult<Vec<SshCandidate>> {
    scan(&st)
}
#[tauri::command]
pub async fn import_ssh_keys(
    st: tauri::State<'_, std::sync::Arc<AppState>>,
    items: Vec<SshImportItem>,
) -> ApiResult<Vec<SshImportResult>> {
    import(&st, items)
}

#[cfg(test)]
mod special_file_tests {
    use super::*;
    use std::{
        ffi::CString,
        io::Write,
        os::unix::fs::OpenOptionsExt,
        time::{Duration, Instant},
    };
    #[test]
    fn scan_skips_fifo_without_waiting_for_a_writer() {
        let d = tempfile::tempdir().unwrap();
        let ssh = d.path().join("ssh");
        std::fs::create_dir(&ssh).unwrap();
        let fifo = ssh.join("pipe");
        let path = CString::new(fifo.to_string_lossy().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        let mut st = AppState::new(
            d.path().join("vault"),
            d.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        st.ssh_dir = ssh;
        crate::commands::lifecycle::create_vault(&st, crate::commands::lifecycle::generate_seed())
            .unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&fifo)
            {
                let _ = f.write_all(b"junk");
            }
        });
        let start = Instant::now();
        assert!(scan(&st).unwrap().is_empty());
        let elapsed = start.elapsed();
        release.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(150),
            "scan blocked for {elapsed:?}"
        );
    }
}
