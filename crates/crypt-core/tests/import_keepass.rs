#![cfg(feature = "fs")]
use crypt_core::*;
use keepass::{Database, DatabaseKey, config::KdfConfig};

const PW: &str = "correct horse";

/// A small KDBX4 file with groups, a recycle bin, TOTP, custom fields and an attachment.
fn kdbx() -> Vec<u8> {
    let mut db = Database::new();
    db.config.kdf_config = KdfConfig::Aes { rounds: 10 };
    let mut root = db.root_mut();
    root.add_entry().edit(|e| {
        e.set_unprotected("Title", "Почта");
        e.set_unprotected("UserName", "anton@example.com");
        e.set_protected("Password", "p@ss");
        e.set_unprotected("URL", "https://mail.example.com");
        e.set_unprotected("Notes", "Резервные коды в сейфе");
        e.set_unprotected(
            "otp",
            "otpauth://totp/Mail:anton?secret=GEZDGNBVGY3TQOJQ&issuer=Mail",
        );
        e.set_unprotected("Отдел", "Бухгалтерия");
        e.set_protected("PIN карты", "4321");
        e.tags.push("Важное".into());
    });
    let mut work = root.add_group();
    work.name = "Работа".into();
    let mut servers = work.add_group();
    servers.name = "Серверы".into();
    servers.add_entry().edit(|e| {
        e.set_unprotected("Title", "VPN");
        e.set_protected("Password", "vpn-secret");
        e.set_unprotected("otp", "otpauth://hotp/VPN?secret=GEZDGNBV&counter=1");
        e.add_attachment("cert.pem", keepass::db::Value::unprotected(b"x".to_vec()));
    });
    let mut root = db.root_mut();
    let mut bin = root.add_group();
    bin.name = "Корзина".into();
    let bin_id = bin.id().uuid();
    bin.add_entry()
        .edit(|e| e.set_unprotected("Title", "Удалённое"));
    db.meta.recyclebin_uuid = Some(bin_id);
    let mut out = Vec::new();
    db.save(&mut out, DatabaseKey::new().with_password(PW))
        .unwrap();
    out
}

fn secret(v: &Vault, id: &RecordId, name: &str) -> Option<String> {
    v.secret_value(id, name).unwrap().map(|s| s.to_string())
}

#[test]
fn keepass_entries_become_login_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(dir.path(), &SeedPhrase::generate()).unwrap();
    let report = v.import_keepass(&kdbx(), Some(PW), None).unwrap();
    assert_eq!(report.imported, 2);
    assert_eq!(report.extra_secrets, 1);
    assert_eq!(report.skipped_recycled, 1);
    assert_eq!(report.skipped_attachments, 1);
    assert_eq!(report.invalid_totp, 1);

    let all = v.list(&ListFilter {
        secrets: Some(true),
        ..Default::default()
    });
    let by = |t: &str| all.iter().find(|r| r.title == t).unwrap().id;
    let mail = by("Почта");
    let r = v.get(&mail).unwrap();
    assert_eq!(r.secret_kind, Some(SecretKind::Login));
    assert_eq!(r.tags, vec!["keepass", "важное"]);
    assert!(r.body.contains("Резервные коды в сейфе"));
    assert!(r.body.contains("Отдел") && r.body.contains("Бухгалтерия"));
    assert!(
        !r.body.contains("4321"),
        "protected custom fields never land in the body"
    );
    assert_eq!(
        secret(&v, &mail, "username").as_deref(),
        Some("anton@example.com")
    );
    assert_eq!(secret(&v, &mail, "password").as_deref(), Some("p@ss"));
    assert_eq!(
        secret(&v, &mail, "url").as_deref(),
        Some("https://mail.example.com")
    );
    assert!(
        secret(&v, &mail, "totp_seed")
            .unwrap()
            .starts_with("otpauth://")
    );

    let pin = by("Почта · PIN карты");
    assert_eq!(v.get(&pin).unwrap().secret_kind, Some(SecretKind::Note));
    assert_eq!(secret(&v, &pin, "note").as_deref(), Some("4321"));

    let vpn = by("VPN");
    assert_eq!(
        v.get(&vpn).unwrap().tags,
        vec!["keepass", "работа", "серверы"]
    );
    assert_eq!(secret(&v, &vpn, "totp_seed"), None);
    assert!(!all.iter().any(|r| r.title == "Удалённое"));
    // One labelled version per imported record, not one per field.
    assert_eq!(
        v.record_versions(&mail)
            .unwrap()
            .iter()
            .map(|x| x.kind.as_str())
            .collect::<Vec<_>>(),
        ["import"]
    );
}

#[test]
fn wrong_keepass_password_imports_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(dir.path(), &SeedPhrase::generate()).unwrap();
    let err = v.import_keepass(&kdbx(), Some("nope"), None).unwrap_err();
    assert!(matches!(err, CryptError::WrongImportKey), "{err:?}");
    assert!(matches!(
        v.import_keepass(b"not a database", Some(PW), None)
            .unwrap_err(),
        CryptError::ImportFormat(_)
    ));
    assert!(v.list(&ListFilter::default()).is_empty());
}
