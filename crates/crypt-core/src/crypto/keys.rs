use super::{secret_key::SecretKey, seed::SeedPhrase};
use bip39::{Language, Mnemonic};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fmt;
use zeroize::Zeroizing;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VaultId(String);
impl fmt::Display for VaultId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
pub struct VaultKeys {
    pub data_key: SecretKey,
    pub sync_key: SecretKey,
    pub vault_id: VaultId,
}
impl VaultKeys {
    pub fn derive(seed: &SeedPhrase) -> Self {
        let mnemonic =
            Mnemonic::parse_in(Language::English, seed.as_str()).expect("SeedPhrase is validated");
        let bip39_seed = Zeroizing::new(mnemonic.to_seed(""));
        let hkdf = Hkdf::<Sha256>::new(Some(b"crypt/v1"), &*bip39_seed);
        let mut data = Zeroizing::new([0u8; 32]);
        let mut sync = Zeroizing::new([0u8; 32]);
        let mut id = Zeroizing::new([0u8; 16]);
        hkdf.expand(b"crypt/v1/data-key", &mut *data)
            .expect("valid HKDF length");
        hkdf.expand(b"crypt/v1/sync-auth", &mut *sync)
            .expect("valid HKDF length");
        hkdf.expand(b"crypt/v1/vault-id", &mut *id)
            .expect("valid HKDF length");
        Self {
            data_key: SecretKey::from_bytes(*data),
            sync_key: SecretKey::from_bytes(*sync),
            vault_id: VaultId(id.iter().map(|b| format!("{b:02x}")).collect()),
        }
    }
}
