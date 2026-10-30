use crate::{CryptError, Result};
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use zeroize::Zeroizing;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TotpAlg {
    Sha1,
    Sha256,
    Sha512,
}
pub struct TotpParams {
    pub secret: Zeroizing<Vec<u8>>,
    pub algorithm: TotpAlg,
    pub digits: u32,
    pub period: u64,
}
#[derive(Serialize)]
pub struct TotpCode {
    pub code: String,
    pub remaining: u64,
    pub period: u64,
}
fn invalid() -> CryptError {
    CryptError::Validation("invalid TOTP parameters".into())
}
fn decode(s: &str) -> Result<String> {
    let mut out = Vec::new();
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        out.push(if b == b'%' {
            let a = bytes.next().ok_or_else(invalid)?;
            let b = bytes.next().ok_or_else(invalid)?;
            u8::from_str_radix(std::str::from_utf8(&[a, b]).map_err(|_| invalid())?, 16)
                .map_err(|_| invalid())?
        } else {
            b
        });
    }
    String::from_utf8(out).map_err(|_| invalid())
}
pub fn parse_totp(input: &str) -> Result<TotpParams> {
    let mut secret = Zeroizing::new(input.to_string());
    let mut algorithm = TotpAlg::Sha1;
    let mut digits = 6;
    let mut period = 30;
    if input.contains("://") {
        let uri = input.strip_prefix("otpauth://totp/").ok_or_else(invalid)?;
        let (_, query) = uri.split_once('?').ok_or_else(invalid)?;
        let mut found = false;
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').ok_or_else(invalid)?;
            let value = Zeroizing::new(decode(value)?);
            match key {
                "secret" => {
                    secret = value;
                    found = true;
                }
                "algorithm" => {
                    algorithm = match value.to_ascii_uppercase().as_str() {
                        "SHA1" => TotpAlg::Sha1,
                        "SHA256" => TotpAlg::Sha256,
                        "SHA512" => TotpAlg::Sha512,
                        _ => return Err(invalid()),
                    }
                }
                "digits" => digits = value.parse().map_err(|_| invalid())?,
                "period" => period = value.parse().map_err(|_| invalid())?,
                _ => {}
            }
        }
        if !found {
            return Err(invalid());
        }
    }
    if !(6..=8).contains(&digits) || period == 0 {
        return Err(invalid());
    }
    let normalized = Zeroizing::new(
        secret
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>(),
    );
    let encoding = if normalized.contains('=') {
        data_encoding::BASE32
    } else {
        data_encoding::BASE32_NOPAD
    };
    let secret = Zeroizing::new(
        encoding
            .decode(normalized.as_bytes())
            .map_err(|_| invalid())?,
    );
    if secret.is_empty() {
        return Err(invalid());
    }
    Ok(TotpParams {
        secret,
        algorithm,
        digits,
        period,
    })
}
pub fn totp_at(p: &TotpParams, unix_secs: u64) -> TotpCode {
    let counter = (unix_secs / p.period).to_be_bytes();
    macro_rules! digest {
        ($hash:ty) => {{
            let mut m = Hmac::<$hash>::new_from_slice(&p.secret).expect("HMAC accepts any key");
            m.update(&counter);
            Zeroizing::new(m.finalize().into_bytes().to_vec())
        }};
    }
    let digest = match p.algorithm {
        TotpAlg::Sha1 => digest!(sha1::Sha1),
        TotpAlg::Sha256 => digest!(sha2_totp::Sha256),
        TotpAlg::Sha512 => digest!(sha2_totp::Sha512),
    };
    let offset = (digest[digest.len() - 1] & 15) as usize;
    let n = u32::from_be_bytes(digest[offset..offset + 4].try_into().unwrap()) & 0x7fffffff;
    TotpCode {
        code: format!(
            "{:0width$}",
            n % 10u32.pow(p.digits),
            width = p.digits as usize
        ),
        remaining: p.period - unix_secs % p.period,
        period: p.period,
    }
}
