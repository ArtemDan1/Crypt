//! Local OTP imports. Aegis format: github.com/beemdevelopment/Aegis/blob/master/docs/vault.md
use crate::{
    CryptError, RecordId, Result,
    totp::{TotpAlg, parse_totp},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use zeroize::Zeroizing;
pub struct OtpDraft {
    pub issuer: String,
    pub account: String,
    pub secret: Zeroizing<Vec<u8>>,
    pub algorithm: TotpAlg,
    pub digits: u32,
    pub period: u32,
}
#[derive(Default)]
pub struct OtpParse {
    pub drafts: Vec<OtpDraft>,
    pub skipped: Vec<(String, OtpSkip)>,
    pub missing_batches: Vec<(u32, u32)>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum OtpSkip {
    Hotp,
    BadSecret,
    Unsupported,
}
#[derive(Debug)]
pub enum OtpTarget {
    Attach { record: RecordId, title: String },
    New,
}
#[derive(Debug)]
pub struct OtpPreviewRow {
    pub issuer: String,
    pub account: String,
    pub target: OtpTarget,
}
#[derive(Debug, Serialize)]
pub struct OtpImportReport {
    pub attached: usize,
    pub created: usize,
    pub skipped: Vec<(String, OtpSkip)>,
    pub missing_batches: Vec<(u32, u32)>,
}
fn invalid() -> CryptError {
    CryptError::ImportFormat("Файл кодов 2FA не распознан или повреждён".into())
}
fn percent(text: &str) -> Result<String> {
    let mut out = Vec::new();
    let mut iter = text.bytes();
    while let Some(b) = iter.next() {
        if b == b'%' {
            let a = iter.next().ok_or_else(invalid)?;
            let b = iter.next().ok_or_else(invalid)?;
            let pair = [a, b];
            out.push(
                u8::from_str_radix(std::str::from_utf8(&pair).map_err(|_| invalid())?, 16)
                    .map_err(|_| invalid())?,
            );
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out).map_err(|_| invalid())
}
fn b64(text: &str) -> Result<Zeroizing<Vec<u8>>> {
    for encoding in [
        data_encoding::BASE64,
        data_encoding::BASE64_NOPAD,
        data_encoding::BASE64URL,
        data_encoding::BASE64URL_NOPAD,
    ] {
        if let Ok(v) = encoding.decode(text.as_bytes()) {
            return Ok(Zeroizing::new(v));
        }
    }
    Err(invalid())
}
fn unhex(text: &str) -> Result<Zeroizing<Vec<u8>>> {
    data_encoding::HEXLOWER_PERMISSIVE
        .decode(text.as_bytes())
        .map(Zeroizing::new)
        .map_err(|_| invalid())
}
pub fn decode_qr_images(images: &[Vec<u8>]) -> Result<Vec<String>> {
    let mut result = vec![];
    for bytes in images {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| invalid())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().map_err(|_| invalid())?.to_luma8();
        let mut prepared = rqrr::PreparedImage::prepare(image);
        let grids = prepared.detect_grids();
        if grids.is_empty() {
            return Err(CryptError::ImportFormat(
                "На изображении не найден QR-код".into(),
            ));
        }
        for grid in grids {
            let (_, text) = grid.decode().map_err(|_| {
                CryptError::ImportFormat("Не удалось прочитать один из QR-кодов".into())
            })?;
            result.push(text);
        }
    }
    Ok(result)
}
pub fn otp_uri(d: &OtpDraft) -> Zeroizing<String> {
    let algorithm = match d.algorithm {
        TotpAlg::Sha1 => "SHA1",
        TotpAlg::Sha256 => "SHA256",
        TotpAlg::Sha512 => "SHA512",
    };
    let escape = |text: &str| -> String {
        text.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    };
    let secret = Zeroizing::new(data_encoding::BASE32_NOPAD.encode(&d.secret));
    Zeroizing::new(format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={algorithm}&digits={}&period={}",
        escape(&d.issuer),
        escape(&d.account),
        &*secret,
        escape(&d.issuer),
        d.digits,
        d.period
    ))
}
fn normal_uri(uri: &str) -> std::result::Result<OtpDraft, OtpSkip> {
    if uri.starts_with("otpauth://hotp/") {
        return Err(OtpSkip::Hotp);
    }
    let rest = uri
        .strip_prefix("otpauth://totp/")
        .ok_or(OtpSkip::Unsupported)?;
    let (label, query) = rest.split_once('?').ok_or(OtpSkip::BadSecret)?;
    let label = percent(label).map_err(|_| OtpSkip::BadSecret)?;
    let (prefix, account) = label
        .split_once(':')
        .map(|(i, a)| (i, a))
        .unwrap_or(("", &label));
    let mut issuer = prefix.to_owned();
    for pair in query.split('&') {
        if let Some(("issuer", value)) = pair.split_once('=') {
            issuer = percent(value).map_err(|_| OtpSkip::BadSecret)?;
        }
    }
    let params = parse_totp(uri).map_err(|_| OtpSkip::BadSecret)?;
    Ok(OtpDraft {
        issuer,
        account: account.to_owned(),
        secret: params.secret,
        algorithm: params.algorithm,
        digits: params.digits,
        period: u32::try_from(params.period).map_err(|_| OtpSkip::Unsupported)?,
    })
}
// Bounds-checked protobuf reader; ignores unknown fields, rejects truncated/overflowing lengths.
fn varint(bytes: &[u8], at: &mut usize) -> Result<u64> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let b = *bytes.get(*at).ok_or_else(invalid)?;
        *at += 1;
        if shift == 63 && b > 1 {
            return Err(invalid());
        }
        value |= ((b & 127) as u64) << shift;
        if b & 128 == 0 {
            return Ok(value);
        }
    }
    Err(invalid())
}
enum Pb<'a> {
    Int(u64),
    Bytes(&'a [u8]),
}
fn protobuf(bytes: &[u8]) -> Result<Vec<(u32, Pb<'_>)>> {
    let mut out = vec![];
    let mut at = 0;
    while at < bytes.len() {
        let tag = varint(bytes, &mut at)?;
        let number = (tag >> 3) as u32;
        if number == 0 {
            return Err(invalid());
        }
        let field = match tag & 7 {
            0 => Pb::Int(varint(bytes, &mut at)?),
            2 => {
                let len = usize::try_from(varint(bytes, &mut at)?).map_err(|_| invalid())?;
                let end = at.checked_add(len).ok_or_else(invalid)?;
                let data = bytes.get(at..end).ok_or_else(invalid)?;
                at = end;
                Pb::Bytes(data)
            }
            1 | 5 => {
                at = at
                    .checked_add(if tag & 7 == 1 { 8 } else { 4 })
                    .filter(|end| *end <= bytes.len())
                    .ok_or_else(invalid)?;
                continue;
            }
            _ => return Err(invalid()),
        };
        out.push((number, field));
        if out.len() > 100_000 {
            return Err(invalid());
        }
    }
    Ok(out)
}
fn migration_otp(bytes: &[u8]) -> Result<std::result::Result<OtpDraft, OtpSkip>> {
    let mut secret = Zeroizing::new(vec![]);
    let mut name = String::new();
    let mut issuer = String::new();
    let mut algorithm = 0;
    let mut digits = 0;
    let mut kind = 0;
    for (n, value) in protobuf(bytes)? {
        match (n, value) {
            (1, Pb::Bytes(v)) => *secret = v.to_vec(),
            (2, Pb::Bytes(v)) => name = std::str::from_utf8(v).map_err(|_| invalid())?.to_owned(),
            (3, Pb::Bytes(v)) => issuer = std::str::from_utf8(v).map_err(|_| invalid())?.to_owned(),
            (4, Pb::Int(v)) => algorithm = v,
            (5, Pb::Int(v)) => digits = v,
            (6, Pb::Int(v)) => kind = v,
            _ => {}
        }
    }
    if kind == 1 {
        return Ok(Err(OtpSkip::Hotp));
    }
    if kind != 2 {
        return Ok(Err(OtpSkip::Unsupported));
    }
    let algorithm = match algorithm {
        1 => TotpAlg::Sha1,
        2 => TotpAlg::Sha256,
        3 => TotpAlg::Sha512,
        _ => return Ok(Err(OtpSkip::Unsupported)),
    };
    let digits = match digits {
        1 => 6,
        2 => 8,
        _ => return Ok(Err(OtpSkip::Unsupported)),
    };
    if secret.is_empty() {
        return Ok(Err(OtpSkip::BadSecret));
    }
    if let Some((i, a)) = name.split_once(':') {
        if issuer.is_empty() {
            issuer = i.to_owned();
        }
        name = a.to_owned();
    }
    Ok(Ok(OtpDraft {
        issuer,
        account: name,
        secret,
        algorithm,
        digits,
        period: 30,
    }))
}
pub fn parse_otp_uris(uris: &[String]) -> OtpParse {
    let mut result = OtpParse::default();
    let mut batches: BTreeMap<u64, (u32, BTreeSet<u32>)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (i, uri) in uris.iter().enumerate() {
        if !seen.insert(uri.as_str()) {
            continue;
        }
        let label = format!("QR №{}", i + 1);
        if uri.starts_with("otpauth-migration://") {
            let parse = (|| -> Result<_> {
                let query = uri.split_once('?').ok_or_else(invalid)?.1;
                let data = query
                    .split('&')
                    .find_map(|p| p.strip_prefix("data="))
                    .ok_or_else(invalid)?;
                let decoded = Zeroizing::new(percent(data)?);
                let bytes = b64(&decoded)?;
                let fields = protobuf(&bytes)?;
                let mut parameters = vec![];
                let mut version = 0;
                let mut size = 1;
                let mut index = 0;
                let mut id = 0;
                for (n, value) in fields {
                    match (n, value) {
                        (1, Pb::Bytes(v)) => parameters.push(v),
                        (2, Pb::Int(v)) => version = v,
                        (3, Pb::Int(v)) => size = v,
                        (4, Pb::Int(v)) => index = v,
                        (5, Pb::Int(v)) => id = v,
                        _ => {}
                    }
                }
                if version != 1 || size == 0 || size > 1000 || index >= size {
                    return Err(invalid());
                }
                let mut drafts = vec![];
                let mut skipped = vec![];
                for parameter in parameters {
                    match migration_otp(parameter)? {
                        Ok(d) => drafts.push(d),
                        Err(reason) => skipped.push((label.clone(), reason)),
                    }
                }
                Ok((drafts, skipped, id, size as u32, index as u32))
            })();
            match parse {
                Ok((drafts, skipped, id, size, index)) => {
                    let entry = batches.entry(id).or_insert((size, BTreeSet::new()));
                    if entry.0 != size {
                        result.skipped.push((label, OtpSkip::Unsupported));
                        continue;
                    }
                    if !entry.1.insert(index) {
                        continue;
                    }
                    result.drafts.extend(drafts);
                    result.skipped.extend(skipped);
                }
                Err(_) => result.skipped.push((label, OtpSkip::BadSecret)),
            }
        } else {
            match normal_uri(uri) {
                Ok(d) => result.drafts.push(d),
                Err(reason) => result.skipped.push((label, reason)),
            }
        }
    }
    for (_, (size, indices)) in batches {
        for i in 0..size {
            if !indices.contains(&i) {
                result.missing_batches.push((i + 1, size));
            }
        }
    }
    result
}
fn secret<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    String::deserialize(d).map(Zeroizing::new)
}
#[derive(Deserialize)]
struct OtpInfo {
    #[serde(default, deserialize_with = "secret")]
    secret: Zeroizing<String>,
    #[serde(default)]
    algo: String,
    #[serde(default = "six")]
    digits: u32,
    #[serde(default = "thirty")]
    period: u32,
}
fn six() -> u32 {
    6
}
fn thirty() -> u32 {
    30
}
#[derive(Deserialize)]
struct AegisEntry {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    issuer: String,
    info: OtpInfo,
}
#[derive(Deserialize)]
struct AegisDb {
    entries: Vec<AegisEntry>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum AegisData {
    Plain(AegisDb),
    Encrypted(String),
}
#[derive(Deserialize)]
struct AegisFile {
    version: u32,
    #[serde(default)]
    header: serde_json::Value,
    db: AegisData,
}
fn make_draft(
    issuer: String,
    account: String,
    kind: &str,
    secret: &str,
    algorithm: &str,
    digits: u32,
    period: u32,
) -> std::result::Result<OtpDraft, OtpSkip> {
    if kind.eq_ignore_ascii_case("hotp") {
        return Err(OtpSkip::Hotp);
    }
    if !kind.eq_ignore_ascii_case("totp") {
        return Err(OtpSkip::Unsupported);
    }
    let algorithm = match algorithm.to_ascii_uppercase().as_str() {
        "SHA1" | "" => TotpAlg::Sha1,
        "SHA256" => TotpAlg::Sha256,
        "SHA512" => TotpAlg::Sha512,
        _ => return Err(OtpSkip::Unsupported),
    };
    if !(6..=8).contains(&digits) || period == 0 {
        return Err(OtpSkip::Unsupported);
    }
    let parsed = parse_totp(secret).map_err(|_| OtpSkip::BadSecret)?;
    Ok(OtpDraft {
        issuer,
        account,
        secret: parsed.secret,
        algorithm,
        digits,
        period,
    })
}
fn decrypt(key: &[u8], cipher: &[u8], params: &serde_json::Value) -> Result<Zeroizing<Vec<u8>>> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::AeadInPlace};
    let nonce = unhex(params["nonce"].as_str().ok_or_else(invalid)?)?;
    let tag = unhex(params["tag"].as_str().ok_or_else(invalid)?)?;
    if nonce.len() != 12 || tag.len() != 16 || key.len() != 32 {
        return Err(invalid());
    }
    let mut plain = Zeroizing::new(cipher.to_vec());
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| invalid())?
        .decrypt_in_place_detached(
            nonce.as_slice().into(),
            b"",
            &mut *plain,
            tag.as_slice().into(),
        )
        .map_err(|_| CryptError::ImportPassword)?;
    Ok(plain)
}
pub fn parse_aegis(bytes: &[u8], password: Option<&str>) -> Result<OtpParse> {
    let file: AegisFile = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if file.version != 1 {
        return Err(invalid());
    }
    let db = match file.db {
        AegisData::Plain(db) => db,
        AegisData::Encrypted(cipher) => {
            let password = password.ok_or(CryptError::NeedsPassword)?;
            let cipher = b64(&cipher)?;
            let slots = file.header["slots"].as_array().ok_or_else(invalid)?;
            if slots.len() > 32 {
                return Err(invalid());
            }
            let mut master = None;
            for slot in slots.iter().filter(|s| s["type"].as_u64() == Some(1)) {
                let n = slot["n"].as_u64().ok_or_else(invalid)?;
                let r = u32::try_from(slot["r"].as_u64().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?;
                let p = u32::try_from(slot["p"].as_u64().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?;
                if !n.is_power_of_two()
                    || n < 2
                    || r == 0
                    || p == 0
                    || p > 16
                    || n.checked_mul(r as u64)
                        .and_then(|v| v.checked_mul(128))
                        .is_none_or(|v| v > 256 * 1024 * 1024)
                    || n.checked_mul(r as u64)
                        .and_then(|v| v.checked_mul(p as u64))
                        .is_none_or(|v| v > 16 * 1024 * 1024)
                {
                    return Err(invalid());
                }
                let salt = unhex(slot["salt"].as_str().ok_or_else(invalid)?)?;
                let params = scrypt::Params::new(n.trailing_zeros() as u8, r, p, 32)
                    .map_err(|_| invalid())?;
                let mut key = Zeroizing::new([0u8; 32]);
                scrypt::scrypt(password.as_bytes(), &salt, &params, &mut *key)
                    .map_err(|_| invalid())?;
                let cipher = unhex(slot["key"].as_str().ok_or_else(invalid)?)?;
                match decrypt(key.as_slice(), &cipher, &slot["key_params"]) {
                    Ok(value) => {
                        master = Some(value);
                        break;
                    }
                    Err(CryptError::ImportPassword) => {}
                    Err(e) => return Err(e),
                }
            }
            let master = master.ok_or(CryptError::ImportPassword)?;
            let plain = decrypt(&master, &cipher, &file.header["params"])?;
            serde_json::from_slice::<AegisDb>(&plain).map_err(|_| invalid())?
        }
    };
    let mut result = OtpParse::default();
    for entry in db.entries {
        let label = if entry.issuer.is_empty() {
            entry.name.clone()
        } else {
            format!("{} · {}", entry.issuer, entry.name)
        };
        match make_draft(
            entry.issuer,
            entry.name,
            &entry.kind,
            &entry.info.secret,
            &entry.info.algo,
            entry.info.digits,
            entry.info.period,
        ) {
            Ok(d) => result.drafts.push(d),
            Err(reason) => result.skipped.push((label, reason)),
        }
    }
    Ok(result)
}
#[derive(Deserialize)]
struct TwoOtp {
    #[serde(default)]
    account: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    issuer: String,
    #[serde(default)]
    algorithm: String,
    #[serde(default = "six")]
    digits: u32,
    #[serde(default = "thirty")]
    period: u32,
    #[serde(rename = "tokenType", default)]
    kind: String,
}
#[derive(Deserialize)]
struct TwoService {
    #[serde(default)]
    name: String,
    #[serde(default, deserialize_with = "secret")]
    secret: Zeroizing<String>,
    otp: TwoOtp,
}
#[derive(Deserialize)]
struct TwoFile {
    #[serde(default)]
    services: Vec<TwoService>,
    #[serde(rename = "servicesEncrypted", default)]
    encrypted: Option<serde_json::Value>,
}
pub fn parse_2fas(bytes: &[u8]) -> Result<OtpParse> {
    let file: TwoFile = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if file.encrypted.is_some_and(|v| !v.is_null()) {
        return Err(CryptError::ImportFormat(
            "Экспортируйте 2FAS без пароля".into(),
        ));
    }
    if file.services.is_empty()
        && !std::str::from_utf8(bytes)
            .unwrap_or("")
            .contains("\"services\"")
    {
        return Err(invalid());
    }
    let mut result = OtpParse::default();
    for s in file.services {
        let issuer = if s.otp.issuer.is_empty() {
            s.name.clone()
        } else {
            s.otp.issuer
        };
        let account = if s.otp.account.is_empty() {
            s.otp.label
        } else {
            s.otp.account
        };
        let label = format!("{issuer} · {account}");
        let kind = if s.otp.kind.is_empty() {
            "TOTP"
        } else {
            &s.otp.kind
        };
        match make_draft(
            issuer,
            account,
            kind,
            &s.secret,
            &s.otp.algorithm,
            s.otp.digits,
            s.otp.period,
        ) {
            Ok(d) => result.drafts.push(d),
            Err(reason) => result.skipped.push((label, reason)),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_aegis_twofas_and_hotp() {
        assert_eq!(
            parse_aegis(
                include_bytes!("../../tests/fixtures/otp/aegis_plain.json"),
                None
            )
            .unwrap()
            .drafts
            .len(),
            1
        );
        assert_eq!(
            parse_2fas(include_bytes!("../../tests/fixtures/otp/twofas.2fas"))
                .unwrap()
                .drafts[0]
                .account,
            "alice"
        );
        let p = parse_otp_uris(&["otpauth://hotp/A?secret=GEZDGNBVGY3TQOJQ".into()]);
        assert_eq!(p.skipped[0].1, OtpSkip::Hotp);
        assert!(matches!(
            parse_2fas(br#"{"servicesEncrypted":"encrypted"}"#),
            Err(CryptError::ImportFormat(_))
        ));
    }
    #[test]
    fn attaches_only_to_one_login_without_totp() {
        let d = tempfile::tempdir().unwrap();
        let mut v = crate::Vault::create(d.path(), &crate::SeedPhrase::generate()).unwrap();
        let id = v
            .create_record(crate::NewRecord {
                title: "GitHub".into(),
                secret_kind: Some(crate::SecretKind::Login),
                ..Default::default()
            })
            .unwrap();
        v.set_secret_value(&id, "url", Some("https://github.com"))
            .unwrap();
        v.set_secret_value(&id, "username", Some("alice")).unwrap();
        let parse = || {
            parse_aegis(
                include_bytes!("../../tests/fixtures/otp/aegis_plain.json"),
                None,
            )
            .unwrap()
            .drafts
        };
        assert!(
            matches!(v.preview_otp(&parse())[0].target,OtpTarget::Attach {record,..} if record==id)
        );
        let mut repeated = parse();
        repeated.extend(parse());
        assert!(matches!(v.preview_otp(&repeated)[1].target, OtpTarget::New));
        let duplicate = v
            .create_record(crate::NewRecord {
                title: "GitHub duplicate".into(),
                secret_kind: Some(crate::SecretKind::Login),
                ..Default::default()
            })
            .unwrap();
        v.set_secret_value(&duplicate, "url", Some("https://github.com"))
            .unwrap();
        v.set_secret_value(&duplicate, "username", Some("alice"))
            .unwrap();
        assert!(matches!(v.preview_otp(&parse())[0].target, OtpTarget::New));
        v.delete(&duplicate).unwrap();
        let report = v.import_otp(parse(), "2FA").unwrap();
        assert_eq!(report.attached, 1);
        assert!(v.secret_value(&id, "totp_seed").unwrap().is_some());
        assert!(matches!(v.preview_otp(&parse())[0].target, OtpTarget::New));
        assert_eq!(v.import_otp(parse(), "2FA").unwrap().created, 1);
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    fn pb(n: u8, data: &[u8]) -> Vec<u8> {
        let mut v = vec![n * 8 + 2];
        varint(data.len() as u64, &mut v);
        v.extend(data);
        v
    }
    fn varint(mut n: u64, v: &mut Vec<u8>) {
        while n >= 128 {
            v.push((n as u8 & 127) | 128);
            n >>= 7;
        }
        v.push(n as u8);
    }
    fn migration(index: u8) -> String {
        let mut otp = pb(1, b"synthetic-secret");
        otp.extend(pb(
            2,
            if index == 0 {
                b"GitHub:alice"
            } else {
                b"GitLab:bob"
            },
        ));
        otp.extend([32, 1, 40, 1, 48, 2]);
        let mut p = pb(1, &otp);
        p.extend([16, 1, 24, 2, 32, index, 40, 42]);
        format!(
            "otpauth-migration://offline?data={}",
            data_encoding::BASE64.encode(&p)
        )
    }
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/otp")
    }
    fn qr(text: &str) -> Vec<u8> {
        let code = qrcode::QrCode::new(text).unwrap();
        let width = code.width();
        let colors = code.to_colors();
        let img = image::GrayImage::from_fn(
            ((width + 8) * 6) as u32,
            ((width + 8) * 6) as u32,
            |x, y| {
                let x = x as usize / 6;
                let y = y as usize / 6;
                image::Luma([
                    if x >= 4
                        && y >= 4
                        && x < width + 4
                        && y < width + 4
                        && colors[(y - 4) * width + x - 4] == qrcode::Color::Dark
                    {
                        0
                    } else {
                        255
                    },
                ])
            },
        );
        let mut output = std::io::Cursor::new(vec![]);
        img.write_to(&mut output, image::ImageFormat::Png).unwrap();
        output.into_inner()
    }
    #[test]
    #[ignore = "regenerate synthetic fixtures explicitly"]
    fn generate_otp_fixtures() {
        let root = root();
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..2 {
            std::fs::write(
                root.join(format!("ga_batch{}.png", i + 1)),
                qr(&migration(i)),
            )
            .unwrap();
        }
        std::fs::write(
            root.join("otpauth.png"),
            qr("otpauth://totp/GitHub:alice?secret=GEZDGNBVGY3TQOJQ&issuer=GitHub"),
        )
        .unwrap();
        let master = [7u8; 32];
        let salt = [9u8; 32];
        let mut derived = [0; 32];
        scrypt::scrypt(
            b"synthetic-password",
            &salt,
            &scrypt::Params::new(10, 8, 1, 32).unwrap(),
            &mut derived,
        )
        .unwrap();
        let encrypt = |key: &[u8], nonce: [u8; 12], plain: &[u8]| {
            let c = Aes256Gcm::new_from_slice(key)
                .unwrap()
                .encrypt((&nonce).into(), plain)
                .unwrap();
            (
                hex(&c[..c.len() - 16]),
                serde_json::json!({"nonce":hex(&nonce),"tag":hex(&c[c.len()-16..])}),
            )
        };
        let (key, key_params) = encrypt(&derived, [1; 12], &master);
        let db=br#"{"version":3,"entries":[{"type":"totp","name":"alice","issuer":"GitHub","info":{"secret":"GEZDGNBVGY3TQOJQ","algo":"SHA1","digits":6,"period":30}}]}"#;
        let (cipher, params) = encrypt(&master, [2; 12], db);
        let ciphertext = unhex(&cipher).unwrap();
        let file = serde_json::json!({"version":1,"header":{"slots":[{"type":1,"n":1024,"r":8,"p":1,"salt":hex(&salt),"key":key,"key_params":key_params}],"params":params},"db":data_encoding::BASE64.encode(&ciphertext)});
        std::fs::write(
            root.join("aegis_encrypted.json"),
            serde_json::to_vec(&file).unwrap(),
        )
        .unwrap();
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    #[test]
    fn qr_batches_and_missing_batch() {
        let images =
            ["ga_batch1.png", "ga_batch2.png"].map(|f| std::fs::read(root().join(f)).unwrap());
        let uris = decode_qr_images(&images).unwrap();
        assert_eq!(uris.len(), 2);
        let parsed = parse_otp_uris(&uris);
        assert_eq!(parsed.drafts.len(), 2);
        assert!(parsed.missing_batches.is_empty());
        assert_eq!(parse_otp_uris(&uris[..1]).missing_batches, vec![(2, 2)]);
        assert_eq!(
            parse_otp_uris(
                &decode_qr_images(&[std::fs::read(root().join("otpauth.png")).unwrap()]).unwrap()
            )
            .drafts
            .len(),
            1
        );
    }
    #[test]
    fn encrypted_aegis_correct_wrong_and_missing_password() {
        let data = std::fs::read(root().join("aegis_encrypted.json")).unwrap();
        assert_eq!(
            parse_aegis(&data, Some("synthetic-password"))
                .unwrap()
                .drafts
                .len(),
            1
        );
        assert!(matches!(
            parse_aegis(&data, None),
            Err(CryptError::NeedsPassword)
        ));
        assert!(matches!(
            parse_aegis(&data, Some("wrong")),
            Err(CryptError::ImportPassword)
        ));
    }
}

#[cfg(test)]
mod service_identity_tests {
    use super::*;
    #[test]
    fn personal_github_does_not_attach_to_enterprise_subdomain() {
        let d = tempfile::tempdir().unwrap();
        let mut v = crate::Vault::create(d.path(), &crate::SeedPhrase::generate()).unwrap();
        let id = v
            .create_record(crate::NewRecord {
                title: "Enterprise".into(),
                secret_kind: Some(crate::SecretKind::Login),
                ..Default::default()
            })
            .unwrap();
        v.set_secret_value(&id, "username", Some("alice")).unwrap();
        v.set_secret_value(&id, "url", Some("https://github.company.com"))
            .unwrap();
        let parse = || {
            parse_aegis(
                include_bytes!("../../tests/fixtures/otp/aegis_plain.json"),
                None,
            )
            .unwrap()
            .drafts
        };
        assert!(matches!(v.preview_otp(&parse())[0].target, OtpTarget::New));
        v.set_secret_value(&id, "url", Some("https://login.github.com"))
            .unwrap();
        assert!(matches!(
            v.preview_otp(&parse())[0].target,
            OtpTarget::Attach { .. }
        ));
    }
}
