use crypt_core::index::{Index, links::parse_wikilinks};
use crypt_core::model::record::RecordDoc;
use crypt_core::*;
#[test]
fn parses_links_with_and_without_alias() {
    let id = RecordId::new();
    let links = parse_wikilinks(&format!(
        "[[{id}]] и [[{id}|Аня]] и [[не-id]] [[ZZZZZZZZZZZZZZZZZZZZZZZZZZ]]"
    ));
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target, id);
    assert_eq!(links[1].alias.as_deref(), Some("Аня"));
}
#[test]
fn index_case_ranking_unicode_snippets_and_update() {
    let a = RecordId::new();
    let b = RecordId::new();
    let mut r = RecordDoc::new(1, a, "Бюджет Яндекс 360", 100);
    r.set_body(&format!("{}Яндекс{}", "д".repeat(90), "х".repeat(90)))
        .unwrap();
    let mut other = RecordDoc::new(1, b, "Other", 9999999999999);
    other.set_body("Бюджет в теле").unwrap();
    let mut index = Index::default();
    index.upsert(&r);
    index.upsert(&other);
    assert_eq!(index.search("бюджет", 10)[0].id, a);
    for query in ["яндекс", "ЯНДЕКС"] {
        let hits = index.search(query, 10);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.chars().count() <= 80);
        assert!(hits[0].snippet.contains("Яндекс"));
    }
    r.add_tag("#Инфра").unwrap();
    index.upsert(&r);
    assert_eq!(index.search("ИНФРА", 1)[0].id, a);
    r.set_title("renamed").unwrap();
    r.set_body("").unwrap();
    index.upsert(&r);
    assert!(index.search("Яндекс", 10).is_empty());
    assert!(index.search("x", 0).is_empty());
}
#[cfg(feature = "fs")]
#[test]
fn vault_search_backlinks_secrets_trash_and_purge() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let c = v
        .create_record(NewRecord {
            title: "Login".into(),
            secret_kind: Some(SecretKind::Login),
            ..Default::default()
        })
        .unwrap();
    v.set_secret_value(&c, "password", Some("v7#Qm!r2Lx9"))
        .unwrap();
    assert!(v.search("v7#Qm", 10).is_empty());
    let a = v
        .create_record(NewRecord {
            title: "A".into(),
            ..Default::default()
        })
        .unwrap();
    v.set_body(&a, &format!("[[{c}|login]]")).unwrap();
    let t = starter_types().remove(0);
    let field = t.fields[6].id;
    v.put_type(t.clone()).unwrap();
    let b = v
        .create_record(NewRecord {
            title: "B".into(),
            type_id: Some(t.id),
            ..Default::default()
        })
        .unwrap();
    v.set_field(&b, &field, Some(FieldValue::Links(vec![c])))
        .unwrap();
    assert_eq!(v.backlinks(&c).len(), 2);
    v.delete(&a).unwrap();
    assert_eq!(v.backlinks(&c)[0].id, b);
    assert!(v.search("A", 10).iter().all(|h| h.id != a));
    v.restore(&a).unwrap();
    assert_eq!(v.backlinks(&c).len(), 2);
    v.set_body(&a, "").unwrap();
    assert_eq!(v.backlinks(&c).len(), 1);
    v.lock();
    let mut v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert_eq!(v.backlinks(&c)[0].id, b);
    assert!(v.search("v7#Qm", 10).is_empty());
    v.delete(&c).unwrap();
    assert!(v.backlinks(&c).is_empty());
    assert!(v.search("Login", 10).is_empty());
}
