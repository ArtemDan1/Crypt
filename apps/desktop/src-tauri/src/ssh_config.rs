#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn ssh_config_block_is_idempotent_and_reversible() {
        let orig = "Host gh\n  HostName github.com\n  IdentityAgent none\n";
        let a = with_block(orig);
        assert_eq!(with_block(&a), a);
        assert_eq!(a.matches("# >>> Crypt ssh-agent >>>").count(), 1);
        assert!(a.starts_with(orig));
        assert!(has_block(&a));
        assert_eq!(without_block(&a), orig);
        assert_eq!(without_block(&with_block("Host x")), "Host x\n");
        for original in ["", "# keep\n\n", "Host x\r\n  HostName example.org\r\n"] {
            assert_eq!(without_block(&with_block(original)), original);
        }
        let dir = tempfile::tempdir().unwrap();
        let ssh = dir.path().join("ssh");
        std::fs::create_dir(&ssh).unwrap();
        std::fs::write(ssh.join("config"), orig).unwrap();
        std::fs::set_permissions(ssh.join("config"), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        configure(&ssh).unwrap();
        configure(&ssh).unwrap();
        assert_eq!(
            std::fs::read_to_string(ssh.join("config.crypt-backup")).unwrap(),
            orig
        );
        assert_eq!(
            std::fs::metadata(ssh.join("config"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        unconfigure(&ssh).unwrap();
        assert_eq!(std::fs::read_to_string(ssh.join("config")).unwrap(), orig);
        let new = dir.path().join("new");
        configure(&new).unwrap();
        assert_eq!(
            std::fs::metadata(&new).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(new.join("config"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        unconfigure(&new).unwrap();
        assert_eq!(std::fs::read_to_string(new.join("config")).unwrap(), "");
    }
    #[test]
    fn config_keeps_backup_and_existing_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(&path, "Host original\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        configure(dir.path()).unwrap();
        std::fs::write(&path, with_block("Host changed\n")).unwrap();
        configure(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("config.crypt-backup")).unwrap(),
            "Host original\n"
        );
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }
}

use std::{
    fs,
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};
pub const BLOCK: &str = "# >>> Crypt ssh-agent >>>\nHost *\n  IdentityAgent \"~/Library/Application Support/dev.crypt.app/agent.sock\"\n# <<< Crypt ssh-agent <<<\n";
const START: &str = "# >>> Crypt ssh-agent >>>";
const END: &str = "# <<< Crypt ssh-agent <<<";
fn block_range(s: &str) -> Option<std::ops::Range<usize>> {
    let start = s
        .match_indices(START)
        .find(|(i, _)| *i == 0 || s.as_bytes()[i - 1] == b'\n')?
        .0;
    let tail = &s[start + START.len()..];
    if !tail.starts_with('\n') && !tail.starts_with("\r\n") {
        return None;
    }
    let end_start = tail
        .match_indices(END)
        .find(|(i, _)| *i > 0 && tail.as_bytes()[i - 1] == b'\n')?
        .0
        + start
        + START.len();
    let end = end_start + END.len();
    let end = if s[end..].starts_with("\r\n") {
        end + 2
    } else if s[end..].starts_with('\n') {
        end + 1
    } else if end == s.len() {
        end
    } else {
        return None;
    };
    let start = if start > 0 && s[..start].ends_with("\n\n") {
        start - 1
    } else {
        start
    };
    Some(start..end)
}
pub fn has_block(original: &str) -> bool {
    block_range(original).is_some()
}
pub fn without_block(original: &str) -> String {
    let mut out = original.to_owned();
    while let Some(range) = block_range(&out) {
        out.replace_range(range, "");
    }
    out
}
pub fn with_block(original: &str) -> String {
    let mut out = without_block(original);
    if !out.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(BLOCK);
    out
}
fn read_config(dir: &Path) -> io::Result<String> {
    match fs::read_to_string(dir.join("config")) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e),
    }
}
fn write_config_temp(config: &Path, temp: &Path, text: &str) -> io::Result<()> {
    let dir = config
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing config directory"))?;
    let mode = match fs::metadata(config) {
        Ok(meta) => meta.permissions().mode() & 0o777,
        Err(e) if e.kind() == io::ErrorKind::NotFound => 0o600,
        Err(e) => return Err(e),
    };
    // create_new prevents following a pre-existing symlink or overwriting another writer's temp file.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(temp)?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        file.sync_all()?;
        fs::rename(temp, config)?;
        fs::File::open(dir)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn write_config(dir: &Path, text: &str) -> io::Result<()> {
    let config = dir.join("config");
    let config = match fs::symlink_metadata(&config) {
        Ok(meta) if meta.file_type().is_symlink() => fs::canonicalize(&config)?,
        Ok(_) => config,
        Err(e) if e.kind() == io::ErrorKind::NotFound => config,
        Err(e) => return Err(e),
    };
    let parent = config
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing config directory"))?;
    let temp = parent.join(format!(".config.crypt-{:016x}.tmp", rand::random::<u64>()));
    write_config_temp(&config, &temp, text)
}
pub fn configure(dir: &Path) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let original = read_config(dir)?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(dir.join("config.crypt-backup"))
    {
        Ok(mut file) => {
            file.write_all(original.as_bytes())?;
            file.sync_all()?;
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    write_config(dir, &with_block(&original))
}
pub fn unconfigure(dir: &Path) -> io::Result<()> {
    let original = read_config(dir)?;
    if has_block(&original) {
        write_config(dir, &without_block(&original))?;
    }
    Ok(())
}

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};
use serde::Serialize;
use tauri::State;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConfigStatus {
    pub configured: bool,
    pub path: String,
    pub block: String,
    pub export_line: String,
}
fn config_error(_: io::Error) -> ApiError {
    ApiError::new("Io", "Не удалось прочитать или изменить ~/.ssh/config")
}
#[tauri::command]
pub async fn ssh_config_status(st: State<'_, std::sync::Arc<AppState>>) -> ApiResult<SshConfigStatus> {
    let text = read_config(&st.ssh_dir).map_err(config_error)?;
    let socket = st
        .settings_path
        .parent()
        .ok_or_else(|| ApiError::new("Io", "Не найден каталог приложения"))?
        .join("agent.sock");
    let escaped = socket
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    Ok(SshConfigStatus {
        configured: has_block(&text),
        path: st.ssh_dir.join("config").display().to_string(),
        block: BLOCK.into(),
        export_line: format!("export SSH_AUTH_SOCK=\"{escaped}\""),
    })
}
#[tauri::command]
pub async fn configure_ssh_config(st: State<'_, std::sync::Arc<AppState>>) -> ApiResult<()> {
    configure(&st.ssh_dir).map_err(config_error)
}
#[tauri::command]
pub async fn unconfigure_ssh_config(st: State<'_, std::sync::Arc<AppState>>) -> ApiResult<()> {
    unconfigure(&st.ssh_dir).map_err(config_error)
}

#[cfg(test)]
mod atomic_tests {
    use super::*;
    #[test]
    fn colliding_temporary_file_is_never_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let temp = dir.path().join("collision");
        fs::write(&temp, "other writer").unwrap();
        assert!(write_config_temp(&dir.path().join("config"), &temp, "new config").is_err());
        assert_eq!(fs::read_to_string(&temp).unwrap(), "other writer");
        assert!(!dir.path().join("config").exists());
    }
}

#[cfg(test)]
mod symlink_tests {
    use super::*;
    #[test]
    fn config_symlink_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let ssh = root.path().join("ssh");
        fs::create_dir(&ssh).unwrap();
        let target = root.path().join("dotfile");
        fs::write(&target, "Host personal\n").unwrap();
        let config = ssh.join("config");
        std::os::unix::fs::symlink(&target, &config).unwrap();
        configure(&ssh).unwrap();
        assert!(
            fs::symlink_metadata(&config)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(has_block(&fs::read_to_string(&target).unwrap()));
        unconfigure(&ssh).unwrap();
        assert_eq!(fs::read_to_string(target).unwrap(), "Host personal\n");
    }
}
