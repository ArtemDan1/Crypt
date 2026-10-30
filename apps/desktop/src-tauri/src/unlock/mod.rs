pub mod authenticator;
pub mod keystore;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod pin;

use crate::error::{ApiError, ApiResult};
use authenticator::Authenticator;
use crypt_core::{SecretKey, Vault};
use keystore::KeyStore;
use serde::Serialize;
use std::path::Path;

pub const MAX_PIN_ATTEMPTS: u8 = 5;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnlockMethods {
    pub biometric: bool,
    pub biometric_available: bool,
    pub pin: bool,
    /// Unknown after failures until a deliberate PIN attempt; status never reads secrets.
    pub attempts_left: Option<u8>,
}

fn key_account(id: &str) -> String {
    format!("data-key:{id}")
}
fn pin_account(id: &str) -> String {
    format!("pin:{id}")
}
fn attempts_account(id: &str) -> String {
    format!("pin-attempts:{id}")
}

pub struct Unlocker<'a> {
    keys: &'a dyn KeyStore,
    auth: &'a dyn Authenticator,
}

impl<'a> Unlocker<'a> {
    pub fn new(keys: &'a dyn KeyStore, auth: &'a dyn Authenticator) -> Self {
        Self { keys, auth }
    }

    fn failures(&self, id: &str) -> ApiResult<u8> {
        Ok(self.keys.get(&attempts_account(id))?.and_then(|v| v.first().copied()).unwrap_or(0))
    }
    pub fn reset_attempts(&self, id: &str) -> ApiResult<()> {
        self.keys.delete(&attempts_account(id))
    }

    pub fn methods(&self, id: &str) -> ApiResult<UnlockMethods> {
        Ok(UnlockMethods {
            biometric: self.keys.contains(&key_account(id))?,
            biometric_available: self.auth.available(),
            pin: self.keys.contains(&pin_account(id))?,
            attempts_left: if self.keys.contains(&attempts_account(id))? { None } else { Some(MAX_PIN_ATTEMPTS) },
        })
    }

    pub fn enable_biometric(&self, vault: &Vault) -> ApiResult<()> {
        self.auth.authenticate("включить вход по Touch ID")?;
        self.keys.set(&key_account(&vault.vault_id().to_string()), vault.data_key().as_bytes())
    }
    pub fn disable_biometric(&self, id: &str) -> ApiResult<()> {
        self.keys.delete(&key_account(id))
    }

    pub fn unlock_biometric(&self, dir: &Path, id: &str) -> ApiResult<Vault> {
        let Some(raw) = self.keys.get(&key_account(id))? else {
            return Err(ApiError::new("BiometricNotSet", "Вход по Touch ID не настроен"));
        };
        self.auth.authenticate("открыть хранилище")?;
        let bytes: [u8; 32] = raw.as_slice().try_into()
            .map_err(|_| ApiError::new("Keychain", "Ключ в связке повреждён"))?;
        let vault = Vault::open_with_key(dir, SecretKey::from_bytes(bytes))?;
        self.reset_attempts(id)?;
        Ok(vault)
    }

    pub fn set_pin(&self, vault: &Vault, pin: &str) -> ApiResult<()> {
        let id = vault.vault_id().to_string();
        let w = pin::wrap(pin, vault.data_key(), &id)?;
        let json = zeroize::Zeroizing::new(serde_json::to_vec(&w).expect("serializable"));
        self.keys.set(&pin_account(&id), &json)?;
        self.reset_attempts(&id)
    }
    pub fn remove_pin(&self, id: &str) -> ApiResult<()> {
        self.keys.delete(&pin_account(id))?;
        self.reset_attempts(id)
    }

