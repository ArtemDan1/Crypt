use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SecretKind {
    Login,
    Card,
    Token,
    SshKey,
    Note,
}

use zeroize::Zeroizing;
pub struct SecretRevision {
    pub value: Zeroizing<String>,
    pub replaced_at: i64,
}
impl std::fmt::Debug for SecretRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretRevision")
            .field("value", &"***")
            .field("replaced_at", &self.replaced_at)
            .finish()
    }
}
impl SecretKind {
    pub fn value_names(&self) -> &'static [&'static str] {
        match self {
            Self::Login => &["username", "password", "url", "totp_seed"],
            Self::Card => &["number", "holder", "expiry", "cvv", "pin"],
            Self::Token => &["token", "url"],
            Self::SshKey => &["private_key", "public_key", "passphrase", "comment"],
            Self::Note => &["note"],
        }
    }
}
