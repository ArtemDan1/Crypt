use super::{
    Vault,
    import_csv::{LoginDraft, SkipReason, domain},
};
use crate::{NewRecord, Result, SecretKind};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
#[derive(Debug, Serialize)]
pub struct LoginPreview {
    pub new: usize,
    pub duplicates: usize,
    pub skipped: usize,
    pub sample: Vec<(String, String)>,
}
#[derive(Debug, Serialize)]
pub struct LoginImportReport {
    pub imported: usize,
    pub duplicates: usize,
    pub skipped: Vec<(usize, SkipReason)>,
}
fn key(url: &str, username: &str, password: &str) -> [u8; 32] {
    let mut hash = Sha256::new();
    for value in [domain(url).as_str(), username, password] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    hash.finalize().into()
}
impl Vault {
    fn login_keys(&self) -> HashSet<[u8; 32]> {
        self.records
            .values()
            .filter(|r| r.deleted_at().is_none() && r.secret_kind() == Some(SecretKind::Login))
            .filter_map(|r| {
                let password = r.secret_value("password").ok().flatten()?;
                let url = r.secret_value("url").ok().flatten().unwrap_or_default();
                let user = r
                    .secret_value("username")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                Some(key(&url, &user, &password))
            })
            .collect()
    }
    pub fn preview_logins(&self, drafts: &[LoginDraft]) -> LoginPreview {
        let mut seen = self.login_keys();
        let mut result = LoginPreview {
            new: 0,
            duplicates: 0,
            skipped: 0,
            sample: vec![],
        };
        for d in drafts {
            if seen.insert(key(
                d.url.as_deref().unwrap_or(""),
                &d.username,
                &d.password,
            )) {
                result.new += 1;
                if result.sample.len() < 5 {
                    result.sample.push((d.title.clone(), d.username.clone()));
                }
            } else {
                result.duplicates += 1;
            }
        }
        result
    }
    /// Earlier imports put secrets into a dated folder («Импорт из браузера · 31 окт»).
    /// Secrets live in «Секреты», so such a folder holding only secrets is dissolved:
    /// its secrets move to the root and the folder goes away. Returns how many were removed.
    pub fn tidy_import_folders(&mut self) -> Result<usize> {
        let folders = self.folders();
        let mut removed = 0;
        for f in folders
            .iter()
            .filter(|f| f.parent.is_none() && is_import_folder(&f.name))
        {
            if folders.iter().any(|c| c.parent == Some(f.id)) {
                continue;
            }
            let inside: Vec<(crate::RecordId, bool)> = self
                .records
                .iter()
                .filter(|(_, r)| r.folder() == Some(f.id))
                .map(|(id, r)| (*id, r.secret_kind().is_some()))
                .collect();
            // Empty or holding notes: the user may want it, leave it alone.
            if inside.is_empty() || inside.iter().any(|(_, secret)| !secret) {
                continue;
            }
            for (id, _) in inside {
                self.set_folder(&id, None)?;
            }
            self.delete_folder(&f.id)?;
            removed += 1;
        }
        Ok(removed)
    }
    /// Logins become secrets: they live in «Секреты», not in a folder.
    pub fn import_logins(&mut self, drafts: Vec<LoginDraft>) -> Result<LoginImportReport> {
        let mut seen = self.login_keys();
        let mut result = LoginImportReport {
            imported: 0,
            duplicates: 0,
            skipped: vec![],
        };
        for d in drafts {
            if !seen.insert(key(
                d.url.as_deref().unwrap_or(""),
                &d.username,
                &d.password,
            )) {
                result.duplicates += 1;
                continue;
            }
            self.insert_built(
                crate::RecordId::new(),
                NewRecord {
                    title: d.title,
                    secret_kind: Some(SecretKind::Login),
                    ..Default::default()
                },
                "import",
                |r| {
                    r.set_secret_value("password", Some(&d.password))?;
                    r.set_secret_value("username", Some(&d.username))?;
                    r.set_secret_value("url", d.url.as_deref())?;
                    if let Some(note) = d.note {
                        r.set_body(&note)?;
                    }
                    if let Some(totp) = d.totp {
                        r.set_secret_value("totp_seed", Some(&totp))?;
                    }
                    Ok(())
                },
            )?;
            result.imported += 1;
        }
        Ok(result)
    }
}

fn is_import_folder(name: &str) -> bool {
    name.starts_with("Импорт из браузера · ") || name.starts_with("Импорт 2FA · ")
}

