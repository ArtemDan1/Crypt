use crate::{CryptError, Result};
use rand::{rngs::OsRng, seq::SliceRandom};
use serde::Deserialize;
use zeroize::Zeroizing;
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct PasswordOptions {
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
}
impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 24,
            lower: true,
            upper: true,
            digits: true,
            symbols: true,
            avoid_ambiguous: false,
        }
    }
}
pub fn generate_password(o: &PasswordOptions) -> Result<Zeroizing<String>> {
    let classes: Vec<Vec<u8>> = [
        (o.lower, "abcdefghijklmnopqrstuvwxyz"),
        (o.upper, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        (o.digits, "0123456789"),
        (o.symbols, "!#$%&*+-=?@^_~"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, s)| {
        s.bytes()
            .filter(|c| !o.avoid_ambiguous || !b"0O1lI".contains(c))
            .collect()
    })
    .collect();
    if !(8..=128).contains(&o.length) || classes.is_empty() || o.length < classes.len() {
        return Err(CryptError::Validation("invalid password options".into()));
    }
    let mut rng = OsRng;
    let alphabet: Vec<u8> = classes.iter().flatten().copied().collect();
    let mut bytes = Zeroizing::new(Vec::with_capacity(o.length));
    for class in &classes {
        bytes.push(*class.choose(&mut rng).unwrap());
    }
    while bytes.len() < o.length {
        bytes.push(*alphabet.choose(&mut rng).unwrap());
    }
    bytes.shuffle(&mut rng);
    Ok(Zeroizing::new(bytes.iter().map(|&b| b as char).collect()))
}
