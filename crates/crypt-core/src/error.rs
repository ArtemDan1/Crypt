use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptError {
    #[error("Введите пароль экспорта")]
    NeedsPassword,
    #[error("Неверный пароль экспорта")]
    ImportPassword,
    #[error("{0}")]
    BackupFormat(String),
    #[error("Эта фраза не подходит к файлу бэкапа")]
    BackupSeedMismatch,
    #[error("private key requires a passphrase")]
    PassphraseRequired,
    #[error("wrong private key passphrase")]
    WrongPassphrase,
    #[error("filesystem error")]
    Io(#[from] std::io::Error),
    #[error("invalid envelope magic")]
    BadMagic,
    #[error("unsupported envelope version: {0}")]
    UnsupportedEnvelope(u8),
    #[error("decryption failed")]
    Decrypt,
    #[error("invalid seed phrase")]
    InvalidSeed,
    #[error("wrong vault key")]
    WrongKey,
    #[error("vault directory is not empty")]
    VaultExists,
    #[error("not a vault")]
    NotAVault,
    #[error("unsupported vault format: {0}")]
    UnsupportedFormat(u32),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("CRDT operation failed")]
    Crdt(String),
    #[error("invalid JSON")]
    Json(#[from] serde_json::Error),
    #[error("wrong password or key file for the imported database")]
    WrongImportKey,
    /// Unreadable import source; the message is user-facing (Russian).
    #[error("import failed: {0}")]
    ImportFormat(String),
    #[error("corrupt vault file: {file}")]
    Corrupt { file: String },
}
pub type Result<T> = std::result::Result<T, CryptError>;