#[cfg(test)]
mod tests {
    use super::super::import_csv::parse_browser_csv;
    use super::*;
    #[test]
    fn tidy_dissolves_only_secret_only_import_folders() {
        let d = tempfile::tempdir().unwrap();
        let mut v = Vault::create(d.path(), &crate::SeedPhrase::generate()).unwrap();
        let secret = |v: &mut Vault, folder| {
            v.create_record(NewRecord {
                title: "s".into(),
                secret_kind: Some(SecretKind::Login),
                folder: Some(folder),
                ..Default::default()
            })
            .unwrap()
        };
        let old = v.create_folder("Импорт из браузера · 31 окт", None).unwrap();
        let s1 = secret(&mut v, old);
        let mixed = v.create_folder("Импорт 2FA · 1 ноя", None).unwrap();
        secret(&mut v, mixed);
        v.create_record(NewRecord {
            title: "note".into(),
            folder: Some(mixed),
            ..Default::default()
        })
        .unwrap();
        let mine = v.create_folder("Пароли", None).unwrap();
        secret(&mut v, mine);
        v.create_folder("Импорт 2FA · 2 ноя", None).unwrap();

        assert_eq!(v.tidy_import_folders().unwrap(), 1);
        let names: Vec<_> = v.folders().into_iter().map(|f| f.name).collect();
        assert!(!names.contains(&"Импорт из браузера · 31 окт".to_string()));
        assert_eq!(names.len(), 3);
        assert_eq!(v.record(&s1).unwrap().folder(), None);
        assert_eq!(v.tidy_import_folders().unwrap(), 0);
    }
    #[test]
    fn duplicates_inside_file_and_existing_but_changed_password_is_new() {
        let d = tempfile::tempdir().unwrap();
        let mut v = Vault::create(d.path(), &crate::SeedPhrase::generate()).unwrap();
        let bytes=b"name,url,username,password,note\nA,https://www.example.com/a,u,p,\nB,https://EXAMPLE.com/b,u,p,\nC,https://example.com,u,different,";
        let parsed = parse_browser_csv(bytes).unwrap();
        let p = v.preview_logins(&parsed.logins);
        assert_eq!((p.new, p.duplicates), (2, 1));
        let r = v.import_logins(parsed.logins).unwrap();
        assert_eq!((r.imported, r.duplicates), (2, 1));
        assert!(v.folders().is_empty(), "secrets need no import folder");
        let p = v.preview_logins(&parse_browser_csv(bytes).unwrap().logins);
        assert_eq!((p.new, p.duplicates), (0, 3));
    }
}

use super::import_otp::{OtpDraft, OtpImportReport, OtpPreviewRow, OtpTarget, otp_uri};
impl Vault {
    fn otp_target(&self, d: &OtpDraft) -> OtpTarget {
        let issuer = d.issuer.trim().to_lowercase();
        if issuer.is_empty() {
            return OtpTarget::New;
        }
        let mut candidates = vec![];
        for r in self
            .records
            .values()
            .filter(|r| r.deleted_at().is_none() && r.secret_kind() == Some(SecretKind::Login))
        {
            let url = r.secret_value("url").ok().flatten().unwrap_or_default();
            let host = domain(&url);
            let labels: Vec<_> = host.split('.').collect();
            let service = if issuer.contains('.') {
                host == issuer || host.ends_with(&format!(".{issuer}"))
            } else if labels.len() >= 2 {
                // Brand names refer to the site's base domain, never an arbitrary subdomain.
                let suffix = labels[labels.len() - 1];
                let second = labels[labels.len() - 2];
                let base = if suffix.len() == 2
                    && ["co", "com", "net", "org", "gov", "ac", "edu"].contains(&second)
                {
                    labels
                        .get(labels.len().wrapping_sub(3))
                        .copied()
                        .unwrap_or("")
                } else {
                    second
                };
                base == issuer
            } else {
                host == issuer
            };
            let user = r
                .secret_value("username")
                .ok()
                .flatten()
                .unwrap_or_default();
            if service && (user.is_empty() || *user == d.account) {
                candidates.push(r);
            }
        }
        if candidates.len() != 1 {
            return OtpTarget::New;
        }
        let r = candidates[0];
        if r.secret_value("totp_seed")
            .ok()
            .flatten()
            .is_some_and(|s| !s.is_empty())
        {
            return OtpTarget::New;
        }
        OtpTarget::Attach {
            record: r.id(),
            title: r.summary().title,
        }
    }
    pub fn preview_otp(&self, drafts: &[OtpDraft]) -> Vec<OtpPreviewRow> {
        let mut used = HashSet::new();
        drafts
            .iter()
            .map(|d| {
                let target = match self.otp_target(d) {
                    OtpTarget::Attach { record, title } if used.insert(record) => {
                        OtpTarget::Attach { record, title }
                    }
                    _ => OtpTarget::New,
                };
                OtpPreviewRow {
                    issuer: d.issuer.clone(),
                    account: d.account.clone(),
                    target,
                }
            })
            .collect()
    }
    /// New 2FA entries are secrets too: no import folder.
    pub fn import_otp(&mut self, drafts: Vec<OtpDraft>) -> Result<OtpImportReport> {
        // Resolve all targets against the same vault state that produced the preview.
        let targets = self.preview_otp(&drafts);
        let mut result = OtpImportReport {
            attached: 0,
            created: 0,
            skipped: vec![],
            missing_batches: vec![],
        };
        let mut used = HashSet::new();
        for (d, target) in drafts.into_iter().zip(targets) {
            let uri = otp_uri(&d);
            if let OtpTarget::Attach { record, .. } = target.target {
                if used.insert(record) {
                    self.set_secret_value(&record, "totp_seed", Some(&uri))?;
                    result.attached += 1;
                    continue;
                }
            }
            self.insert_built(
                crate::RecordId::new(),
                NewRecord {
                    title: if d.issuer.is_empty() {
                        d.account.clone()
                    } else {
                        d.issuer.clone()
                    },
                    secret_kind: Some(SecretKind::Login),
                    ..Default::default()
                },
                "import",
                |r| {
                    r.set_secret_value("username", Some(&d.account))?;
                    r.set_secret_value("totp_seed", Some(&uri))?;
                    Ok(())
                },
            )?;
            result.created += 1;
        }
        Ok(result)
    }
}
