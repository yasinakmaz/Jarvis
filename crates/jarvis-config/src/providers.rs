//! `[providers.<ad>]` tabloları.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use toml::Value;

use crate::reader::{Errors, Section, expect_array, expect_str, expect_table, join};
use crate::{
    BaseUrl, Capability, ConfigError, ConfigErrorKind, EnvLookup, EnvVarName, Provider,
    ProviderName,
};

const PROVIDER_FIELDS: &[&str] = &[
    "base_url",
    "api_key_env",
    "model",
    "capabilities",
    "requests_per_minute",
    "timeout_seconds",
];

/// Geçerli sağlayıcılar ve dosyada adı geçen her sağlayıcı (geçersiz olanlar dahil).
#[derive(Default)]
pub struct Providers {
    /// Tüm kuralları geçen sağlayıcılar.
    pub valid: BTreeMap<ProviderName, Provider>,
    /// Dosyada adı geçen her sağlayıcı.
    pub declared: BTreeSet<String>,
}

pub fn providers(root: &Section<'_>, env: &dyn EnvLookup, errors: &mut Errors) -> Providers {
    let mut out = Providers::default();
    let Some(value) = root.required("providers", errors) else {
        return out;
    };
    let Some(table) = expect_table(value, &root.path("providers"), errors) else {
        return out;
    };
    let mut entries: Vec<(&String, &Value)> = table.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (raw, value) in entries {
        out.declared.insert(raw.clone());
        let path = join("providers", raw);
        let name = ProviderName::parse(raw);
        if name.is_none() {
            let kind = ConfigErrorKind::InvalidProviderName { value: raw.clone() };
            errors.push(ConfigError::new(&path, kind));
        }
        let Some(table) = expect_table(value, &path, errors) else {
            continue;
        };
        let section = Section::open(path, table, PROVIDER_FIELDS, errors);
        if let (Some(name), Some(provider)) = (name, provider(&section, env, errors)) {
            out.valid.insert(name, provider);
        }
    }
    out
}

fn provider(section: &Section<'_>, env: &dyn EnvLookup, errors: &mut Errors) -> Option<Provider> {
    let base_url = section.required_str("base_url", errors).and_then(|value| {
        BaseUrl::parse(value)
            .map_err(|reason| {
                let kind = ConfigErrorKind::InvalidBaseUrl {
                    value: value.to_owned(),
                    reason,
                };
                errors.push(ConfigError::new(section.path("base_url"), kind));
            })
            .ok()
    });
    let api_key_env = section
        .required_str("api_key_env", errors)
        .and_then(|value| api_key_env(value, &section.path("api_key_env"), env, errors));
    let model = section.required_str("model", errors).and_then(|value| {
        if value.is_empty() {
            errors.push(ConfigError::new(
                section.path("model"),
                ConfigErrorKind::Empty,
            ));
            return None;
        }
        Some(value.to_owned())
    });
    let capabilities = section
        .required("capabilities", errors)
        .and_then(|value| capabilities(value, &section.path("capabilities"), errors));
    let requests_per_minute = section.required_positive("requests_per_minute", errors);
    let timeout = section.required_positive("timeout_seconds", errors);
    Some(Provider {
        base_url: base_url?,
        api_key_env: api_key_env?,
        model: model?,
        capabilities: capabilities?,
        requests_per_minute: requests_per_minute?,
        timeout: Duration::from_secs(u64::from(timeout?.get())),
    })
}

/// Adı doğrular ve değişkenin ortamda tanımlı ve boş olmadığını denetler (değer okunmaz).
fn api_key_env(
    value: &str,
    path: &str,
    env: &dyn EnvLookup,
    errors: &mut Errors,
) -> Option<EnvVarName> {
    let Some(name) = EnvVarName::parse(value) else {
        let kind = ConfigErrorKind::InvalidEnvVarName {
            value: value.to_owned(),
        };
        errors.push(ConfigError::new(path, kind));
        return None;
    };
    if env.get(name.as_str()).is_none_or(|v| v.is_empty()) {
        let kind = ConfigErrorKind::EnvVarMissing {
            name: value.to_owned(),
        };
        errors.push(ConfigError::new(path, kind));
        return None;
    }
    Some(name)
}

fn capabilities(value: &Value, path: &str, errors: &mut Errors) -> Option<BTreeSet<Capability>> {
    let items = expect_array(value, path, errors)?;
    let mut set = BTreeSet::new();
    let before = errors.len();
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let Some(text) = expect_str(item, &item_path, errors) else {
            continue;
        };
        let kind = match Capability::parse(text) {
            Some(capability) if set.insert(capability) => continue,
            Some(_) => ConfigErrorKind::DuplicateCapability {
                value: text.to_owned(),
            },
            None => ConfigErrorKind::UnknownCapability {
                value: text.to_owned(),
            },
        };
        errors.push(ConfigError::new(item_path, kind));
    }
    (errors.len() == before).then_some(set)
}
