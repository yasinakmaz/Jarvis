//! Gizli değer maskeleme (Mimari §6, §8, §12).
//!
//! Günlük, olay ve hata metinleri yazılmadan önce buradan geçer.

/// Gizli değerin yerine yazılan metin.
pub const MASK: &str = "***";

/// Bilinen gizli değerleri ve `Bearer <token>` desenini [`MASK`] ile değiştirir.
///
/// Boş gizli değerler yok sayılır. `Bearer` büyük/küçük harf duyarsızdır.
#[must_use]
pub fn redact(text: &str, secrets: &[&str]) -> String {
    let masked = secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(text.to_owned(), |acc, secret| acc.replace(secret, MASK));
    mask_bearer(&masked)
}

const BEARER: &str = "bearer ";

/// `Bearer ` sonrasındaki boşluksuz belirteci maskeler. ASCII küçültme bayt konumlarını
/// korur; bu yüzden küçük harfli kopyada bulunan konumlar asıl metinde de geçerlidir.
fn mask_bearer(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(found) = lower.get(cursor..).and_then(|rest| rest.find(BEARER)) {
        let token_start = cursor.saturating_add(found).saturating_add(BEARER.len());
        let token_len = text.get(token_start..).map_or(0, |rest| {
            rest.find(char::is_whitespace).unwrap_or(rest.len())
        });
        out.push_str(text.get(cursor..token_start).unwrap_or_default());
        if token_len > 0 {
            out.push_str(MASK);
        }
        cursor = token_start.saturating_add(token_len);
    }
    out.push_str(text.get(cursor..).unwrap_or_default());
    out
}
