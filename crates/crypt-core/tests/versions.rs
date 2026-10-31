#![cfg(feature = "fs")]
use crypt_core::*;

fn vault() -> (tempfile::TempDir, SeedPhrase, Vault) {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let v = Vault::create(dir.path(), &seed).unwrap();
    (dir, seed, v)
}
fn new(title: &str) -> NewRecord {
    NewRecord {
        title: title.into(),
        ..Default::default()
    }
}
fn kinds(v: &Vault, id: &RecordId) -> Vec<String> {
    v.record_versions(id)
        .unwrap()
        .into_iter()
        .map(|x| x.kind)
        .collect()
}

#[test]
fn versions_are_labelled_grouped_and_newest_first() {
    let (_d, _s, mut v) = vault();
    let folder = v.create_folder("Папка", None).unwrap();
    let id = v
        .create_record(NewRecord {
            folder: Some(folder),
            ..new("Черновик")
        })
        .unwrap();
    assert_eq!(kinds(&v, &id), ["create"]);
    v.set_title(&id, "План").unwrap();
    v.set_body(&id, "раз").unwrap();
    v.set_body(&id, "раз два").unwrap();
    v.add_tag(&id, "работа").unwrap();
    assert_eq!(kinds(&v, &id), ["tags", "body", "title", "create"]);
    let all = v.record_versions(&id).unwrap();
    assert!(all.iter().all(|x| x.this_device && x.at > 0));
    assert!(all.windows(2).all(|w| w[0].at >= w[1].at));
}

#[test]
fn old_version_shows_old_content() {
    let (_d, _s, mut v) = vault();
    let id = v.create_record(new("Старое")).unwrap();
    v.set_body(&id, "первый текст").unwrap();
    let first = v.record_versions(&id).unwrap()[0].id.clone();
    v.set_title(&id, "Новое").unwrap();
    v.set_body(&id, "второй текст").unwrap();
    v.add_tag(&id, "тег").unwrap();
    let old = v.record_at(&id, &first).unwrap();
    assert_eq!(old.title, "Старое");
    assert_eq!(old.body, "первый текст");
    assert!(old.tags.is_empty());
    // Current state is untouched by looking at history.
    assert_eq!(v.get(&id).unwrap().title, "Новое");
}

#[test]
fn revert_adds_a_version_and_keeps_history() {
    let (_d, _s, mut v) = vault();
    let mut t = starter_types().remove(0);
    let field = t.fields[0].id;
    t.fields.truncate(1);
    v.put_type(t.clone()).unwrap();
    let id = v
        .create_record(NewRecord {
            type_id: Some(t.id),
            ..new("A")
        })
        .unwrap();
    v.set_body(&id, "текст A").unwrap();
    v.set_field(&id, &field, Some(FieldValue::Text("1".into())))
        .unwrap();
    let target = v.record_versions(&id).unwrap()[1].id.clone(); // after body
    v.set_title(&id, "B").unwrap();
    v.set_body(&id, "текст B").unwrap();
    v.set_field(&id, &field, Some(FieldValue::Text("2".into())))
        .unwrap();
    v.add_tag(&id, "новый").unwrap();
    let before = v.record_versions(&id).unwrap().len();
    v.revert_record(&id, &target).unwrap();
    let now = v.get(&id).unwrap();
    assert_eq!(now.title, "A");
    assert_eq!(now.body, "текст A");
    assert!(!now.fields.contains_key(&field));
    assert!(now.tags.is_empty());
    let versions = v.record_versions(&id).unwrap();
    assert_eq!(versions.len(), before + 1);
    assert_eq!(versions[0].kind, "revert");
    // The reverted-over state is still reachable.
    assert_eq!(v.record_at(&id, &versions[1].id).unwrap().title, "B");
}

#[test]
fn revert_never_touches_secret_values() {
    let (_d, _s, mut v) = vault();
    let id = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Login),
            ..new("Почта")
        })
        .unwrap();
    let start = v.record_versions(&id).unwrap()[0].id.clone();
    v.set_secret_value(&id, "password", Some("p1")).unwrap();
    v.set_title(&id, "Почта 2").unwrap();
    v.revert_record(&id, &start).unwrap();
    assert_eq!(v.get(&id).unwrap().title, "Почта");
    assert_eq!(
        v.secret_value(&id, "password")
            .unwrap()
            .as_deref()
            .map(String::as_str),
        Some("p1")
    );
    assert_eq!(kinds(&v, &id)[1..], ["title", "secret", "create"]);
}

#[test]
fn foreign_or_garbage_versions_are_rejected() {
    let (_d, _s, mut v) = vault();
    let a = v.create_record(new("a")).unwrap();
    let b = v.create_record(new("b")).unwrap();
    v.set_body(&b, "x".repeat(50).as_str()).unwrap();
    let late = v.record_versions(&b).unwrap()[0].id.clone();
    for bad in ["", "abc", "1:2:3", "18446744073709551615:0", &late] {
        assert!(v.record_at(&a, bad).is_err(), "{bad}");
        assert!(v.revert_record(&a, bad).is_err(), "{bad}");
    }
}

