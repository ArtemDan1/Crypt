use crate::error::{ApiError, ApiResult};
use argon2::{Algorithm, Argon2, Params, Version};
use crypt_core::SecretKey;
use crypt_core::crypto::envelope::{open, seal};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[cfg(not(test))]
const PARAMS: (u32, u32, u32) = (64 * 1024, 3, 1);
#[cfg(test)]
const PARAMS: (u32, u32, u32) = (1024, 1, 1);

/// Data key wrapped under a PIN-derived key. Kept in the keychain, never in vault files.
#[derive(Serialize, Deserialize, Clone)]
pub struct PinWrap {
    pub salt: [u8; 16],
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
    pub envelope: Vec<u8>,
}
impl std::fmt::Debug for PinWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PinWrap(***)")
    }
}

pub fn validate_pin(pin: &str) -> ApiResult<()> {
    if (6..=12).contains(&pin.len()) && pin.bytes().all(|b| b.is_ascii_digit()) {
        Ok(())
    } else {
        Err(ApiError::new("Validation", "PIN — от 6 до 12 цифр"))
    }
}

fn aad(vault_id: &str) -> Vec<u8> {
    format!("crypt/v1/pin-wrap/{vault_id}").into_bytes()
}

fn derive(pin: &str, salt: &[u8; 16], m: u32, t: u32, p: u32) -> ApiResult<SecretKey> {
    let params = Params::new(m, t, p, Some(32)).map_err(|_| ApiError::new("Validation", "Некорректные параметры PIN"))?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pin.as_bytes(), salt, &mut *out)
        .map_err(|_| ApiError::new("Validation", "Не удалось обработать PIN"))?;
    Ok(SecretKey::from_bytes(*out))
}

pub fn wrap(pin: &str, data_key: &SecretKey, vault_id: &str) -> ApiResult<PinWrap> {
    validate_pin(pin)?;
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let (m_kib, t, p) = PARAMS;
    let k = derive(pin, &salt, m_kib, t, p)?;
    Ok(PinWrap { salt, m_kib, t, p, envelope: seal(&k, &aad(vault_id), data_key.as_bytes()) })
}

pub fn unwrap(pin: &str, w: &PinWrap, vault_id: &str) -> ApiResult<SecretKey> {
    let k = derive(pin, &w.salt, w.m_kib, w.t, w.p)?;
    let plain = open(&k, &aad(vault_id), &w.envelope)
        .map_err(|_| ApiError::new("WrongPin", "Неверный PIN"))?;
    let bytes: [u8; 32] = plain.as_slice().try_into().map_err(|_| ApiError::new("WrongPin", "Неверный PIN"))?;
    Ok(SecretKey::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypt_core::SecretKey;

    #[test]
    fn pin_wrap_roundtrip_and_wrong_pin() {
        let key = SecretKey::from_bytes([7; 32]);
        let w = wrap("482913", &key, "vid").unwrap();
        assert_eq!(unwrap("482913", &w, "vid").unwrap().as_bytes(), key.as_bytes());
        assert_eq!(unwrap("000000", &w, "vid").unwrap_err().code, "WrongPin");
        assert_eq!(unwrap("482913", &w, "other-vault").unwrap_err().code, "WrongPin");
    }

    #[test]
    fn pin_validation() {
        for bad in ["12345", "12ab56", "1234567890123", "", "12 3456"] {
            assert_eq!(validate_pin(bad).unwrap_err().code, "Validation", "{bad}");
        }
        for ok in ["123456", "123456789012"] {
            assert!(validate_pin(ok).is_ok());
        }
    }

    #[test]
    fn wrap_rejects_invalid_pin() {
        assert_eq!(wrap("12", &SecretKey::from_bytes([1; 32]), "v").unwrap_err().code, "Validation");
    }
}
