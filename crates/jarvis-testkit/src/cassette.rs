//! Kaset biçimi: satır başına bir tur (Tasarım 0011). Ayrıştırma ve gövde karşılaştırması.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

/// Kaset hataları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CassetteError {
    /// Dosya okunamadı.
    #[error("kaset dosyası okunamadı ({path}): {detail}")]
    Io {
        /// Dosya yolu.
        path: String,
        /// Sistem hatası.
        detail: String,
    },
    /// Satır ayrıştırılamadı ya da kural ihlali.
    #[error("kaset satır {line}: {detail}")]
    Parse {
        /// 1'den başlayan satır numarası.
        line: usize,
        /// Neden.
        detail: String,
    },
    /// Kasette hiç tur yok.
    #[error("kaset boş: en az bir tur gerekir")]
    Empty,
    /// Sunucu dinleyemedi.
    #[error("kaset sunucusu başlatılamadı: {0}")]
    Bind(String),
    /// Tekrar sırasında beklenmeyen durum (uyuşmazlık, tükenme, kullanılmayan tur).
    #[error("kaset tekrarı tutarsız: {0}")]
    Replay(String),
}

/// Beklenen istek.
#[derive(Debug, Clone, Deserialize)]
pub struct ExpectedRequest {
    /// HTTP yöntemi.
    pub method: String,
    /// Yol, ör. `/v1/chat/completions`.
    pub path: String,
    /// Kanonik JSON olarak karşılaştırılan gövde.
    pub body: Value,
}

/// Kayıtlı yanıt: `body`, `stream` ya da `text`'ten tam biri.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseSpec {
    /// HTTP durumu.
    pub status: u16,
    /// Ek başlıklar.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// JSON gövde.
    pub body: Option<Value>,
    /// SSE `data:` parçaları (`[DONE]` dahil).
    pub stream: Option<Vec<String>>,
    /// JSON olmayan gövde (ör. HTML hata sayfası).
    pub text: Option<String>,
    /// Yanıttan önce gerçek zamanlı bekleme (zaman aşımı/iptal testleri).
    pub delay_ms: Option<u64>,
}

/// Bir tur.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Turn {
    /// 1'den başlayan sıra.
    pub turn: u32,
    /// Beklenen istek.
    pub request: ExpectedRequest,
    /// Verilecek yanıt.
    pub response: ResponseSpec,
}

/// Kaset metnini ayrıştırır.
///
/// # Errors
///
/// Boş kaset, bozuk satır, sıra dışı tur numarası ya da yanıtta `body`/`stream`/`text`
/// kuralının ihlali.
pub fn parse(text: &str) -> Result<Vec<Turn>, CassetteError> {
    let mut turns: Vec<Turn> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let number = index.saturating_add(1);
        let fail = |detail: String| CassetteError::Parse {
            line: number,
            detail,
        };
        let turn: Turn = serde_json::from_str(line).map_err(|e| fail(format!("JSON: {e}")))?;
        let expected = u32::try_from(turns.len())
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| fail("çok fazla tur".to_owned()))?;
        if turn.turn != expected {
            return Err(fail(format!(
                "tur numarası {expected} olmalı, bulunan {}",
                turn.turn
            )));
        }
        let response = &turn.response;
        let sources = [
            response.body.is_some(),
            response.stream.is_some(),
            response.text.is_some(),
        ];
        if sources.into_iter().filter(|present| *present).count() != 1 {
            return Err(fail(
                "yanıtta body, stream ve text'ten tam biri bulunmalı".to_owned(),
            ));
        }
        turns.push(turn);
    }
    if turns.is_empty() {
        return Err(CassetteError::Empty);
    }
    Ok(turns)
}

/// İki JSON değerinin ilk farkı, JSON işaretçisiyle; eşitse `None`.
#[must_use]
pub fn first_difference(expected: &Value, actual: &Value) -> Option<String> {
    difference_at("", expected, actual)
}

fn difference_at(path: &str, expected: &Value, actual: &Value) -> Option<String> {
    match (expected, actual) {
        (Value::Object(want), Value::Object(got)) => {
            for (key, want_value) in want {
                let here = format!("{path}/{key}");
                match got.get(key) {
                    None => return Some(format!("{here}: eksik (beklenen {want_value})")),
                    Some(got_value) => {
                        if let Some(found) = difference_at(&here, want_value, got_value) {
                            return Some(found);
                        }
                    }
                }
            }
            got.keys()
                .find(|key| !want.contains_key(*key))
                .map(|key| format!("{path}/{key}: fazladan alan"))
        }
        (Value::Array(want), Value::Array(got)) => {
            for (index, (want_item, got_item)) in want.iter().zip(got).enumerate() {
                if let Some(found) = difference_at(&format!("{path}/{index}"), want_item, got_item)
                {
                    return Some(found);
                }
            }
            (want.len() != got.len()).then(|| {
                format!(
                    "{}: dizi uzunluğu beklenen {}, gelen {}",
                    root(path),
                    want.len(),
                    got.len()
                )
            })
        }
        _ => (expected != actual)
            .then(|| format!("{}: beklenen {expected}, gelen {actual}", root(path))),
    }
}

const fn root(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}
