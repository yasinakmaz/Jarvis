//! Doğrulanmış adlar: sağlayıcı adı, ortam değişkeni adı ve yetenekler.

use std::fmt;

/// `[providers.<ad>]` tablosunun adı: `^[a-z][a-z0-9_-]{0,31}$`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderName(String);

impl ProviderName {
    /// En uzun ad.
    pub const MAX_LEN: usize = 32;

    /// Adı doğrular; geçersizse `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let mut chars = value.chars();
        let head_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase());
        let tail_ok =
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        (head_ok && tail_ok && value.len() <= Self::MAX_LEN).then(|| Self(value.to_owned()))
    }

    /// Ad.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProviderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Ortam değişkeni adı: `^[A-Za-z_][A-Za-z0-9_]{0,127}$`. Değer değil, yalnızca ad.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// En uzun ad.
    pub const MAX_LEN: usize = 128;

    /// Adı doğrular; geçersizse `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let mut chars = value.chars();
        let head_ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        let tail_ok = chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        (head_ok && tail_ok && value.len() <= Self::MAX_LEN).then(|| Self(value.to_owned()))
    }

    /// Ad.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EnvVarName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Sağlayıcının bildirdiği yetenek. Bildirime dayanır; ağdan doğrulanmaz (Tasarım 0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Capability {
    /// Araç çağrısı (`tools`). `planner` rolü için şart.
    Tools,
    /// Görüntü girdisi (`vision`). `vision` rolü için şart.
    Vision,
    /// Akışlı yanıt (`streaming`).
    Streaming,
}

impl Capability {
    /// Tüm yetenekler.
    pub const ALL: [Self; 3] = [Self::Tools, Self::Vision, Self::Streaming];

    /// Dosyadaki adı.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Vision => "vision",
            Self::Streaming => "streaming",
        }
    }

    /// Dosyadaki addan çözer.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == value)
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
