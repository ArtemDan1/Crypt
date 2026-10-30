#![cfg(feature = "fs")]
use crypt_core::*;
#[test]
fn full_user_journey() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut vault = Vault::create(dir.path(), &seed).unwrap();
    let cost = FieldId::new();
    let renewal = FieldId::new();
    let access = FieldId::new();
    let t = TypeDef {
        id: TypeId::new(),
        name: "Сервис".into(),
        icon: "cloud".into(),
        fields: vec![
            FieldDef {
                id: cost,
                name: "Стоимость".into(),
                kind: FieldKind::Number,
            },
            FieldDef {
                id: renewal,
                name: "Продление".into(),
                kind: FieldKind::Date,
            },
            FieldDef {
                id: access,
                name: "Доступ".into(),
                kind: FieldKind::SecretLink { multiple: false },
            },
        ],
    };
    vault.put_type(t.clone()).unwrap();
    let login = vault
        .create_record(NewRecord {
            title: "Яндекс ID".into(),
            secret_kind: Some(SecretKind::Login),
            ..Default::default()
        })
        .unwrap();
    vault
        .set_secret_value(&login, "password", Some("v7#Qm!r2Lx9"))
        .unwrap();
    let budget = vault
        .create_record(NewRecord {
            title: "Бюджет 2026".into(),
            ..Default::default()
        })
        .unwrap();
    let service = vault
        .create_record(NewRecord {
            title: "Яндекс 360".into(),
            type_id: Some(t.id),
            ..Default::default()
        })
        .unwrap();
    vault
        .set_field(&service, &cost, Some(FieldValue::Number(399.0)))
        .unwrap();
    vault
        .set_field(
            &service,
            &renewal,
            Some(FieldValue::Date("2026-11-28".into())),
        )
        .unwrap();
    vault
        .set_field(&service, &access, Some(FieldValue::Links(vec![login])))
        .unwrap();
    vault
        .set_body(&service, &format!("Подписка [[{budget}|Бюджет 2026]]"))
        .unwrap();
    let key = vault.data_key().clone();
    vault.lock();
    let mut vault = Vault::open_with_key(dir.path(), key).unwrap();
    let record = vault.get(&service).unwrap();
    assert_eq!(record.fields[&cost], FieldValue::Number(399.0));
    assert_eq!(
        record.fields[&renewal],
        FieldValue::Date("2026-11-28".into())
    );
    assert_eq!(record.fields[&access], FieldValue::Links(vec![login]));
    assert_eq!(vault.backlinks(&login)[0].id, service);
    assert_eq!(vault.search("бюджет", 10)[0].id, budget);
    assert_eq!(
        &*vault.secret_value(&login, "password").unwrap().unwrap(),
        "v7#Qm!r2Lx9"
    );
    vault
        .set_secret_value(&login, "password", Some("Replacement-password"))
        .unwrap();
    assert_eq!(vault.secret_history(&login, "password").unwrap().len(), 1);
    vault.delete(&login).unwrap();
    assert_eq!(
        vault.get(&service).unwrap().fields[&access],
        FieldValue::Links(vec![])
    );
    assert!(vault.get(&login).is_err());
    assert!(vault.search("Яндекс ID", 10).is_empty());
    assert!(vault.search("v7#Qm", 10).is_empty());
    assert!(!dir.path().join(format!("objects/{login}.bin")).exists());
}
