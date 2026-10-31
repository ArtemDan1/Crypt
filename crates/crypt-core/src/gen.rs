use crate::{CryptError, Result};
use rand::{Rng, rngs::OsRng, seq::SliceRandom};
use serde::Deserialize;
use zeroize::Zeroizing;

/// EFF large wordlist (7776 words): each word adds log2(7776) ≈ 12.9 bits.
pub const WORDLIST: &str = include_str!("eff_wordlist.txt");

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!\"#$%&'*+,./:;=?@\\^`|~";
const BRACKETS: &str = "[]{}()<>";
const LATIN1: &str = "äöüßéèçñåø";
const AMBIGUOUS: &str = "O0Il1|";

#[derive(Deserialize, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum GenMode {
    #[default]
    Chars,
    Words,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct PasswordOptions {
    pub mode: GenMode,
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub minus: bool,
    pub underline: bool,
    pub space: bool,
    pub symbols: bool,
    pub brackets: bool,
    pub latin1: bool,
    pub avoid_ambiguous: bool,
    pub each_class: bool,
    pub include: String,
    pub exclude: String,
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub add_number: bool,
}
impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            mode: GenMode::Chars,
            length: 24,
            lower: true,
            upper: true,
            digits: true,
            minus: false,
            underline: false,
            space: false,
            symbols: true,
            brackets: false,
            latin1: false,
            avoid_ambiguous: false,
            each_class: true,
            include: String::new(),
            exclude: String::new(),
            words: 5,
            separator: "-".into(),
            capitalize: false,
            add_number: false,
        }
    }
}

pub struct Generated {
    pub value: Zeroizing<String>,
    pub bits: u32,
}

fn invalid(msg: &str) -> CryptError {
    CryptError::Validation(msg.into())
}

/// Enabled character sets after removing ambiguous and excluded characters; empty sets dropped.
fn sets(o: &PasswordOptions) -> Vec<Vec<char>> {
    let keep = |c: &char| !(o.avoid_ambiguous && AMBIGUOUS.contains(*c)) && !o.exclude.contains(*c);
    [
        (o.lower, LOWER),
        (o.upper, UPPER),
        (o.digits, DIGITS),
        (o.minus, "-"),
        (o.underline, "_"),
        (o.space, " "),
        (o.symbols, SYMBOLS),
        (o.brackets, BRACKETS),
        (o.latin1, LATIN1),
        (!o.include.is_empty(), o.include.as_str()),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, s)| s.chars().filter(keep).collect::<Vec<_>>())
    .filter(|v| !v.is_empty())
    .collect()
}

fn chars(o: &PasswordOptions) -> Result<Generated> {
    if !(4..=128).contains(&o.length) {
        return Err(invalid("Длина пароля — от 4 до 128 символов"));
    }
    let sets = sets(o);
    let mut pool: Vec<char> = Vec::new();
    for c in sets.iter().flatten() {
        if !pool.contains(c) {
            pool.push(*c);
        }
    }
    if pool.is_empty() {
        return Err(invalid("Выберите хотя бы один набор символов"));
    }
    let mut rng = OsRng;
    let mut out: Vec<char> = Vec::with_capacity(o.length);
    if o.each_class && sets.len() <= o.length {
        for set in &sets {
            out.push(*set.choose(&mut rng).unwrap());
        }
    }
    while out.len() < o.length {
        out.push(*pool.choose(&mut rng).unwrap());
    }
    out.shuffle(&mut rng);
    let value = Zeroizing::new(out.iter().collect::<String>());
    out.iter_mut().for_each(|c| *c = '\0');
    let bits = (o.length as f64 * (pool.len() as f64).log2()).floor() as u32;
    Ok(Generated { value, bits })
}

fn words(o: &PasswordOptions) -> Result<Generated> {
    if !(3..=12).contains(&o.words) {
        return Err(invalid("Число слов — от 3 до 12"));
    }
    let list: Vec<&str> = WORDLIST.lines().collect();
    let mut rng = OsRng;
    let mut parts: Vec<String> = (0..o.words)
        .map(|_| {
            let w = *list.choose(&mut rng).unwrap();
            if o.capitalize {
                let mut cs = w.chars();
                cs.next()
                    .map(|f| f.to_uppercase().chain(cs).collect())
                    .unwrap_or_default()
            } else {
                w.to_string()
            }
        })
        .collect();
    let mut bits = o.words as f64 * (list.len() as f64).log2();
    if o.add_number {
        parts.push(rng.gen_range(0..=99u32).to_string());
        bits += 100f64.log2();
    }
    Ok(Generated {
        value: Zeroizing::new(parts.join(&o.separator)),
        bits: bits.floor() as u32,
    })
}

