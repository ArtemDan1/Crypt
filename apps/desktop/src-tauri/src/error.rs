use crypt_core::CryptError;
use serde::Serialize;

/// Error shape the frontend receives: `code` is stable, `message` is safe to show.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }
}

impl From<CryptError> for ApiError {
    fn from(e: CryptError) -> Self {
        let (code, message): (&str, String) = match &e {
            CryptError::PassphraseRequired => ("PassphraseRequired", "Ключ защищён паролем".into()),
            CryptError::WrongPassphrase => ("WrongPassphrase", "Неверный пароль ключа".into()),
            CryptError::Io(_) => ("Io", "Ошибка чтения или записи файлов хранилища".into()),
            CryptError::BadMagic | CryptError::UnsupportedEnvelope(_) | CryptError::Decrypt => {
                ("Decrypt", "Не удалось расшифровать данные".into())
            }
            CryptError::InvalidSeed => ("InvalidSeed", "Сид-фраза некорректна: нужно 24 слова из словаря BIP39".into()),
            CryptError::WrongKey => ("WrongKey", "Неверная сид-фраза или ключ".into()),
            CryptError::VaultExists => ("VaultExists", "Хранилище уже существует".into()),
            CryptError::NotAVault => ("NotAVault", "Хранилище не найдено".into()),
            CryptError::UnsupportedFormat(n) => ("UnsupportedFormat", format!("Формат хранилища {n} не поддерживается этой версией")),
            CryptError::NotFound(_) => ("NotFound", "Запись не найдена".into()),
            CryptError::Validation(m) => ("Validation", m.clone()),
            CryptError::Crdt(_) => ("Crdt", "Внутренняя ошибка документа".into()),
            CryptError::Json(_) => ("Json", "Повреждённые данные".into()),
            CryptError::Corrupt { file } => ("Corrupt", format!("Повреждён файл {file}")),
            CryptError::WrongImportKey => ("WrongImportKey", "Неверный мастер-пароль или ключ-файл".into()),
            CryptError::ImportFormat(m) => ("ImportFormat", m.clone()),
        };
        Self::new(code, message)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