    fn check_pin(&self, id: &str, pin: &str) -> ApiResult<SecretKey> {
        let Some(raw) = self.keys.get(&pin_account(id))? else {
            return Err(ApiError::new("PinLocked", "Вход по PIN недоступен — используйте Touch ID или сид-фразу"));
        };
        let w: pin::PinWrap = serde_json::from_slice(&raw)
            .map_err(|_| ApiError::new("Keychain", "Данные PIN повреждены"))?;
        match pin::unwrap(pin, &w, id) {
            Ok(k) => {
                self.reset_attempts(id)?;
                Ok(k)
            }
            Err(e) if e.code == "WrongPin" => {
                let failures = self.failures(id)?.saturating_add(1);
                if failures >= MAX_PIN_ATTEMPTS {
                    self.remove_pin(id)?;
                    return Err(ApiError::new("PinLocked", "PIN введён неверно 5 раз и отключён — используйте Touch ID или сид-фразу"));
                }
                self.keys.set(&attempts_account(id), &[failures])?;
                let left = MAX_PIN_ATTEMPTS - failures;
                Err(ApiError::new("WrongPin", format!("Неверный PIN. Осталось попыток: {left}")))
            }
            Err(e) => Err(e),
        }
    }

    pub fn unlock_pin(&self, dir: &Path, id: &str, pin: &str) -> ApiResult<Vault> {
        let key = self.check_pin(id, pin)?;
        Ok(Vault::open_with_key(dir, key)?)
    }

    pub fn confirm_deletion(&self, id: &str, pin: Option<&str>) -> ApiResult<()> {
        match pin {
            Some(p) => self.check_pin(id, p).map(|_| ()),
            None => Ok(self.auth.authenticate("удалить хранилище без возможности отмены")?),
        }
    }

