#![cfg(feature = "fs")]
use crypt_core::{storage::fs::write_atomic, *};
use std::fs;
#[test]
fn write_atomic_replaces_and_leaves_no_tmp_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    write_atomic(&path, b"a").unwrap();
    write_atomic(&path, b"b").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"b");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn create_open_unlock_and_wrong_key() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let vault = Vault::create(dir.path(), &seed).unwrap();
    let id = vault.vault_id().clone();
    let key = vault.data_key().clone();
    vault.lock();
    let vault = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert_eq!(vault.vault_id(), &id);
    assert_eq!(
        Vault::open_with_key(dir.path(), key)
            .unwrap()
            .reveal_seed()
            .unwrap()
            .as_str(),
        seed.as_str()
    );
    assert!(matches!(
        Vault::open_with_seed(dir.path(), &SeedPhrase::generate()),
        Err(CryptError::WrongKey)
    ));
    assert!(matches!(
        Vault::create(dir.path(), &seed),
        Err(CryptError::VaultExists)
    ));
    fs::write(dir.path().join("objects/garbage.123.tmp"), b"unfinished").unwrap();
    assert!(Vault::open_with_seed(dir.path(), &seed).is_ok());
}
#[test]
fn unsupported_format_and_non_vault() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    assert!(matches!(
        Vault::open_with_seed(dir.path(), &seed),
        Err(CryptError::NotAVault)
    ));
    Vault::create(dir.path(), &seed).unwrap();
    let p = dir.path().join("vault.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    json["format"] = 2.into();
    fs::write(p, serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(matches!(
        Vault::open_with_seed(dir.path(), &seed),
        Err(CryptError::UnsupportedFormat(2))
    ));
}
#[test]
fn nothing_on_disk_contains_seed_phrase() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    Vault::create(dir.path(), &seed).unwrap();
    for entry in fs::read_dir(dir.path()).unwrap().flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let bytes = fs::read(entry.path()).unwrap();
        for word in seed.words().into_iter().filter(|w| w.len() >= 5) {
            assert!(!bytes.windows(word.len()).any(|s| s == word.as_bytes()));
        }
    }
}
