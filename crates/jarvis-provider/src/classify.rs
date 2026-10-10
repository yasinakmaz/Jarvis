//! Kütüphane hatalarını yeniden denenebilir / denenemez diye sınıflar (Tasarım 0006).
//!
//! `async-openai` HTTP başlıklarını (ör. `Retry-After`) hata yolunda iletmez; bu yüzden
//! 429'da bekleme süresi her zaman kendi geri çekilmemizden gelir.

use async_openai::error::OpenAIError;
use jarvis_types::redact;

use crate::{ProviderError, RetryReason};

/// Hata mesajlarının en uzun hali (karakter).
pub const MAX_MESSAGE_CHARS: usize = 512;

/// Yeniden denenebilir başarısızlık.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retry {
    pub reason: RetryReason,
    pub status: Option<u16>,
    pub message: String,
}

/// Bir denemenin hatalı sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Beklenip yeniden denenir.
    Retry(Retry),
    /// Hemen döner.
    Fatal(ProviderError),
}

impl Outcome {
    /// Yeniden denenmeyecekse (denemeler tükendi / akış ortası) nihai hata.
    #[must_use]
    pub fn into_final(self) -> ProviderError {
        match self {
            Self::Fatal(error) => error,
            Self::Retry(Retry {
                reason: RetryReason::RateLimited,
                ..
            }) => ProviderError::RateLimited { retry_after: None },
            Self::Retry(Retry {
                reason: RetryReason::Unavailable,
                status,
                message,
            }) => ProviderError::Unavailable { status, message },
        }
    }
}

/// Gizli değerleri maskeler ve en çok [`MAX_MESSAGE_CHARS`] karaktere kısaltır.
#[must_use]
pub fn sanitize(text: &str, secrets: &[&str]) -> String {
    let masked = redact(text, secrets);
    let mut chars = masked.chars();
    let head: String = chars.by_ref().take(MAX_MESSAGE_CHARS).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

/// HTTP durumundan sınıflama.
#[must_use]
pub fn from_status(status: u16, message: &str, secrets: &[&str]) -> Outcome {
    let message = sanitize(message, secrets);
    match status {
        429 => Outcome::Retry(Retry {
            reason: RetryReason::RateLimited,
            status: Some(status),
            message,
        }),
        408 | 500..=599 => Outcome::Retry(Retry {
            reason: RetryReason::Unavailable,
            status: Some(status),
            message,
        }),
        400..=499 => Outcome::Fatal(ProviderError::Rejected { status, message }),
        _ => Outcome::Fatal(ProviderError::InvalidResponse(format!(
            "beklenmeyen HTTP durumu {status}: {message}"
        ))),
    }
}

/// `async-openai` hatasından sınıflama.
#[must_use]
pub fn from_openai(error: OpenAIError, secrets: &[&str]) -> Outcome {
    match error {
        OpenAIError::ApiError(response) => from_status(
            response.status_code.as_u16(),
            &response.api_error.to_string(),
            secrets,
        ),
        OpenAIError::Reqwest(error) => {
            let detail = if error.is_timeout() {
                "zaman aşımı".to_owned()
            } else if error.is_connect() {
                "bağlantı kurulamadı".to_owned()
            } else {
                sanitize(&error.to_string(), secrets)
            };
            Outcome::Retry(Retry {
                reason: RetryReason::Unavailable,
                status: error.status().map(|status| status.as_u16()),
                message: detail,
            })
        }
        OpenAIError::StreamError(error) => Outcome::Retry(Retry {
            reason: RetryReason::Unavailable,
            status: None,
            message: sanitize(&error.to_string(), secrets),
        }),
        OpenAIError::JSONDeserialize(error, content) => {
            Outcome::Fatal(ProviderError::InvalidResponse(format!(
                "yanıt çözümlenemedi ({error}); içerik: {}",
                sanitize(&content, secrets)
            )))
        }
        OpenAIError::InvalidArgument(detail) => {
            Outcome::Fatal(ProviderError::InvalidRequest(sanitize(&detail, secrets)))
        }
        other => Outcome::Fatal(ProviderError::InvalidResponse(sanitize(
            &other.to_string(),
            secrets,
        ))),
    }
}

#[cfg(test)]
#[path = "classify_tests.rs"]
mod tests;
