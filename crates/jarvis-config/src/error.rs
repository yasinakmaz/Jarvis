//! Yapılandırma hataları: her biri alan yolu ve düzeltme önerisi taşır (Tasarım 0004).

use std::fmt;

/// Tek bir yapılandırma hatası: nerede (`path`) ve ne (`kind`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// Noktalı alan yolu (`providers.nvidia.model`), ortam değişkeni adı ya da dosya yolu.
    /// Bütün belgeyi ilgilendiren hatalarda boştur.
    pub path: String,
    /// Hatanın türü.
    pub kind: ConfigErrorKind,
}

impl ConfigError {
    /// Yeni hata.
    #[must_use]
    pub fn new(path: impl Into<String>, kind: ConfigErrorKind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.kind)
        } else {
            write!(f, "{}: {}", self.path, self.kind)
        }
    }
}

impl std::error::Error for ConfigError {}

/// Hata türleri. Mesajlar Türkçedir ve kullanıcıya düzeltme yolunu söyler.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigErrorKind {
    /// Dosya okunamadı.
    #[error("dosya okunamadı: {detail}")]
    Io {
        /// İşletim sisteminin hata metni.
        detail: String,
    },
    /// TOML sözdizimi hatası.
    #[error("TOML ayrıştırılamadı: {detail}")]
    Syntax {
        /// Ayrıştırıcının hata metni (satır/sütun içerir).
        detail: String,
    },
    /// Şemada olmayan alan.
    #[error("bilinmeyen alan{}", suggestion_suffix(suggestion.as_deref()))]
    UnknownField {
        /// Yazım hatasıysa en yakın geçerli alan adı.
        suggestion: Option<String>,
    },
    /// Zorunlu alan yok.
    #[error("zorunlu alan eksik")]
    MissingField,
    /// Alanın TOML tipi yanlış.
    #[error("{expected} bekleniyordu, {found} bulundu")]
    WrongType {
        /// Beklenen tip.
        expected: &'static str,
        /// Bulunan tip.
        found: &'static str,
    },
    /// Desteklenmeyen şema sürümü.
    #[error("{found} bu sürümde desteklenmiyor ({expected} bekleniyor)")]
    SchemaVersion {
        /// Dosyadaki sürüm.
        found: i64,
        /// Bu sürümün desteklediği tek şema sürümü.
        expected: i64,
    },
    /// Tamsayı izin verilen aralıkta değil.
    #[error("{found} aralık dışında ({min}..={max})")]
    OutOfRange {
        /// Bulunan değer.
        found: i64,
        /// En küçük izinli değer.
        min: i64,
        /// En büyük izinli değer.
        max: i64,
    },
    /// Metin alanı boş.
    #[error("boş olamaz")]
    Empty,
    /// `IP:port` biçiminde değil.
    #[error("'{value}' geçerli bir IP:port değil (ör. 127.0.0.1:7878)")]
    InvalidAddress {
        /// Verilen değer.
        value: String,
    },
    /// Dinleme adresi loopback değil.
    #[error("{ip} reddedildi; 127.0.0.1 kullanın")]
    NonLoopback {
        /// Verilen IP.
        ip: String,
    },
    /// Dinleme portu 0 (rastgele port; istemciler bulamaz).
    #[error("port 0 olamaz; sabit bir port verin (ör. 7878)")]
    PortZero,
    /// Sağlayıcı adı geçersiz.
    #[error("'{value}' geçersiz sağlayıcı adı (küçük harfle başlar; a-z, 0-9, _ ve -; ≤ 32)")]
    InvalidProviderName {
        /// Verilen ad.
        value: String,
    },
    /// Ortam değişkeni adı geçersiz.
    #[error("'{value}' geçersiz ortam değişkeni adı (harf veya _ ile başlar; A-Z, 0-9, _)")]
    InvalidEnvVarName {
        /// Verilen ad.
        value: String,
    },
    /// Anahtarın ortam değişkeni tanımlı değil ya da boş.
    #[error("{name} tanımlı değil (secrets.env)")]
    EnvVarMissing {
        /// Değişkenin adı.
        name: String,
    },
    /// `base_url` geçersiz.
    #[error("'{value}' reddedildi: {reason}")]
    InvalidBaseUrl {
        /// Verilen değer.
        value: String,
        /// Neden.
        reason: crate::BaseUrlError,
    },
    /// Bilinmeyen yetenek adı.
    #[error("'{value}' bilinmeyen yetenek (tools, vision, streaming)")]
    UnknownCapability {
        /// Verilen ad.
        value: String,
    },
    /// Aynı yetenek iki kez yazılmış.
    #[error("'{value}' iki kez yazılmış")]
    DuplicateCapability {
        /// Tekrarlanan ad.
        value: String,
    },
    /// Rol, tanımlı olmayan bir sağlayıcıyı gösteriyor.
    #[error("'{name}' adlı sağlayıcı tanımlı değil ([providers.{name}] ekleyin)")]
    UnknownProvider {
        /// Gösterilen sağlayıcı adı.
        name: String,
    },
    /// Rolün gerektirdiği yetenek sağlayıcıda bildirilmemiş.
    #[error("'{provider}' sağlayıcısı '{capability}' bildirmiyor")]
    MissingCapability {
        /// Sağlayıcı adı.
        provider: String,
        /// Gereken yetenek.
        capability: crate::Capability,
    },
    /// XDG yolları için `HOME` gerekli ama tanımlı değil ya da boş.
    #[error("tanımlı değil veya boş; XDG yolları çözülemiyor")]
    HomeMissing,
    /// XDG yolu göreli (XDG yalnızca mutlak yol kabul eder).
    #[error("'{value}' göreli; mutlak yol olmalı")]
    RelativePath {
        /// Verilen değer.
        value: String,
    },
}

/// Öneri varsa ` (öneri mi?)` eki.
fn suggestion_suffix(suggestion: Option<&str>) -> String {
    suggestion
        .map(|s| format!(" ({s} mi?)"))
        .unwrap_or_default()
}

/// Bir veya daha çok yapılandırma hatası. Doğrulama ilk hatada durmaz; hepsi toplanır.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigErrors(pub Vec<ConfigError>);

impl ConfigErrors {
    /// Hataları sırayla gezer.
    pub fn iter(&self) -> std::slice::Iter<'_, ConfigError> {
        self.0.iter()
    }

    /// Hata sayısı.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Hata yoksa `true`.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<ConfigError> for ConfigErrors {
    fn from(error: ConfigError) -> Self {
        Self(vec![error])
    }
}

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for error in &self.0 {
            if !first {
                writeln!(f)?;
            }
            first = false;
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigErrors {}

impl<'a> IntoIterator for &'a ConfigErrors {
    type Item = &'a ConfigError;
    type IntoIter = std::slice::Iter<'a, ConfigError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
