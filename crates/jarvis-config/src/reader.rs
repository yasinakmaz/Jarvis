//! TOML tablosunu şemaya göre okuma yardımcıları. Her okuma hatası listeye eklenir ve `None`
//! döner; çağıran sonraki alanlara devam eder (ilk hatada durulmaz).

use std::num::NonZeroU32;

use toml::{Table, Value};

use crate::{ConfigError, ConfigErrorKind};

/// Toplanan hatalar.
pub type Errors = Vec<ConfigError>;

/// Bu uzaklığa kadar bilinmeyen alana "şunu mu demek istediniz" önerilir.
const SUGGEST_DISTANCE: usize = 2;

/// Noktalı alan yolu.
pub fn join(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

/// Hata mesajlarındaki TOML tip adı.
const fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "metin",
        Value::Integer(_) => "tamsayı",
        Value::Float(_) => "ondalık",
        Value::Boolean(_) => "mantıksal",
        Value::Datetime(_) => "tarih",
        Value::Array(_) => "dizi",
        Value::Table(_) => "tablo",
    }
}

fn wrong_type(expected: &'static str, value: &Value, path: &str, errors: &mut Errors) {
    let kind = ConfigErrorKind::WrongType {
        expected,
        found: type_name(value),
    };
    errors.push(ConfigError::new(path, kind));
}

/// Metin bekler.
pub fn expect_str<'v>(value: &'v Value, path: &str, errors: &mut Errors) -> Option<&'v str> {
    value.as_str().or_else(|| {
        wrong_type("metin", value, path, errors);
        None
    })
}

/// Tablo bekler.
pub fn expect_table<'v>(value: &'v Value, path: &str, errors: &mut Errors) -> Option<&'v Table> {
    value.as_table().or_else(|| {
        wrong_type("tablo", value, path, errors);
        None
    })
}

/// Dizi bekler.
pub fn expect_array<'v>(value: &'v Value, path: &str, errors: &mut Errors) -> Option<&'v [Value]> {
    value.as_array().map(Vec::as_slice).or_else(|| {
        wrong_type("dizi", value, path, errors);
        None
    })
}

/// Tamsayı bekler.
pub fn expect_int(value: &Value, path: &str, errors: &mut Errors) -> Option<i64> {
    value.as_integer().or_else(|| {
        wrong_type("tamsayı", value, path, errors);
        None
    })
}

/// `1..=u32::MAX` aralığında tamsayı bekler.
pub fn expect_positive(value: &Value, path: &str, errors: &mut Errors) -> Option<NonZeroU32> {
    let found = expect_int(value, path, errors)?;
    u32::try_from(found)
        .ok()
        .and_then(NonZeroU32::new)
        .or_else(|| {
            let kind = ConfigErrorKind::OutOfRange {
                found,
                min: 1,
                max: i64::from(u32::MAX),
            };
            errors.push(ConfigError::new(path, kind));
            None
        })
}

/// Hata zaten listeye eklendi; çağıran yalnızca durur.
#[derive(Debug)]
pub struct Reported;

/// Şeması bilinen bir tablo. Açılırken bilinmeyen alanları raporlar.
pub struct Section<'a> {
    path: String,
    table: &'a Table,
}

impl<'a> Section<'a> {
    /// Tabloyu açar; `known` dışındaki alanları (sıralı, öneriyle) raporlar.
    pub fn open(path: String, table: &'a Table, known: &[&str], errors: &mut Errors) -> Self {
        let mut unknown: Vec<&String> = table
            .keys()
            .filter(|key| !known.contains(&key.as_str()))
            .collect();
        unknown.sort();
        for key in unknown {
            let kind = ConfigErrorKind::UnknownField {
                suggestion: suggest(key, known),
            };
            errors.push(ConfigError::new(join(&path, key), kind));
        }
        Self { path, table }
    }

    /// Alanın yolu.
    pub fn path(&self, key: &str) -> String {
        join(&self.path, key)
    }

    /// İsteğe bağlı alan.
    pub fn optional(&self, key: &str) -> Option<&'a Value> {
        self.table.get(key)
    }

    /// Zorunlu alan; yoksa hata.
    pub fn required(&self, key: &str, errors: &mut Errors) -> Option<&'a Value> {
        self.optional(key).or_else(|| {
            errors.push(ConfigError::new(
                self.path(key),
                ConfigErrorKind::MissingField,
            ));
            None
        })
    }

    /// Zorunlu metin alanı.
    pub fn required_str(&self, key: &str, errors: &mut Errors) -> Option<&'a str> {
        expect_str(self.required(key, errors)?, &self.path(key), errors)
    }

    /// Zorunlu pozitif tamsayı alanı.
    pub fn required_positive(&self, key: &str, errors: &mut Errors) -> Option<NonZeroU32> {
        expect_positive(self.required(key, errors)?, &self.path(key), errors)
    }

    /// İsteğe bağlı alt tablo: yoksa `Ok(None)`; tablo değilse hata eklenir ve `Err`.
    pub fn child(
        &self,
        key: &str,
        known: &[&str],
        errors: &mut Errors,
    ) -> Result<Option<Self>, Reported> {
        let Some(value) = self.optional(key) else {
            return Ok(None);
        };
        let path = self.path(key);
        let table = expect_table(value, &path, errors).ok_or(Reported)?;
        Ok(Some(Self::open(path, table, known, errors)))
    }
}

/// En yakın bilinen alan adı (yazım hatası önerisi).
fn suggest(key: &str, known: &[&str]) -> Option<String> {
    known
        .iter()
        .map(|candidate| (distance(key, candidate), *candidate))
        .filter(|(d, _)| *d <= SUGGEST_DISTANCE)
        .min()
        .map(|(_, candidate)| candidate.to_owned())
}

/// Levenshtein uzaklığı (karakter bazında).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i.saturating_add(1)];
        for ((&diag, &up), &cb) in prev.iter().zip(prev.iter().skip(1)).zip(&b) {
            let left = cur.last().copied().unwrap_or(usize::MAX);
            let replace = diag.saturating_add(usize::from(ca != cb));
            cur.push(
                replace
                    .min(up.saturating_add(1))
                    .min(left.saturating_add(1)),
            );
        }
        prev = cur;
    }
    prev.last().copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_counts_single_character_edits() {
        assert_eq!(distance("", ""), 0);
        assert_eq!(distance("model", "model"), 0);
        assert_eq!(distance("vison", "vision"), 1);
        assert_eq!(distance("model", "modle"), 2);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("abc", ""), 3);
    }

    #[test]
    fn suggestion_is_the_closest_known_name_within_two_edits() {
        assert_eq!(
            suggest("fats", &["fast", "planner"]),
            Some("fast".to_owned())
        );
        assert_eq!(
            suggest("planer", &["planner", "fast"]),
            Some("planner".to_owned())
        );
        assert_eq!(suggest("xyz", &["fast"]), None);
        assert_eq!(suggest("abcd", &["abxy", "abcx"]), Some("abcx".to_owned()));
    }
}