pub fn generate(o: &PasswordOptions) -> Result<Generated> {
    match o.mode {
        GenMode::Chars => chars(o),
        GenMode::Words => words(o),
    }
}

pub fn generate_password(o: &PasswordOptions) -> Result<Zeroizing<String>> {
    Ok(generate(o)?.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> PasswordOptions {
        PasswordOptions {
            lower: false,
            upper: false,
            digits: false,
            symbols: false,
            ..Default::default()
        }
    }
    const SETS: [&str; 4] = [
        "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "0123456789",
        "!\"#$%&'*+,./:;=?@\\^`|~",
    ];

    #[test]
    fn chars_respect_length_and_pool() {
        let o = PasswordOptions {
            length: 40,
            digits: true,
            ..none()
        };
        let g = generate(&o).unwrap();
        assert_eq!(g.value.chars().count(), 40);
        assert!(g.value.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn each_class_guarantees_every_set() {
        let o = PasswordOptions {
            length: 4,
            each_class: true,
            ..Default::default()
        };
        for _ in 0..100 {
            let g = generate(&o).unwrap();
            for set in SETS {
                assert!(
                    g.value.chars().any(|c| set.contains(c)),
                    "{} lacks {set}",
                    g.value.as_str()
                );
            }
        }
    }

    #[test]
    fn avoid_ambiguous_and_exclude() {
        let o = PasswordOptions {
            length: 64,
            avoid_ambiguous: true,
            exclude: "abc".into(),
            ..Default::default()
        };
        for _ in 0..20 {
            let g = generate(&o).unwrap();
            assert!(
                !g.value.chars().any(|c| "O0Il1|abc".contains(c)),
                "{}",
                g.value.as_str()
            );
        }
    }

    #[test]
    fn include_adds_chars() {
        let o = PasswordOptions {
            length: 8,
            include: "€".into(),
            ..none()
        };
        assert_eq!(generate(&o).unwrap().value.as_str(), "€€€€€€€€");
    }

    #[test]
    fn empty_pool_is_error() {
        let err = generate(&PasswordOptions {
            length: 8,
            ..none()
        })
        .err()
        .unwrap();
        assert!(
            err.to_string()
                .contains("Выберите хотя бы один набор символов")
        );
    }

    #[test]
    fn words_mode() {
        let o = PasswordOptions {
            mode: GenMode::Words,
            words: 5,
            separator: "-".into(),
            capitalize: true,
            ..Default::default()
        };
        let list: std::collections::HashSet<&str> = WORDLIST.lines().collect();
        let g = generate(&o).unwrap();
        let parts: Vec<&str> = g.value.split('-').collect();
        assert_eq!(parts.len(), 5);
        for p in &parts {
            assert!(p.chars().next().unwrap().is_uppercase());
            assert!(list.contains(p.to_lowercase().as_str()), "{p}");
        }
        let o = PasswordOptions {
            add_number: true,
            ..o
        };
        let g = generate(&o).unwrap();
        let parts: Vec<&str> = g.value.split('-').collect();
        assert_eq!(parts.len(), 6);
        assert!(parts[5].parse::<u32>().unwrap() <= 99);
    }

    #[test]
    fn bits_values() {
        let digits = PasswordOptions {
            length: 6,
            digits: true,
            ..none()
        };
        assert_eq!(generate(&digits).unwrap().bits, 19);
        let words = PasswordOptions {
            mode: GenMode::Words,
            words: 5,
            ..Default::default()
        };
        assert_eq!(generate(&words).unwrap().bits, 64);
        let words_num = PasswordOptions {
            add_number: true,
            ..words
        };
        assert_eq!(generate(&words_num).unwrap().bits, 71);
    }

    #[test]
    fn wordlist_has_7776_unique_words() {
        let all: Vec<&str> = WORDLIST.lines().collect();
        let set: std::collections::HashSet<&str> = all.iter().copied().collect();
        assert_eq!(all.len(), 7776);
        assert_eq!(set.len(), 7776);
    }

    #[test]
    fn old_json_still_deserializes() {
        let o: PasswordOptions = serde_json::from_str(
            r#"{"length":20,"lower":true,"upper":false,"digits":true,"symbols":false,"avoidAmbiguous":true}"#,
        )
        .unwrap();
        assert_eq!(o.mode, GenMode::Chars);
        assert_eq!(o.length, 20);
        assert!(o.avoid_ambiguous && !o.upper);
    }
}
