//! TOML yapılandırma şeması, yükleme ve doğrulama (Tasarım 0004).
//!
//! Geçersiz yapılandırma başlangıçta durur: [`load`] ya tamamen doğrulanmış bir [`Config`]
//! ya da hataların **hepsini** alan yollarıyla döndürür. Anahtarların değerleri burada
//! okunmaz; yalnızca ortam değişkeninin adı ve varlığı denetlenir.

pub mod env;
pub mod error;
pub mod model;
pub mod names;
pub mod paths;
mod providers;
mod reader;
pub mod url;
mod validate;

use std::path::Path;

pub use env::{EnvLookup, SystemEnv};
pub use error::{ConfigError, ConfigErrorKind, ConfigErrors};
pub use model::{Config, Limits, Provider, Roles, SCHEMA_VERSION, Server};
pub use names::{Capability, EnvVarName, ProviderName};
pub use paths::{Paths, Profile};
pub use url::{BaseUrl, BaseUrlError};

/// `config.toml` metnini ayrıştırır ve tamamen doğrular. Saftır: dosya sistemine dokunmaz.
///
/// # Errors
///
/// Söz dizimi hatası, bilinmeyen/eksik alan ya da kural ihlali; hepsi birlikte döner.
pub fn parse(text: &str, env: &dyn EnvLookup) -> Result<Config, ConfigErrors> {
    validate::parse(text, env)
}

/// Dosyayı okur ve [`parse`] eder.
///
/// # Errors
///
/// Dosya okunamazsa (yol `path` alanında) ya da [`parse`] başarısızsa.
pub fn load(path: &Path, env: &dyn EnvLookup) -> Result<Config, ConfigErrors> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        ConfigError::new(
            path.display().to_string(),
            ConfigErrorKind::Io {
                detail: e.to_string(),
            },
        )
    })?;
    parse(&text, env)
}
