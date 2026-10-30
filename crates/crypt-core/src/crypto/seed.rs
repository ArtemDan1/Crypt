use crate::{CryptError, Result};
use bip39::{Language, Mnemonic};
use zeroize::Zeroizing;

pub struct SeedPhrase(Zeroizing<String>);
impl SeedPhrase {
    pub fn generate() -> Self {
        Self(Zeroizing::new(
            Mnemonic::generate_in(Language::English, 24)
                .expect("24 is a valid BIP39 word count")
                .to_string(),
        ))
    }
    pub fn parse(input: &str) -> Result<Self> {
        let lower = Zeroizing::new(input.to_lowercase());
        let normalized = Zeroizing::new(lower.split_whitespace().collect::<Vec<_>>().join(" "));
        let mnemonic = Mnemonic::parse_in(Language::English, &*normalized)
            .map_err(|_| CryptError::InvalidSeed)?;
        if mnemonic.word_count() != 24 {
            return Err(CryptError::InvalidSeed);
        }
        Ok(Self(normalized))
    }
    pub fn words(&self) -> Vec<&str> {
        self.0.split_whitespace().collect()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Debug for SeedPhrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SeedPhrase(***)")
    }
}
