//! API'nin `OpenAI` hata gövdesindeki `code` alanında taşınan kodlar (Mimari §4).

use serde::{Deserialize, Serialize};

/// Jarvis hata kodu. Tel adı [`ErrorCode::as_str`] ile aynıdır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorCode {
    /// Kullanıcı riskli bir eylemi onaylamadı ya da onay zaman aşımına uğradı.
    ApprovalDenied,
    /// Masaüstü arka ucu kullanılamıyor (portal reddi, bozuk oturum).
    DesktopUnavailable,
    /// Adım, süre veya tekrar sınırı aşıldı.
    LimitReached,
    /// Koşu iptal edildi ya da `POST /halt` çağrıldı.
    Halted,
    /// Sağlayıcıya ulaşılamadı (denemeler tükendi).
    ProviderUnavailable,
    /// Sağlayıcı isteği reddetti (400/401/403/404).
    ProviderRejected,
    /// İstek geçersiz.
    InvalidRequest,
    /// Kimlik doğrulama başarısız.
    Unauthorized,
    /// Kaynak bulunamadı.
    NotFound,
    /// Beklenmeyen iç hata.
    Internal,
}

impl ErrorCode {
    /// Tüm kodlar (eşleme tablolarını ve testleri kapsamlı tutmak için).
    pub const ALL: [Self; 10] = [
        Self::ApprovalDenied,
        Self::DesktopUnavailable,
        Self::LimitReached,
        Self::Halted,
        Self::ProviderUnavailable,
        Self::ProviderRejected,
        Self::InvalidRequest,
        Self::Unauthorized,
        Self::NotFound,
        Self::Internal,
    ];

    /// Tel adı, ör. `"approval_denied"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalDenied => "approval_denied",
            Self::DesktopUnavailable => "desktop_unavailable",
            Self::LimitReached => "limit_reached",
            Self::Halted => "halted",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderRejected => "provider_rejected",
            Self::InvalidRequest => "invalid_request",
            Self::Unauthorized => "unauthorized",
            Self::NotFound => "not_found",
            Self::Internal => "internal",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