#[test]
fn history_survives_reopen() {
    let (d, seed, mut v) = vault();
    let id = v.create_record(new("x")).unwrap();
    v.set_body(&id, "y").unwrap();
    let before = v.record_versions(&id).unwrap();
    drop(v);
    let v = Vault::open_with_seed(d.path(), &seed).unwrap();
    assert_eq!(v.record_versions(&id).unwrap(), before);
}

#[test]
fn snapshot_restore_brings_back_contents_and_trash_state() {
    let (d, seed, mut v) = vault();
    let edited = v.create_record(new("Редактируемая")).unwrap();
    v.set_body(&edited, "до").unwrap();
    let trashed_later = v.create_record(new("Удалённая позже")).unwrap();
    let was_trashed = v.create_record(new("Была в корзине")).unwrap();
    v.delete(&was_trashed).unwrap();
    let purged = v.create_record(new("Стёртая")).unwrap();
    let secret = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Token),
            ..new("Токен")
        })
        .unwrap();
    let snap = v.create_snapshot("  Перед чисткой ").unwrap();
    assert_eq!(snap.name, "Перед чисткой");
    assert!(v.create_snapshot("   ").is_err());

    v.set_body(&edited, "после").unwrap();
    v.delete(&trashed_later).unwrap();
    v.restore(&was_trashed).unwrap();
    v.purge(&purged).unwrap();
    let created_after = v.create_record(new("Новая")).unwrap();
    let secret_after = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Token),
            ..new("Новый токен")
        })
        .unwrap();
    v.set_title(&secret, "Токен 2").unwrap();
    drop(v);

    let mut v = Vault::open_with_seed(d.path(), &seed).unwrap();
    assert_eq!(v.snapshots().len(), 1);
    let report = v.restore_snapshot(&snap.id).unwrap();
    assert_eq!(report.missing, 1);
    assert_eq!(report.trashed, 1);
    assert_eq!(v.get(&edited).unwrap().body, "до");
    assert!(v.get(&trashed_later).unwrap().deleted_at.is_none());
    assert!(v.get(&was_trashed).unwrap().deleted_at.is_some());
    assert!(v.get(&created_after).unwrap().deleted_at.is_some());
    // Secrets are never trashed (that would purge them); their titles still revert.
    assert!(v.get(&secret_after).is_ok());
    assert_eq!(v.get(&secret).unwrap().title, "Токен");
    v.delete_snapshot(&snap.id).unwrap();
    assert!(v.snapshots().is_empty());
    assert!(v.restore_snapshot(&snap.id).is_err());
}

#[test]
fn snapshot_restore_removes_untouched_imported_secrets() {
    use crypt_core::import_csv::LoginDraft;
    let (_d, _s, mut v) = vault();
    let snap = v.create_snapshot("Перед импортом").unwrap();
    let draft = |t: &str| LoginDraft {
        title: t.into(),
        url: Some(format!("https://{t}")),
        username: format!("user-{t}"),
        password: zeroize::Zeroizing::new("pw".into()),
        note: None,
        totp: None,
    };
    v.import_logins(vec![draft("a.com"), draft("b.com")], "Импорт")
        .unwrap();
    let manual = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Token),
            ..new("Ручной")
        })
        .unwrap();
    let list = v.list(&ListFilter::default());
    let edited = list.iter().find(|r| r.title == "b.com").unwrap().id;
    assert_eq!(
        list.iter()
            .find(|r| r.title == "a.com")
            .unwrap()
            .username
            .as_deref(),
        Some("user-a.com")
    );
    v.set_title(&edited, "b.com (мой)").unwrap();

    let report = v.restore_snapshot(&snap.id).unwrap();
    assert_eq!(report.removed, 1);
    let titles: Vec<_> = v
        .list(&ListFilter::default())
        .into_iter()
        .map(|r| r.title)
        .collect();
    assert!(!titles.contains(&"a.com".to_string()));
    assert!(titles.contains(&"b.com (мой)".to_string()));
    assert!(v.get(&manual).is_ok());
}

#[test]
fn delete_many_trashes_notes_and_purges_secrets() {
    let (_d, _s, mut v) = vault();
    let note = v.create_record(new("Заметка")).unwrap();
    let secret = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Token),
            ..new("Токен")
        })
        .unwrap();
    assert_eq!(v.delete_many(&[note, secret]).unwrap(), (1, 1));
    assert!(v.get(&note).unwrap().deleted_at.is_some());
    assert!(v.get(&secret).is_err());
}

#[test]
fn vault_name_defaults_and_survives_reopen() {
    let (d, seed, mut v) = vault();
    assert_eq!(v.name(), "Личное");
    assert!(v.rename("   ").is_err());
    v.rename("  Работа ").unwrap();
    drop(v);
    let v = Vault::open_with_seed(d.path(), &seed).unwrap();
    assert_eq!(v.name(), "Работа");
}
