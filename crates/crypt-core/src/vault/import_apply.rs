use super::{Vault, import_csv::{LoginDraft, SkipReason, domain}};
use crate::{Result, SecretKind, NewRecord};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::collections::HashSet;
#[derive(Debug,Serialize)]
pub struct LoginPreview { pub new:usize, pub duplicates:usize, pub skipped:usize, pub sample:Vec<(String,String)> }
#[derive(Debug,Serialize)]
pub struct LoginImportReport { pub imported:usize, pub duplicates:usize, pub skipped:Vec<(usize,SkipReason)> }
fn key(url:&str, username:&str, password:&str) -> [u8;32] {
    let mut hash=Sha256::new();
    for value in [domain(url).as_str(),username,password] { hash.update((value.len() as u64).to_le_bytes()); hash.update(value.as_bytes()); }
    hash.finalize().into()
}
impl Vault {
    fn login_keys(&self) -> HashSet<[u8;32]> {
        self.records.values().filter(|r| r.deleted_at().is_none() && r.secret_kind()==Some(SecretKind::Login)).filter_map(|r| {
            let password=r.secret_value("password").ok().flatten()?;
            let url=r.secret_value("url").ok().flatten().unwrap_or_default(); let user=r.secret_value("username").ok().flatten().unwrap_or_default();
            Some(key(&url,&user,&password))
        }).collect()
    }
    pub fn preview_logins(&self, drafts:&[LoginDraft]) -> LoginPreview {
        let mut seen=self.login_keys(); let mut result=LoginPreview {new:0,duplicates:0,skipped:0,sample:vec![]};
        for d in drafts { if seen.insert(key(d.url.as_deref().unwrap_or(""),&d.username,&d.password)) {
            result.new+=1; if result.sample.len()<5 {result.sample.push((d.title.clone(),d.username.clone()));}
        } else {result.duplicates+=1;} }
        result
    }
    pub fn import_logins(&mut self, drafts:Vec<LoginDraft>, folder_name:&str) -> Result<LoginImportReport> {
        let folder_name=folder_name.trim(); if folder_name.is_empty() {return Err(crate::CryptError::Validation("Укажите папку импорта".into()));}
        let folder=match self.folders().iter().find(|f|f.name==folder_name && f.parent.is_none()) {Some(f)=>f.id,None=>self.create_folder(folder_name,None)?};
        let mut seen=self.login_keys(); let mut result=LoginImportReport {imported:0,duplicates:0,skipped:vec![]};
        for d in drafts {
            if !seen.insert(key(d.url.as_deref().unwrap_or(""),&d.username,&d.password)) {result.duplicates+=1;continue;}
            self.insert_built(crate::RecordId::new(),NewRecord {title:d.title,secret_kind:Some(SecretKind::Login),folder:Some(folder),..Default::default()},"import",|r| {
                r.set_secret_value("password",Some(&d.password))?; r.set_secret_value("username",Some(&d.username))?; r.set_secret_value("url",d.url.as_deref())?;
                if let Some(note)=d.note {r.set_body(&note)?;}
                if let Some(totp)=d.totp {r.set_secret_value("totp_seed",Some(&totp))?;}
                Ok(())
            })?; result.imported+=1;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::import_csv::parse_browser_csv;
    #[test]
    fn duplicates_inside_file_and_existing_but_changed_password_is_new() {
        let d=tempfile::tempdir().unwrap(); let mut v=Vault::create(d.path(),&crate::SeedPhrase::generate()).unwrap();
        let bytes=b"name,url,username,password,note\nA,https://www.example.com/a,u,p,\nB,https://EXAMPLE.com/b,u,p,\nC,https://example.com,u,different,";
        let parsed=parse_browser_csv(bytes).unwrap();
        let p=v.preview_logins(&parsed.logins); assert_eq!((p.new,p.duplicates),(2,1));
        let r=v.import_logins(parsed.logins,"Browser").unwrap(); assert_eq!((r.imported,r.duplicates),(2,1));
        assert_eq!(v.folders().len(),1);
        let p=v.preview_logins(&parse_browser_csv(bytes).unwrap().logins); assert_eq!((p.new,p.duplicates),(0,3));
    }
}

use super::import_otp::{OtpDraft,OtpPreviewRow,OtpTarget,OtpImportReport,otp_uri};
impl Vault {
    fn otp_target(&self,d:&OtpDraft) -> OtpTarget {
        let issuer=d.issuer.trim().to_lowercase();
        if issuer.is_empty() {return OtpTarget::New;}
        let mut candidates=vec![];
        for r in self.records.values().filter(|r|r.deleted_at().is_none() && r.secret_kind()==Some(SecretKind::Login)) {
            let url=r.secret_value("url").ok().flatten().unwrap_or_default();let host=domain(&url);
            let labels:Vec<_>=host.split('.').collect();
            let service=host==issuer || (labels.len()>1 && labels[..labels.len()-1].contains(&issuer.as_str()));
            let user=r.secret_value("username").ok().flatten().unwrap_or_default();
            if service && (user.is_empty() || *user==d.account) {candidates.push(r);}
        }
        if candidates.len()!=1 {return OtpTarget::New;}
        let r=candidates[0];if r.secret_value("totp_seed").ok().flatten().is_some_and(|s|!s.is_empty()) {return OtpTarget::New;}
        OtpTarget::Attach {record:r.id(),title:r.summary().title}
    }
    pub fn preview_otp(&self,drafts:&[OtpDraft]) -> Vec<OtpPreviewRow> {
        let mut used=HashSet::new();
        drafts.iter().map(|d| {
            let target=match self.otp_target(d) {OtpTarget::Attach {record,title} if used.insert(record)=>OtpTarget::Attach {record,title},_=>OtpTarget::New};
            OtpPreviewRow {issuer:d.issuer.clone(),account:d.account.clone(),target}
        }).collect()
    }
    pub fn import_otp(&mut self,drafts:Vec<OtpDraft>,folder_name:&str) -> Result<OtpImportReport> {
        let folder_name=folder_name.trim();if folder_name.is_empty() {return Err(crate::CryptError::Validation("Укажите папку импорта".into()));}
        // Resolve all targets against the same vault state that produced the preview.
        let targets=self.preview_otp(&drafts);let mut result=OtpImportReport {attached:0,created:0,skipped:vec![],missing_batches:vec![]};
        let mut used=HashSet::new();let mut folder=None;
        for (d,target) in drafts.into_iter().zip(targets) {
            let uri=otp_uri(&d);
            if let OtpTarget::Attach {record,..}=target.target {
                if used.insert(record) {self.set_secret_value(&record,"totp_seed",Some(&uri))?;result.attached+=1;continue;}
            }
            let folder_id=match folder {Some(id)=>id,None=>{
                let id=match self.folders().iter().find(|f|f.name==folder_name && f.parent.is_none()) {Some(f)=>f.id,None=>self.create_folder(folder_name,None)?};folder=Some(id);id
            }};
            self.insert_built(crate::RecordId::new(),NewRecord {title:if d.issuer.is_empty(){d.account.clone()}else{d.issuer.clone()},secret_kind:Some(SecretKind::Login),folder:Some(folder_id),..Default::default()},"import",|r| {
                r.set_secret_value("username",Some(&d.account))?;r.set_secret_value("totp_seed",Some(&uri))?;Ok(())
            })?;result.created+=1;
        }
        Ok(result)
    }
}
