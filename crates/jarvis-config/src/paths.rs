//! Uygulama kimliği ve XDG dizinleri (Mimari §8): prod `jarvis`, dev `jarvis-dev`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::{ConfigError, ConfigErrorKind, ConfigErrors, EnvLookup};

/// Çalışma profili. Geliştirme sürümü ayrı dizinlere yazar; üretim verisine dokunmaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// `jarvis`
    Production,
    /// `jarvis-dev`
    Development,
}

impl Profile {
    /// Uygulama kimliği (dizin adı).
    #[must_use]
    pub const fn app_id(self) -> &'static str {
        match self {
            Self::Production => "jarvis",
            Self::Development => "jarvis-dev",
        }
    }
}

/// Jarvis'in dosyaları.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// `$XDG_CONFIG_HOME/<uygulama>/config.toml`
    pub config: PathBuf,
    /// `$XDG_CONFIG_HOME/<uygulama>/secrets.env` (`0600`)
    pub secrets: PathBuf,
    /// `$XDG_CONFIG_HOME/<uygulama>/token` (`0600`)
    pub token: PathBuf,
    /// `$XDG_DATA_HOME/<uygulama>/jarvis.db`
    pub database: PathBuf,
}

impl Paths {
    /// Yolları çözer. `XDG_CONFIG_HOME`/`XDG_DATA_HOME` tanımlı ve boş değilse mutlak olmalıdır
    /// (göreli değer sessizce yok sayılmaz, hata olur); tanımlı değilse `HOME` altındaki
    /// XDG varsayılanları (`.config`, `.local/share`) kullanılır.
    ///
    /// # Errors
    ///
    /// `HOME` gerektiği hâlde yoksa ya da bir yol göreliyse; hataların hepsi birlikte döner.
    pub fn resolve(profile: Profile, env: &dyn EnvLookup) -> Result<Self, ConfigErrors> {
        let mut errors = Vec::new();
        let config_var = xdg_var(env, "XDG_CONFIG_HOME");
        let data_var = xdg_var(env, "XDG_DATA_HOME");
        // HOME yalnızca bir XDG değişkeni eksikse gerekir ve hatası bir kez raporlanır.
        let home = if config_var.is_none() || data_var.is_none() {
            home(env, &mut errors)
        } else {
            None
        };
        let config = base_dir(
            "XDG_CONFIG_HOME",
            config_var,
            home.as_deref(),
            ".config",
            &mut errors,
        );
        let data = base_dir(
            "XDG_DATA_HOME",
            data_var,
            home.as_deref(),
            ".local/share",
            &mut errors,
        );
        match (config, data) {
            (Some(config), Some(data)) => {
                let config = config.join(profile.app_id());
                Ok(Self {
                    config: config.join("config.toml"),
                    secrets: config.join("secrets.env"),
                    token: config.join("token"),
                    database: data.join(profile.app_id()).join("jarvis.db"),
                })
            }
            _ => Err(ConfigErrors(errors)),
        }
    }
}

/// Tanımlı ve boş olmayan değer (XDG: boş değer tanımsız sayılır).
fn xdg_var(env: &dyn EnvLookup, name: &str) -> Option<OsString> {
    env.get(name).filter(|v| !v.is_empty())
}

/// `HOME`; yoksa ya da boşsa hata.
fn home(env: &dyn EnvLookup, errors: &mut Vec<ConfigError>) -> Option<PathBuf> {
    let Some(value) = xdg_var(env, "HOME") else {
        errors.push(ConfigError::new("HOME", ConfigErrorKind::HomeMissing));
        return None;
    };
    absolute("HOME", value.into(), errors)
}

/// `XDG_*` tanımlıysa onu (mutlak olmalı), değilse `HOME/<fallback>`'i döndürür.
fn base_dir(
    var: &str,
    value: Option<OsString>,
    home: Option<&Path>,
    fallback: &str,
    errors: &mut Vec<ConfigError>,
) -> Option<PathBuf> {
    value.map_or_else(
        || home.map(|home| home.join(fallback)),
        |value| absolute(var, value.into(), errors),
    )
}

/// Göreli yolu hata olarak raporlar (XDG: göreli değer geçersizdir; sessizce yok sayılmaz).
fn absolute(var: &str, path: PathBuf, errors: &mut Vec<ConfigError>) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(path);
    }
    let value = path.display().to_string();
    errors.push(ConfigError::new(
        var,
        ConfigErrorKind::RelativePath { value },
    ));
    None
}
