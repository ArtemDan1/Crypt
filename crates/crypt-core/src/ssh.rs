use crate::{CryptError, RecordId, Result};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::pkcs8::DecodePrivateKey;
use rsa::{pkcs1::DecodeRsaPrivateKey, traits::PublicKeyParts};
use serde::{Deserialize, Serialize};
use signature::{SignatureEncoding, Signer};
use ssh_key::{
    Algorithm, EcdsaCurve, HashAlg, PrivateKey, PublicKey,
    private::{EcdsaKeypair, KeypairData, RsaKeypair},
};
use zeroize::Zeroizing;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SshKeyInfo {
    pub algorithm: String,
    pub bits: Option<u32>,
    pub public_key: String,
    pub fingerprint: String,
    pub comment: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AgentIdentity {
    pub id: RecordId,
    pub title: String,
    pub public_key: String,
    pub fingerprint: String,
}
fn invalid(_: impl std::fmt::Debug) -> CryptError {
    CryptError::Validation("not a private key".into())
}
fn read_key(pem: &str) -> Result<PrivateKey> {
    if pem.contains("Proc-Type: 4,ENCRYPTED") {
        return Err(CryptError::Validation(
            "старый формат с паролем: выполните ssh-keygen -p -f <file>".into(),
        ));
    }
    if let Ok(k) = PrivateKey::from_openssh(pem) {
        return Ok(k);
    }
    if let Ok(k) =
        rsa::RsaPrivateKey::from_pkcs1_pem(pem).or_else(|_| rsa::RsaPrivateKey::from_pkcs8_pem(pem))
    {
        return PrivateKey::new(
            KeypairData::Rsa(RsaKeypair::try_from(k).map_err(invalid)?),
            "",
        )
        .map_err(invalid);
    }
    if let Ok(k) =
        p256::SecretKey::from_sec1_pem(pem).or_else(|_| p256::SecretKey::from_pkcs8_pem(pem))
    {
        let public = k.public_key().to_encoded_point(false);
        let private = k.into();
        return PrivateKey::new(
            KeypairData::Ecdsa(EcdsaKeypair::NistP256 { public, private }),
            "",
        )
        .map_err(invalid);
    }
    if let Ok(k) =
        p384::SecretKey::from_sec1_pem(pem).or_else(|_| p384::SecretKey::from_pkcs8_pem(pem))
    {
        let public = k.public_key().to_encoded_point(false);
        let private = k.into();
        return PrivateKey::new(
            KeypairData::Ecdsa(EcdsaKeypair::NistP384 { public, private }),
            "",
        )
        .map_err(invalid);
    }
    explicit_ec(pem)
}
pub fn public_key_info(public: &str) -> Result<SshKeyInfo> {
    let key = PublicKey::from_openssh(public).map_err(invalid)?;
    info(&key)
}
fn info(key: &PublicKey) -> Result<SshKeyInfo> {
    let bits = match key.algorithm() {
        Algorithm::Ed25519 => None,
        Algorithm::Rsa { .. } => Some(
            rsa::RsaPublicKey::try_from(key.key_data().rsa().ok_or_else(|| invalid(()))?)
                .map_err(invalid)?
                .n()
                .bits() as u32,
        ),
        Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        } => Some(256),
        Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP384,
        } => Some(384),
        _ => return Err(invalid(())),
    };
    let mut public = key.clone();
    public.set_comment("");
    Ok(SshKeyInfo {
        algorithm: key.algorithm().as_str().into(),
        bits,
        public_key: public.to_openssh().map_err(invalid)?.trim().into(),
        fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
        comment: key.comment().into(),
    })
}
pub fn inspect_private_key(
    pem: &str,
    passphrase: Option<&str>,
) -> Result<(SshKeyInfo, Zeroizing<String>)> {
    let key = read_key(pem)?;
    let key = if key.is_encrypted() {
        key.decrypt(passphrase.ok_or(CryptError::PassphraseRequired)?)
            .map_err(|_| CryptError::WrongPassphrase)?
    } else {
        key
    };
    let info = info(key.public_key())?;
    Ok((
        info,
        key.to_openssh(ssh_key::LineEnding::LF).map_err(invalid)?,
    ))
}
pub fn is_encrypted_private_key(pem: &str) -> Option<bool> {
    let key = read_key(pem).ok()?;
    info(key.public_key()).ok()?;
    Some(key.is_encrypted())
}
pub fn generate_ed25519(comment: &str) -> Result<(SshKeyInfo, Zeroizing<String>)> {
    let mut k = PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519).map_err(invalid)?;
    k.set_comment(comment);
    inspect_private_key(
        &k.to_openssh(ssh_key::LineEnding::LF).map_err(invalid)?,
        None,
    )
}
pub fn public_key_blob(public_key: &str) -> Result<Vec<u8>> {
    PublicKey::from_openssh(public_key)
        .map_err(invalid)?
        .to_bytes()
        .map_err(invalid)
}
pub fn sign(pem: &str, data: &[u8], flags: u32) -> Result<Vec<u8>> {
    let key = read_key(pem)?;
    let sig = if let KeypairData::Rsa(r) = key.key_data() {
        // ssh-key 0.6.7's conversion repeats p; construct with both distinct primes.
        let private = rsa::RsaPrivateKey::from_components(
            rsa::BigUint::try_from(&r.public.n).map_err(invalid)?,
            rsa::BigUint::try_from(&r.public.e).map_err(invalid)?,
            rsa::BigUint::try_from(&r.private.d).map_err(invalid)?,
            vec![
                rsa::BigUint::try_from(&r.private.p).map_err(invalid)?,
                rsa::BigUint::try_from(&r.private.q).map_err(invalid)?,
            ],
        )
        .map_err(invalid)?;
        let (algorithm, bytes) = if flags & 2 != 0 {
            (
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha256),
                },
                rsa::pkcs1v15::SigningKey::<sha2::Sha256>::new(private)
                    .try_sign(data)
                    .map_err(invalid)?
                    .to_vec(),
            )
        } else if flags & 4 != 0 {
            (
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha512),
                },
                rsa::pkcs1v15::SigningKey::<sha2::Sha512>::new(private)
                    .try_sign(data)
                    .map_err(invalid)?
                    .to_vec(),
            )
        } else {
            (
                Algorithm::Rsa { hash: None },
                rsa::pkcs1v15::SigningKey::<sha1_ssh::Sha1>::new(private)
                    .try_sign(data)
                    .map_err(invalid)?
                    .to_vec(),
            )
        };
        let name = algorithm.as_str().as_bytes();
        let mut out = Vec::new();
        out.extend_from_slice(&(name.len() as u32).to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(&bytes);
        return Ok(out);
    } else {
        key.try_sign(data).map_err(invalid)?
    };
    Vec::try_from(sig).map_err(invalid)
}

