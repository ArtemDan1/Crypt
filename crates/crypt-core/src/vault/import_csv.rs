use crate::{CryptError, Result};
use serde::Serialize;
use zeroize::Zeroizing;
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize)]
#[serde(rename_all="lowercase")]
pub enum CsvFormat { Chromium, Safari, Firefox }
pub struct LoginDraft { pub title: String, pub url: Option<String>, pub username: String, pub password: Zeroizing<String>, pub note: Option<String>, pub totp: Option<Zeroizing<String>> }
pub struct CsvParse { pub format: CsvFormat, pub logins: Vec<LoginDraft>, pub skipped: Vec<(usize, SkipReason)> }
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub enum SkipReason { NoPassword, BadRow, BadTotp }
fn format_error() -> CryptError { CryptError::ImportFormat("Не похоже на выгрузку паролей браузера. Ожидаются колонки: name/url/username/password или Title/URL/Username/Password".into()) }
fn rows(text: &str) -> Result<Zeroizing<Vec<Vec<String>>>> {
    let mut rows = Zeroizing::new(Vec::new()); let mut row = Zeroizing::new(Vec::new()); let mut field = Zeroizing::new(String::new());
    let mut quoted = false; let mut closed = false; let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' { if chars.peek() == Some(&'"') { chars.next(); field.push('"'); } else { quoted=false; closed=true; } }
            else { field.push(c); }
        } else {
            match c {
                '"' if field.is_empty() && !closed => quoted=true,
                ',' => { row.push(std::mem::take(&mut *field)); closed=false; },
                '\r' | '\n' => {
                    if c == '\r' && chars.peek() == Some(&'\n') { chars.next(); }
                    row.push(std::mem::take(&mut *field));
                    if row.len() > 1 || !row[0].is_empty() { rows.push(std::mem::take(&mut *row)); } else { row.clear(); }
                    closed=false;
                },
                _ if closed || c == '"' => return Err(format_error()),
                _ => field.push(c),
            }
        }
    }
    if quoted { return Err(format_error()); }
    if !field.is_empty() || !row.is_empty() || closed { row.push(std::mem::take(&mut *field)); rows.push(std::mem::take(&mut *row)); }
    Ok(rows)
}
pub fn domain(url: &str) -> String {
    let rest=url.trim().split_once("://").map(|(_,v)|v).unwrap_or(url.trim());
    let host=rest.split(['/', '?', '#']).next().unwrap_or("").rsplit('@').next().unwrap_or("");
    let host=if host.starts_with('[') { host.split(']').next().unwrap_or(host).trim_start_matches('[') } else { host.split(':').next().unwrap_or("") };
    host.trim_end_matches('.').to_lowercase().trim_start_matches("www.").to_owned()
}
pub fn parse_browser_csv(bytes: &[u8]) -> Result<CsvParse> {
    let text=std::str::from_utf8(bytes).map_err(|_|format_error())?; let rows=rows(text)?; let header=rows.first().ok_or_else(format_error)?;
    let has=|name:&str| header.iter().any(|h|h==name);
    let format=if ["Title","URL","Username","Password"].iter().all(|h|has(h)) { CsvFormat::Safari }
        else if ["url","username","password"].iter().all(|h|has(h)) { if has("name") { CsvFormat::Chromium } else { CsvFormat::Firefox } }
        else { return Err(format_error()); };
    let safari=format==CsvFormat::Safari; let col=|name:&str|header.iter().position(|h|h==name);
    let mut result=CsvParse {format,logins:vec![],skipped:vec![]};
    for (i,row) in rows.iter().enumerate().skip(1) {
        let line=i+1;
        if row.len()!=header.len() { result.skipped.push((line,SkipReason::BadRow)); continue; }
        let get=|name:&str| col(name).and_then(|i|row.get(i)).map(String::as_str).unwrap_or("");
        let password=get(if safari {"Password"} else {"password"});
        if password.is_empty() { result.skipped.push((line,SkipReason::NoPassword)); continue; }
        let url=get(if safari {"URL"} else {"url"}); let title=get(if safari {"Title"} else {"name"});
        let title=if title.trim().is_empty() { domain(url) } else {title.to_owned()};
        let otp=get("OTPAuth"); let mut totp=None;
        if !otp.is_empty() { if crate::totp::parse_totp(otp).is_ok() {totp=Some(Zeroizing::new(otp.to_owned()));} else {result.skipped.push((line,SkipReason::BadTotp));} }
        let note=get(if safari {"Notes"} else {"note"});
        result.logins.push(LoginDraft {title,url:(!url.is_empty()).then(||url.to_owned()),username:get(if safari {"Username"} else {"username"}).to_owned(),password:Zeroizing::new(password.to_owned()),note:(!note.is_empty()).then(||note.to_owned()),totp});
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_samples_and_multiline_otp() {
        let chrome=parse_browser_csv(include_bytes!("../../tests/fixtures/csv/chrome.csv")).unwrap();
        assert_eq!(chrome.format,CsvFormat::Chromium); assert_eq!(chrome.logins.len(),1);
        let safari=parse_browser_csv(include_bytes!("../../tests/fixtures/csv/safari.csv")).unwrap();
        assert_eq!(safari.format,CsvFormat::Safari); assert_eq!(safari.logins[0].note.as_deref(),Some("first line\nsecond line")); assert!(safari.logins[0].totp.is_some());
        assert_eq!(parse_browser_csv(include_bytes!("../../tests/fixtures/csv/firefox.csv")).unwrap().format,CsvFormat::Firefox);
    }
    #[test]
    fn bom_crlf_quoted_and_skipped() {
        let parsed=parse_browser_csv("\u{feff}name,url,username,password,note\r\n\"a,b\",https://example.com,u,\"p\"\"q\",n\r\nMissing,https://x,u,,\r\nBad,only\r\n".as_bytes()).unwrap();
        assert_eq!(parsed.logins[0].title,"a,b"); assert_eq!(&*parsed.logins[0].password,"p\"q");
        assert_eq!(parsed.skipped,vec![(3,SkipReason::NoPassword),(4,SkipReason::BadRow)]);
    }
    #[test]
    fn unknown_headers() { assert!(matches!(parse_browser_csv(b"hello,world\na,b"),Err(CryptError::ImportFormat(_)))); }
    #[test]
    fn bad_totp_keeps_login() { let p=parse_browser_csv(b"Title,URL,Username,Password,Notes,OTPAuth\nA,https://x,u,p,,invalid!!!").unwrap(); assert_eq!(p.logins.len(),1); assert_eq!(p.skipped,vec![(2,SkipReason::BadTotp)]); }
}
