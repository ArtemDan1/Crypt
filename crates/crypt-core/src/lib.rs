pub mod crypto;
pub mod error;
pub use crypto::{keys::VaultId, secret_key::SecretKey, seed::SeedPhrase};
pub use error::{CryptError, Result};
#[cfg(test)]
mod task1_tests {
    use super::*;
    const SEED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    #[test]
    fn seed_generation_validation_and_normalization() {
        let seed = SeedPhrase::generate();
        assert_eq!(seed.words().len(), 24);
        assert!(SeedPhrase::parse(seed.as_str()).is_ok());
        let messy = format!("  {}\n", SEED.to_uppercase().replace(' ', "\t "));
        assert_eq!(SeedPhrase::parse(&messy).unwrap().as_str(), SEED);
        for bad in ["abandon ".repeat(24), "abandon ".repeat(11), "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".into()] {
            assert!(matches!(SeedPhrase::parse(&bad), Err(CryptError::InvalidSeed)));
        }
        assert_eq!(format!("{seed:?}"), "SeedPhrase(***)");
    }
    #[test]
    fn keys_are_deterministic_distinct_and_redacted() {
        let seed = SeedPhrase::parse(SEED).unwrap();
        let a = crypto::keys::VaultKeys::derive(&seed);
        let b = crypto::keys::VaultKeys::derive(&seed);
        assert_eq!(a.data_key.as_bytes(), b.data_key.as_bytes());
        assert_ne!(a.data_key.as_bytes(), a.sync_key.as_bytes());
        assert_eq!(a.vault_id.to_string().len(), 32);
        assert_eq!(format!("{:?}", a.data_key), "SecretKey(***)");
    }
}

#[cfg(test)]
mod golden_tests {
    use super::*;
    #[test]
    fn derive_golden_values() {
        let seed = SeedPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
        let keys = crypto::keys::VaultKeys::derive(&seed);
        assert_eq!(
            keys.vault_id.to_string(),
            "be2c295f99d5a2cdb8196b45b5db9abf"
        );
        let data: String = keys
            .data_key
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            data,
            "6feeeb1c440f00da436fd11cbbb602a3c7d0293a8608ca24af6b83dc4712f8b4"
        );
    }
}
pub mod model;
pub mod storage;
#[cfg(feature = "fs")]
mod vault;
pub use model::ids::*;
pub use model::meta::{Snapshot, SnapshotEntry, SnapshotInfo, SnapshotRestoreReport};
pub use model::record::{ListFilter, NewRecord, RecordSummary, RecordVersion, RecordView};
pub use model::secret::SecretKind;
pub use model::secret::SecretRevision;
pub use model::types::*;
#[cfg(feature = "fs")]
pub use vault::{ExportReport, KeepassReport, MarkdownImportReport, Vault};
pub mod index;
pub use index::SearchHit;

pub mod r#gen;
pub mod totp;

pub mod ssh;

#[cfg(feature = "fs")]
pub use vault::backup;

#[cfg(feature="fs")]
pub use vault::{import_csv,import_apply};

#[cfg(feature="fs")]
pub use vault::import_otp;