// Older macOS ssh-keygen emits explicit EC parameters. Recover only when the
// embedded public point exactly matches the scalar on a supported NIST curve.
fn explicit_ec(pem: &str) -> Result<PrivateKey> {
    use p256::pkcs8::{
        PrivateKeyInfo, SecretDocument,
        der::{
            Decode, Reader, SliceReader, Tag, TagNumber, Tagged,
            asn1::{AnyRef, BitStringRef, OctetStringRef},
        },
    };
    let (label, document) = SecretDocument::from_pem(pem).map_err(invalid)?;
    let bytes = if label == "PRIVATE KEY" {
        let info = PrivateKeyInfo::from_der(document.as_bytes()).map_err(invalid)?;
        if info.algorithm.oid.to_string() != "1.2.840.10045.2.1" {
            return Err(invalid(()));
        }
        info.private_key
    } else if label == "EC PRIVATE KEY" {
        document.as_bytes()
    } else {
        return Err(invalid(()));
    };
    let mut reader = SliceReader::new(bytes).map_err(invalid)?;
    let (secret, public) = reader
        .sequence(|r| {
            let version = u8::decode(r)?;
            if version != 1 {
                return Err(r.error(p256::pkcs8::der::ErrorKind::Failed));
            }
            let secret = OctetStringRef::decode(r)?.as_bytes();
            let mut public = None;
            while !r.is_finished() {
                let field = AnyRef::decode(r)?;
                if field.tag()
                    == (Tag::ContextSpecific {
                        constructed: true,
                        number: TagNumber::N1,
                    })
                {
                    public = BitStringRef::from_der(field.value())?.as_bytes();
                }
            }
            Ok((secret, public))
        })
        .map_err(invalid)?;
    let public = public.ok_or_else(|| invalid(()))?;
    if secret.len() == 32 {
        let key = p256::SecretKey::from_slice(secret).map_err(invalid)?;
        let point = key.public_key().to_encoded_point(false);
        if point.as_bytes() != public {
            return Err(invalid(()));
        }
        PrivateKey::new(
            KeypairData::Ecdsa(EcdsaKeypair::NistP256 {
                public: point,
                private: key.into(),
            }),
            "",
        )
        .map_err(invalid)
    } else if secret.len() == 48 {
        let key = p384::SecretKey::from_slice(secret).map_err(invalid)?;
        let point = key.public_key().to_encoded_point(false);
        if point.as_bytes() != public {
            return Err(invalid(()));
        }
        PrivateKey::new(
            KeypairData::Ecdsa(EcdsaKeypair::NistP384 {
                public: point,
                private: key.into(),
            }),
            "",
        )
        .map_err(invalid)
    } else {
        Err(invalid(()))
    }
}
