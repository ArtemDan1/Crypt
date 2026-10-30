#![cfg(feature = "fs")]
use crypt_core::{ssh::*, *};
use signature::Verifier;
fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/ssh/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
fn verify(public: &str, wire: &[u8]) {
    ssh_key::PublicKey::from_openssh(public)
        .unwrap()
        .key_data()
        .verify(b"challenge", &ssh_key::Signature::try_from(wire).unwrap())
        .unwrap();
}
#[test]
fn inspect_matches_ssh_keygen() {
    for name in ["ed25519", "ecdsa256", "ecdsa384", "rsa3072"] {
        let (info, plain) = inspect_private_key(&fixture(name), None).unwrap();
        let public = fixture(&format!("{name}.pub"))
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(info.public_key, public);
        assert_eq!(info.comment, "fixture@crypt");
        let fingerprints = fixture("fingerprints.txt");
        let line = fingerprints
            .lines()
            .find(|l| l.starts_with(&format!("{name} ")))
            .unwrap();
        assert_eq!(info.fingerprint, line.split_whitespace().nth(2).unwrap());
        assert_eq!(is_encrypted_private_key(&plain), Some(false));
    }
}
#[test]
fn encrypted_key_needs_correct_passphrase() {
    let key = fixture("ed25519_enc");
    assert!(matches!(
        inspect_private_key(&key, None),
        Err(CryptError::PassphraseRequired)
    ));
    assert!(matches!(
        inspect_private_key(&key, Some("wrong")),
        Err(CryptError::WrongPassphrase)
    ));
    let (_, plain) = inspect_private_key(&key, Some("correct horse")).unwrap();
    assert_eq!(is_encrypted_private_key(&plain), Some(false));
    assert_eq!(is_encrypted_private_key(&key), Some(true));
    assert_eq!(is_encrypted_private_key(&fixture("ed25519.pub")), None);
}
#[test]
fn junk_is_rejected() {
    for bad in [
        "",
        "hello",
        "PuTTY-User-Key-File-3: ssh-ed25519",
        "host ssh-rsa AAAA",
    ] {
        assert!(matches!(
            inspect_private_key(bad, None),
            Err(CryptError::Validation(_))
        ));
    }
}
#[test]
fn sign_verifies_with_public_key() {
    let (info, key) = generate_ed25519("t").unwrap();
    assert_eq!(info.comment, "t");
    verify(&info.public_key, &sign(&key, b"challenge", 0).unwrap());
    for name in ["ed25519", "ecdsa256", "ecdsa384", "rsa3072"] {
        let (info, key) = inspect_private_key(&fixture(name), None).unwrap();
        for flags in [0, 2, 4] {
            let sig = sign(&key, b"challenge", flags).unwrap();
            if name == "rsa3072" {
                let n = u32::from_be_bytes(sig[..4].try_into().unwrap()) as usize;
                assert_eq!(
                    &sig[4..4 + n],
                    match flags {
                        2 => b"rsa-sha2-256".as_slice(),
                        4 => b"rsa-sha2-512".as_slice(),
                        _ => b"ssh-rsa".as_slice(),
                    }
                );
                if flags == 0 {
                    let public = ssh_key::PublicKey::from_openssh(&info.public_key).unwrap();
                    let rsa =
                        rsa::RsaPublicKey::try_from(public.key_data().rsa().unwrap()).unwrap();
                    rsa::pkcs1v15::VerifyingKey::<sha1_ssh::Sha1>::new(rsa)
                        .verify(
                            b"challenge",
                            &rsa::pkcs1v15::Signature::try_from(&sig[8 + n..]).unwrap(),
                        )
                        .unwrap();
                    continue;
                }
            }
            verify(&info.public_key, &sig);
        }
    }
}
#[test]
fn vault_ssh_roundtrip_and_agent_identities() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(dir.path(), &SeedPhrase::generate()).unwrap();
    let id = v
        .create_record(NewRecord {
            title: "work".into(),
            secret_kind: Some(SecretKind::SshKey),
            ..Default::default()
        })
        .unwrap();
    let info = v
        .set_ssh_private_key(&id, &fixture("ed25519"), None)
        .unwrap();
    assert_eq!(v.ssh_key_info(&id).unwrap().unwrap(), info);
    assert_eq!(v.agent_identities()[0].id, id);
    verify(
        &info.public_key,
        &v.sign_with(&id, b"challenge", 0).unwrap(),
    );
    v.set_agent_enabled(&id, false).unwrap();
    assert!(v.agent_identities().is_empty());
    assert!(v.sign_with(&id, b"challenge", 0).is_err());
    v.set_agent_enabled(&id, true).unwrap();
    v.delete(&id).unwrap();
    assert!(v.agent_identities().is_empty());
    let login = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Login),
            ..Default::default()
        })
        .unwrap();
    assert!(
        v.set_ssh_private_key(&login, &fixture("ed25519"), None)
            .is_err()
    );
}
#[test]
fn vault_totp_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(dir.path(), &SeedPhrase::generate()).unwrap();
    let id = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Login),
            ..Default::default()
        })
        .unwrap();
    assert!(v.totp_code(&id, 59).unwrap().is_none());
    v.set_secret_value(&id, "totp_seed", Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"))
        .unwrap();
    assert_eq!(v.totp_code(&id, 59).unwrap().unwrap().code, "287082");
}
#[test]
fn pem_keys_are_normalized() {
    for name in ["rsa_pem", "ecdsa_pem", "ecdsa384_pem"] {
        for suffix in ["", "_pkcs8"] {
            let (info, pem) = inspect_private_key(&fixture(&format!("{name}{suffix}")), None)
                .unwrap_or_else(|e| panic!("{name}{suffix}: {e}"));
            assert!(pem.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
            let public = fixture(&format!("{name}.pub"))
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(info.public_key, public);
            verify(&public, &sign(&pem, b"challenge", 4).unwrap());
        }
    }
    assert!(
        inspect_private_key(
            "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\n",
            None
        )
        .is_err()
    );
}
