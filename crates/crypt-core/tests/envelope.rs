use crypt_core::{
    CryptError, SecretKey,
    crypto::envelope::{open, seal},
};
fn key(n: u8) -> SecretKey {
    SecretKey::from_bytes([n; 32])
}
#[test]
fn roundtrip() {
    let e = seal(&key(1), b"a", b"hello");
    assert_eq!(&*open(&key(1), b"a", &e).unwrap(), b"hello");
}
#[test]
fn nonce_is_random() {
    assert_ne!(seal(&key(1), b"a", b"x"), seal(&key(1), b"a", b"x"));
}
#[test]
fn authentication() {
    let mut e = seal(&key(1), b"a", b"x");
    assert!(matches!(open(&key(2), b"a", &e), Err(CryptError::Decrypt)));
    assert!(matches!(open(&key(1), b"b", &e), Err(CryptError::Decrypt)));
    let n = e.len() - 1;
    e[n] ^= 1;
    assert!(matches!(open(&key(1), b"a", &e), Err(CryptError::Decrypt)));
}
#[test]
fn malformed_envelopes_do_not_panic() {
    let mut e = seal(&key(1), b"a", b"x");
    assert!(matches!(
        open(&key(1), b"a", b"NOPE"),
        Err(CryptError::BadMagic)
    ));
    for len in 0..e.len() {
        assert!(open(&key(1), b"a", &e[..len]).is_err());
    }
    e[4] = 9;
    assert!(matches!(
        open(&key(1), b"a", &e),
        Err(CryptError::UnsupportedEnvelope(9))
    ));
}
