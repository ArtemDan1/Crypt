use crate::error::{ApiError, ApiResult};
use std::collections::HashMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

pub const SERVICE: &str = "dev.crypt.app";

/// Small secret store keyed by account name.
pub trait KeyStore: Send + Sync {
    /// Inspect public attributes only; never request password data.
    fn contains(&self, account: &str) -> ApiResult<bool>;
    fn get(&self, account: &str) -> ApiResult<Option<Zeroizing<Vec<u8>>>>;
    fn set(&self, account: &str, data: &[u8]) -> ApiResult<()>;
    fn delete(&self, account: &str) -> ApiResult<()>;
}

#[derive(Default)]
pub struct MemoryKeyStore(Mutex<HashMap<String, Vec<u8>>>);
impl KeyStore for MemoryKeyStore {
    fn contains(&self, account: &str) -> ApiResult<bool> {
        Ok(self.0.lock().unwrap().contains_key(account))
    }
    fn get(&self, account: &str) -> ApiResult<Option<Zeroizing<Vec<u8>>>> {
        Ok(self.0.lock().unwrap().get(account).cloned().map(Zeroizing::new))
    }
    fn set(&self, account: &str, data: &[u8]) -> ApiResult<()> {
        self.0.lock().unwrap().insert(account.into(), data.to_vec());
        Ok(())
    }
    fn delete(&self, account: &str) -> ApiResult<()> {
        self.0.lock().unwrap().remove(account);
        Ok(())
    }
}

/// Login keychain (file-based). Works without a paid developer account;
/// access is bound to the app's code signature.
#[cfg(target_os = "macos")]
pub struct MacKeychain;

#[cfg(target_os = "macos")]
impl KeyStore for MacKeychain {
    fn contains(&self, account: &str) -> ApiResult<bool> {
        use security_framework::item::{ItemClass, ItemSearchOptions};
        match ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(SERVICE)
            .account(account)
            .load_attributes(true)
            .load_data(false)
            .search()
        {
            Ok(items) => Ok(!items.is_empty()),
            Err(e) if e.code() == -25300 => Ok(false),
            Err(_) => Err(keychain_error()),
        }
    }

    fn get(&self, account: &str) -> ApiResult<Option<Zeroizing<Vec<u8>>>> {
        use security_framework::passwords::get_generic_password;
        match get_generic_password(SERVICE, account) {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(e) if e.code() == -25300 => Ok(None), // errSecItemNotFound
            Err(_) => Err(keychain_error()),
        }
    }
    fn set(&self, account: &str, data: &[u8]) -> ApiResult<()> {
        security_framework::passwords::set_generic_password(SERVICE, account, data)
            .map_err(|_| keychain_error())
    }
    fn delete(&self, account: &str) -> ApiResult<()> {
        match security_framework::passwords::delete_generic_password(SERVICE, account) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == -25300 => Ok(()),
            Err(_) => Err(keychain_error()),
        }
    }
}

#[cfg(target_os = "macos")]
fn keychain_error() -> ApiError {
    ApiError::new("Keychain", "Нет доступа к связке ключей macOS")
}