    /// Fresh proof of presence, e.g. before showing the seed phrase.
    pub fn confirm_presence(&self, id: &str, pin: Option<&str>) -> ApiResult<()> {
        match pin {
            Some(p) => self.check_pin(id, p).map(|_| ()),
            None => Ok(self.auth.authenticate("показать сид-фразу")?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use authenticator::{AuthError, FakeAuthenticator};
    use crypt_core::{SeedPhrase, Vault};
    use keystore::MemoryKeyStore;

    struct Fx {
        dir: tempfile::TempDir,
        seed: SeedPhrase,
        id: String,
    }
    fn fx() -> Fx {
        let dir = tempfile::tempdir().unwrap();
        let seed = SeedPhrase::generate();
        let v = Vault::create(&dir.path().join("vault"), &seed).unwrap();
        let id = v.vault_id().to_string();
        Fx { dir, seed, id }
    }
    impl Fx {
        fn vdir(&self) -> std::path::PathBuf {
            self.dir.path().join("vault")
        }
        fn open(&self) -> Vault {
            Vault::open_with_seed(&self.vdir(), &self.seed).unwrap()
        }
    }

    #[test]
    fn method_discovery_never_reads_secrets() {
        struct MetadataOnly;
        impl KeyStore for MetadataOnly {
            fn get(&self, _: &str) -> ApiResult<Option<zeroize::Zeroizing<Vec<u8>>>> {
                panic!("passive discovery must not read keychain secret data");
            }
            fn contains(&self, _: &str) -> ApiResult<bool> { Ok(true) }
            fn set(&self, _: &str, _: &[u8]) -> ApiResult<()> { unreachable!() }
            fn delete(&self, _: &str) -> ApiResult<()> { unreachable!() }
        }
        let auth = FakeAuthenticator::ok();
        let methods = Unlocker::new(&MetadataOnly, &auth).methods("vault").unwrap();
        assert!(methods.biometric && methods.pin);
    }

    #[test]
    fn five_wrong_pins_lock_out_even_correct_one() {
        let f = fx();
        let (ks, auth) = (MemoryKeyStore::default(), FakeAuthenticator::ok());
        let u = Unlocker::new(&ks, &auth);
        u.set_pin(&f.open(), "482913").unwrap();
        for i in 1..=4 {
            let e = u.unlock_pin(&f.vdir(), &f.id, "000000").unwrap_err();
            assert_eq!(e.code, "WrongPin");
            assert_eq!(u.failures(&f.id).unwrap(), i);
            assert_eq!(u.methods(&f.id).unwrap().attempts_left, None);
        }
        assert_eq!(u.unlock_pin(&f.vdir(), &f.id, "000000").unwrap_err().code, "PinLocked");
        assert_eq!(u.unlock_pin(&f.vdir(), &f.id, "482913").unwrap_err().code, "PinLocked");
        assert!(!u.methods(&f.id).unwrap().pin);
    }

    #[test]
    fn successful_unlock_resets_attempts() {
        let f = fx();
        let (ks, auth) = (MemoryKeyStore::default(), FakeAuthenticator::ok());
        let u = Unlocker::new(&ks, &auth);
        u.set_pin(&f.open(), "482913").unwrap();
        for _ in 0..3 {
            u.unlock_pin(&f.vdir(), &f.id, "111111").unwrap_err();
        }
        let v = u.unlock_pin(&f.vdir(), &f.id, "482913").unwrap();
        assert_eq!(v.vault_id().to_string(), f.id);
        assert_eq!(u.methods(&f.id).unwrap().attempts_left, Some(5));
    }

    #[test]
    fn biometric_cancel_and_unavailable() {
        let f = fx();
        let ks = MemoryKeyStore::default();
        Unlocker::new(&ks, &FakeAuthenticator::ok()).enable_biometric(&f.open()).unwrap();
        let cancel = FakeAuthenticator::err(AuthError::Cancelled);
        let e = Unlocker::new(&ks, &cancel).unlock_biometric(&f.vdir(), &f.id).unwrap_err();
        assert_eq!(e.code, "AuthCancelled");
        let none = FakeAuthenticator::err(AuthError::Unavailable);
        let e = Unlocker::new(&ks, &none).unlock_biometric(&f.vdir(), &f.id).unwrap_err();
        assert_eq!(e.code, "AuthUnavailable");
        let v = Unlocker::new(&ks, &FakeAuthenticator::ok()).unlock_biometric(&f.vdir(), &f.id).unwrap();
        assert_eq!(v.vault_id().to_string(), f.id);
    }

    #[test]
    fn biometric_without_enrolled_key() {
        let f = fx();
        let (ks, auth) = (MemoryKeyStore::default(), FakeAuthenticator::ok());
        let e = Unlocker::new(&ks, &auth).unlock_biometric(&f.vdir(), &f.id).unwrap_err();
        assert_eq!(e.code, "BiometricNotSet");
    }

    #[test]
    fn enable_biometric_requires_auth_first() {
        let f = fx();
        let ks = MemoryKeyStore::default();
        let failed = FakeAuthenticator::err(AuthError::Failed);
        assert!(Unlocker::new(&ks, &failed).enable_biometric(&f.open()).is_err());
        assert!(!Unlocker::new(&ks, &failed).methods(&f.id).unwrap().biometric);
    }

    #[test]
    fn biometric_unlock_resets_pin_attempts() {
        let f = fx();
        let (ks, auth) = (MemoryKeyStore::default(), FakeAuthenticator::ok());
        let u = Unlocker::new(&ks, &auth);
        u.set_pin(&f.open(), "482913").unwrap();
        u.enable_biometric(&f.open()).unwrap();
        u.unlock_pin(&f.vdir(), &f.id, "111111").unwrap_err();
        u.unlock_biometric(&f.vdir(), &f.id).unwrap();
        assert_eq!(u.methods(&f.id).unwrap().attempts_left, Some(5));
    }

    #[test]
    fn confirm_presence_for_seed_reveal() {
        let f = fx();
        let ks = MemoryKeyStore::default();
        Unlocker::new(&ks, &FakeAuthenticator::ok()).set_pin(&f.open(), "482913").unwrap();
        let cancel = FakeAuthenticator::err(AuthError::Cancelled);
        let u = Unlocker::new(&ks, &cancel);
        assert!(u.confirm_presence(&f.id, None).is_err());
        assert!(u.confirm_presence(&f.id, Some("000000")).is_err());
        assert!(u.confirm_presence(&f.id, Some("482913")).is_ok());
        assert!(Unlocker::new(&ks, &FakeAuthenticator::ok()).confirm_presence(&f.id, None).is_ok());
    }

    #[test]
    fn disable_and_remove() {
        let f = fx();
        let (ks, auth) = (MemoryKeyStore::default(), FakeAuthenticator::ok());
        let u = Unlocker::new(&ks, &auth);
        u.enable_biometric(&f.open()).unwrap();
        u.set_pin(&f.open(), "482913").unwrap();
        u.disable_biometric(&f.id).unwrap();
        u.remove_pin(&f.id).unwrap();
        let m = u.methods(&f.id).unwrap();
        assert!(!m.biometric && !m.pin);
    }
}
