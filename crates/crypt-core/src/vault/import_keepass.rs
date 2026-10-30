//! KeePass (.kdbx / .kdb) → Login secrets.
use super::Vault;
use crate::model::record::{NewRecord, normalize_tag};
use crate::{CryptError, RecordId, Result, SecretKind};
use keepass::{Database, DatabaseKey, db::DatabaseOpenError, error::DatabaseKeyError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct KeepassReport {
    /// Entries imported as Login secrets.
    pub imported: usize,
    /// Protected custom fields imported as separate Note secrets.
    pub extra_secrets: usize,
    pub skipped_recycled: usize,
    pub skipped_attachments: usize,
    /// "otp" values that are not valid TOTP and were dropped.
    pub invalid_totp: usize,
}

/// Fields mapped onto the Login kind or ignored on purpose.
const KNOWN: [&str; 8] = [
    "Title",
    "UserName",
    "Password",
    "URL",
    "Notes",
    "otp",
    "TOTP Seed",
    "TOTP Settings",
];

fn open(data: &[u8], password: Option<&str>, keyfile: Option<&[u8]>) -> Result<Database> {
    let mut key = DatabaseKey::new();
    if let Some(p) = password.filter(|p| !p.is_empty()) {
        key = key.with_password(p);
    }
    if let Some(mut k) = keyfile {
        key = key
            .with_keyfile(&mut k)
            .map_err(|_| CryptError::ImportFormat("Не удалось прочитать ключ-файл".into()))?;
    }
    if key.is_empty() {
        return Err(CryptError::WrongImportKey);
    }
    Database::parse(data, key).map_err(|e| match e {
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey) => CryptError::WrongImportKey,
        DatabaseOpenError::Key(_) => CryptError::WrongImportKey,
        _ => CryptError::ImportFormat("Файл не похож на базу KeePass или повреждён".into()),
    })
}

impl Vault {
    /// Import every entry outside the recycle bin as a Login secret. Group names and
    /// entry tags become tags; plain custom fields go to the body, protected ones become
    /// separate Note secrets. Parsing happens before anything is written.
    pub fn import_keepass(
        &mut self,
        data: &[u8],
        password: Option<&str>,
        keyfile: Option<&[u8]>,
    ) -> Result<KeepassReport> {
        let db = open(data, password, keyfile)?;
        let bin = db.meta.recyclebin_uuid;
        // group uuid → (parent uuid, name); the root has no parent.
        let tree: HashMap<_, _> = db
            .iter_all_groups()
            .map(|g| {
                (
                    g.id().uuid(),
                    (g.parent().map(|p| p.id().uuid()), g.name.clone()),
                )
            })
            .collect();
        let mut report = KeepassReport::default();
        for entry in db.iter_all_entries() {
            // Group path from the entry up to (not including) the root.
            let mut groups = Vec::new();
            let mut recycled = false;
            let mut cursor = Some(entry.parent().id().uuid());
            let mut guard = 0;
            while let Some(id) = cursor.filter(|_| guard < 256) {
                guard += 1;
                recycled |= Some(id) == bin;
                let Some((parent, name)) = tree.get(&id) else {
                    break;
                };
                if parent.is_some() {
                    groups.push(name.clone());
                }
                cursor = *parent;
            }
            if recycled {
                report.skipped_recycled += 1;
                continue;
            }
            report.skipped_attachments += entry.attachments().count();

            let title = entry
                .get_title()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .unwrap_or("Без названия")
                .to_string();
            let mut tags = vec!["keepass".to_string()];
            tags.extend(groups.iter().rev().cloned());
            tags.extend(entry.tags.iter().cloned());
            let tags: Vec<String> = tags.iter().filter_map(|t| normalize_tag(t).ok()).collect();

            let totp = entry
                .get_raw_otp_value()
                .or_else(|| entry.get("TOTP Seed"))
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let totp = match totp {
                Some(t) if crate::totp::parse_totp(t).is_ok() => {
                    Some(Zeroizing::new(t.to_string()))
                }
                Some(_) => {
                    report.invalid_totp += 1;
                    None
                }
                None => None,
            };

            let mut body = entry.get("Notes").unwrap_or_default().trim().to_string();
            let mut extras: Vec<(String, Zeroizing<String>)> = Vec::new();
            let mut custom: Vec<(&String, &keepass::db::Value<String>)> = entry
                .fields
                .iter()
                .filter(|(k, _)| !KNOWN.contains(&k.as_str()))
                .collect();
            custom.sort_by(|a, b| a.0.cmp(b.0));
            let mut plain = String::new();
            for (k, v) in custom {
                if v.get().is_empty() {
                    continue;
                }
                if v.is_protected() {
                    extras.push((k.clone(), Zeroizing::new(v.get().clone())));
                } else {
                    plain.push_str(&format!("- **{k}**: {}\n", v.get()));
                }
            }
            if !plain.is_empty() {
                if !body.is_empty() {
                    body.push_str("\n\n");
                }
                body.push_str("## Поля KeePass\n\n");
                body.push_str(plain.trim_end());
            }

            let values: Vec<(&str, Option<Zeroizing<String>>)> = vec![
                (
                    "username",
                    entry.get_username().map(|s| Zeroizing::new(s.to_string())),
                ),
                (
                    "password",
                    entry.get_password().map(|s| Zeroizing::new(s.to_string())),
                ),
                (
                    "url",
                    entry.get_url().map(|s| Zeroizing::new(s.to_string())),
                ),
                ("totp_seed", totp),
            ];
            self.insert_secret(&title, SecretKind::Login, &body, &tags, &values)?;
            report.imported += 1;
            for (name, value) in extras {
                self.insert_secret(
                    &format!("{title} · {name}"),
                    SecretKind::Note,
                    "",
                    &tags,
                    &[("note", Some(value))],
                )?;
                report.extra_secrets += 1;
            }
        }
        Ok(report)
    }

    fn insert_secret(
        &mut self,
        title: &str,
        kind: SecretKind,
        body: &str,
        tags: &[String],
        values: &[(&str, Option<Zeroizing<String>>)],
    ) -> Result<RecordId> {
        let new = NewRecord {
            title: title.into(),
            secret_kind: Some(kind),
            ..Default::default()
        };
        self.insert_built(RecordId::new(), new, "import", |r| {
            if !body.is_empty() {
                r.set_body(body)?;
            }
            for t in tags {
                r.add_tag(t)?;
            }
            for (name, value) in values {
                if let Some(v) = value.as_ref().filter(|v| !v.is_empty()) {
                    r.set_secret_value(name, Some(v))?;
                }
            }
            Ok(())
        })
    }
}
