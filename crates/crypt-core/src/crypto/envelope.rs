use crate::{CryptError, Result, SecretKey};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroizing;

pub fn seal(key: &SecretKey, aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .expect("plaintext must fit AEAD message limits");
    let mut out = Vec::with_capacity(29 + encrypted.len());
    out.extend_from_slice(b"CRPT");
    out.push(1);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&encrypted);
    out
}
pub fn open(key: &SecretKey, aad: &[u8], envelope: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if !envelope.starts_with(b"CRPT") {
        return Err(CryptError::BadMagic);
    }
    if envelope.len() < 29 {
        return Err(CryptError::Decrypt);
    }
    if envelope[4] != 1 {
        return Err(CryptError::UnsupportedEnvelope(envelope[4]));
    }
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    cipher
        .decrypt(
            XNonce::from_slice(&envelope[5..29]),
            Payload {
                msg: &envelope[29..],
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| CryptError::Decrypt)
}
