//! Sağlayıcı hataları (Tasarım 0006). Mesajlar gizli değerlerden arındırılmış olarak üretilir.

use std::time::Duration;

use jarvis_types::ErrorCode;

/// Sağlayıcı çağrısının başarısızlık nedeni.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProviderError {
    /// 429; denemeler tükendi.
    #[error("sağlayıcı istek sınırına takıldı; denemeler tükendi")]
    RateLimited {
        /// Sağlayıcının bildirdiği bekleme süresi (kütüphane başlığı iletmediği için şimdilik
        /// hep `None`; bkz. Tasarım 0006 uygulama notları).
        retry_after: Option<Duration>,
    },
    /// 5xx, zaman aşımı veya ağ hatası; denemeler tükendi.
    #[error("sağlayıcıya ulaşılamadı{}: {message}", status_suffix(*.status))]
    Unavailable {
        /// HTTP durumu; ağ/zaman aşımı hatalarında yok.
        status: Option<u16>,
        /// Gizli değerlerden arındırılmış, kısaltılmış ayrıntı.
        message: String,
    },
    /// 400/401/403/404 ve diğer 4xx: denenmez.
    #[error("sağlayıcı isteği reddetti (HTTP {status}): {message}")]
    Rejected {
        /// HTTP durumu.
        status: u16,
        /// Gizli değerlerden arındırılmış, kısaltılmış ayrıntı.
        message: String,
    },
    /// Yanıt alan modeline eşlenemedi.
    #[error("sağlayıcı yanıtı anlaşılamadı: {0}")]
    InvalidResponse(String),
    /// İstek tel biçimine çevrilemedi (ör. M1'de desteklenmeyen görüntü girdisi).
    #[error("istek sağlayıcıya gönderilemez: {0}")]
    InvalidRequest(String),
    /// Çağıran vazgeçti.
    #[error("sağlayıcı çağrısı iptal edildi")]
    Cancelled,
    /// Anahtarın ortam değişkeni kurulumda okunamadı.
    #[error(
        "sağlayıcı `{provider}` için `{env_var}` ortam değişkeni tanımlı değil, boş ya da \
         geçerli UTF-8 değil"
    )]
    MissingKey {
        /// Sağlayıcı adı.
        provider: String,
        /// Ortam değişkeninin adı (değeri değil).
        env_var: String,
    },
}

fn status_suffix(status: Option<u16>) -> String {
    status.map_or_else(String::new, |code| format!(" (HTTP {code})"))
}

impl ProviderError {
    /// API'nin hata gövdesindeki kod (Mimari §4).
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::RateLimited { .. }
            | Self::Unavailable { .. }
            | Self::InvalidResponse(_)
            | Self::MissingKey { .. } => ErrorCode::ProviderUnavailable,
            Self::Rejected { .. } => ErrorCode::ProviderRejected,
            Self::InvalidRequest(_) => ErrorCode::InvalidRequest,
            Self::Cancelled => ErrorCode::Halted,
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
