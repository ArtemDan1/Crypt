use crate::{
    CryptError, Result, SeedPhrase, Vault,
    crypto::{envelope::open, keys::VaultKeys},
    storage::layout::Aad,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const MAGIC: &[u8; 8] = b"CRYPTBAK";
pub const MAX_BACKUP_BYTES: u64 = 2 * 1024 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupHeader {
    pub version: u16,
    pub created_at: i64,
    pub objects: u32,
}
fn damaged() -> CryptError {
    CryptError::BackupFormat("Файл повреждён".into())
}

pub fn read_backup_header(input: &mut impl Read) -> Result<BackupHeader> {
    let mut magic = [0; 8];
    if input.read_exact(&mut magic).is_err() || &magic != MAGIC {
        return Err(CryptError::BackupFormat("Это не файл бэкапа Crypt".into()));
    }
    let mut version = [0; 2];
    input.read_exact(&mut version).map_err(|_| damaged())?;
    let version = u16::from_le_bytes(version);
    if version > 1 {
        return Err(CryptError::BackupFormat(
            "Файл от более новой версии Crypt".into(),
        ));
    }
    if version != 1 {
        return Err(damaged());
    }
    let mut len = [0; 4];
    input.read_exact(&mut len).map_err(|_| damaged())?;
    let len = u32::from_le_bytes(len) as usize;
    if len > 4096 {
        return Err(damaged());
    }
    let mut json = vec![0; len];
    input.read_exact(&mut json).map_err(|_| damaged())?;
    let header: BackupHeader = serde_json::from_slice(&json).map_err(|_| damaged())?;
    if header.version != version {
        return Err(damaged());
    }
    Ok(header)
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && part != "device.json")
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
fn files(root: &Path, dir: &Path, result: &mut Vec<(String, PathBuf, u64)>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if entry.file_name() == "device.json" {
            continue;
        }
        if kind.is_symlink() {
            return Err(damaged());
        }
        if kind.is_dir() {
            files(root, &entry.path(), result)?;
        } else if kind.is_file() {
            let path = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| damaged())?
                .to_str()
                .ok_or_else(damaged)?
                .to_owned();
            if !safe_path(&path) || path.len() > u16::MAX as usize {
                return Err(damaged());
            }
            result.push((path, entry.path(), entry.metadata()?.len()));
        } else {
            return Err(damaged());
        }
    }
    Ok(())
}
pub fn write_backup(root: &Path, out: &mut impl Write, now: i64) -> Result<BackupHeader> {
    let mut entries = vec![];
    files(root, root, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let total = entries
        .iter()
        .try_fold(4096u64, |n, (p, _, len)| {
            n.checked_add(*len)?.checked_add(p.len() as u64 + 10)
        })
        .ok_or_else(damaged)?;
    if total > MAX_BACKUP_BYTES {
        return Err(damaged());
    }
    let header = BackupHeader {
        version: 1,
        created_at: now,
        objects: u32::try_from(entries.len()).map_err(|_| damaged())?,
    };
    let mut hash = Sha256::new();
    let mut put = |bytes: &[u8]| -> Result<()> {
        out.write_all(bytes)?;
        hash.update(bytes);
        Ok(())
    };
    let json = serde_json::to_vec(&header)?;
    put(MAGIC)?;
    put(&1u16.to_le_bytes())?;
    put(&(json.len() as u32).to_le_bytes())?;
    put(&json)?;
    for (path, file, len) in entries {
        put(&(path.len() as u16).to_le_bytes())?;
        put(path.as_bytes())?;
        put(&len.to_le_bytes())?;
        let mut input = fs::File::open(file)?;
        let mut left = len;
        let mut buffer = [0; 65536];
        while left > 0 {
            let take = left.min(buffer.len() as u64) as usize;
            input.read_exact(&mut buffer[..take])?;
            put(&buffer[..take])?;
            left -= take as u64;
        }
    }
    out.write_all(&hash.finalize())?;
    Ok(header)
}
fn parse(bytes: &[u8]) -> Result<(BackupHeader, BTreeMap<String, &[u8]>)> {
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err(damaged());
    }
    let mut cursor = std::io::Cursor::new(bytes);
    let header = read_backup_header(&mut cursor)?;
    let end = bytes.len().checked_sub(32).ok_or_else(damaged)?;
    if cursor.position() as usize > end || Sha256::digest(&bytes[..end]).as_slice() != &bytes[end..]
    {
        return Err(damaged());
    }
    let mut entries = BTreeMap::new();
    while (cursor.position() as usize) < end {
        let mut len = [0; 2];
        cursor.read_exact(&mut len).map_err(|_| damaged())?;
        let mut path = vec![0; u16::from_le_bytes(len) as usize];
        cursor.read_exact(&mut path).map_err(|_| damaged())?;
        let path = String::from_utf8(path).map_err(|_| damaged())?;
        if !safe_path(&path) {
            return Err(damaged());
        }
        let mut len = [0; 8];
        cursor.read_exact(&mut len).map_err(|_| damaged())?;
        let len = usize::try_from(u64::from_le_bytes(len)).map_err(|_| damaged())?;
        let start = cursor.position() as usize;
        let next = start.checked_add(len).ok_or_else(damaged)?;
        if next > end || entries.insert(path, &bytes[start..next]).is_some() {
            return Err(damaged());
        }
        cursor.set_position(next as u64);
    }
    if entries.len() != header.objects as usize
        || ["vault.json", "keycheck.bin", "seed.bin", "meta.bin"]
            .iter()
            .any(|name| !entries.contains_key(*name))
    {
        return Err(damaged());
    }
    Ok((header, entries))
}
pub fn verify_backup_seed(bytes: &[u8], seed: &SeedPhrase) -> Result<()> {
    let (_, entries) = parse(bytes)?;
    let keys = VaultKeys::derive(seed);
    let plain = open(
        &keys.data_key,
        &Aad::KeyCheck.bytes(),
        entries["keycheck.bin"],
    )
    .map_err(|_| CryptError::BackupSeedMismatch)?;
    if plain.as_slice() != b"crypt-keycheck-v1" {
        return Err(CryptError::BackupSeedMismatch);
    }
    Ok(())
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn restore_backup(bytes: &[u8], seed: &SeedPhrase, root: &Path) -> Result<BackupHeader> {
    if root.exists() {
        return Err(CryptError::VaultExists);
    }
    verify_backup_seed(bytes, seed)?;
    let (header, entries) = parse(bytes)?;
    let parent = root.parent().ok_or_else(damaged)?;
    fs::create_dir_all(parent)?;
    let temp = Temporary(parent.join(format!(".crypt-restore-{}", ulid::Ulid::new())));
    fs::create_dir(&temp.0)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp.0, fs::Permissions::from_mode(0o700))?;
    }
    fs::create_dir(temp.0.join("objects"))?;
    for (path, data) in entries {
        let target = temp.0.join(path);
        fs::create_dir_all(target.parent().ok_or_else(damaged)?)?;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        opts.open(target)?.write_all(data)?;
    }
    let vault = Vault::open_with_seed(&temp.0, seed).map_err(|_| damaged())?;
    if !vault.load_errors().is_empty() {
        return Err(damaged());
    }
    drop(vault);
    fs::remove_file(temp.0.join("device.json"))?;
    if root.exists() {
        return Err(CryptError::VaultExists);
    }
    fs::rename(&temp.0, root)?;
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewRecord, SecretKind, Vault};
    fn source() -> (tempfile::TempDir, SeedPhrase, Vec<u8>, crate::RecordId) {
        let dir = tempfile::tempdir().unwrap();
        let seed = SeedPhrase::generate();
        let mut v = Vault::create(&dir.path().join("source"), &seed).unwrap();
        let id = v
            .create_record(NewRecord {
                title: "Mail".into(),
                secret_kind: Some(SecretKind::Login),
                ..Default::default()
            })
            .unwrap();
        v.set_secret_value(&id, "password", Some("synthetic-secret"))
            .unwrap();
        let mut bytes = vec![];
        write_backup(&dir.path().join("source"), &mut bytes, 123).unwrap();
        (dir, seed, bytes, id)
    }
    #[test]
    fn roundtrip() {
        let (d, seed, bytes, id) = source();
        let root = d.path().join("restored");
        let h = restore_backup(&bytes, &seed, &root).unwrap();
        assert_eq!(h.created_at, 123);
        let v = Vault::open_with_seed(&root, &seed).unwrap();
        assert_eq!(v.get(&id).unwrap().title, "Mail");
        assert_eq!(
            v.secret_value(&id, "password").unwrap().unwrap().as_str(),
            "synthetic-secret"
        );
    }
    #[test]
    fn device_json_not_included() {
        let (d, seed, bytes, _) = source();
        let root = d.path().join("restored");
        restore_backup(&bytes, &seed, &root).unwrap();
        assert!(!root.join("device.json").exists());
        let _v = Vault::open_with_seed(&root, &seed).unwrap();
        assert_ne!(
            std::fs::read(root.join("device.json")).unwrap(),
            std::fs::read(d.path().join("source/device.json")).unwrap()
        );
    }
    #[test]
    fn wrong_seed() {
        let (d, _, bytes, _) = source();
        let root = d.path().join("restored");
        assert!(matches!(
            restore_backup(&bytes, &SeedPhrase::generate(), &root),
            Err(CryptError::BackupSeedMismatch)
        ));
        assert!(!root.exists());
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    }
    #[test]
    fn not_a_backup() {
        assert!(
            matches!(read_backup_header(&mut &b"random"[..]), Err(CryptError::BackupFormat(m)) if m == "Это не файл бэкапа Crypt")
        );
    }
    #[test]
    fn future_version() {
        let mut bytes = b"CRYPTBAK".to_vec();
        bytes.extend(99u16.to_le_bytes());
        assert!(
            matches!(read_backup_header(&mut bytes.as_slice()), Err(CryptError::BackupFormat(m)) if m == "Файл от более новой версии Crypt")
        );
    }
    #[test]
    fn truncated() {
        let (d, seed, bytes, _) = source();
        let root = d.path().join("restored");
        assert!(
            matches!(restore_backup(&bytes[..bytes.len()/2], &seed, &root), Err(CryptError::BackupFormat(m)) if m == "Файл повреждён")
        );
        assert!(!root.exists());
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    }
    #[test]
    fn path_traversal() {
        let (d, seed, mut bytes, _) = source();
        let at = bytes.windows(10).position(|w| w == b"vault.json").unwrap();
        bytes[at..at + 10].copy_from_slice(b"../badfile");
        let end = bytes.len() - 32;
        let hash = Sha256::digest(&bytes[..end]);
        bytes[end..].copy_from_slice(&hash);
        assert!(
            matches!(restore_backup(&bytes, &seed, &d.path().join("restored")), Err(CryptError::BackupFormat(m)) if m == "Файл повреждён")
        );
        assert!(!d.path().join("badfile").exists());
    }
}
