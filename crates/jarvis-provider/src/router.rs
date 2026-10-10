//! Rol → sağlayıcı yönlendirmesi (Tasarım 0006). Anahtarlar burada okunur ve `SecretString`
//! olur; bir sağlayıcıyı paylaşan roller aynı hız sınırı kovasını paylaşır.

use std::collections::BTreeMap;
use std::sync::Arc;

use jarvis_config::{Config, EnvLookup, Provider, ProviderName};
use jarvis_types::Clock;
use secrecy::SecretString;

use crate::{ChatModel, Jitter, OpenAiModel, ProviderError, RandomJitter, RawChat, RoleName};

/// `/providers` ucunun satırı; anahtar içermez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    /// Rol.
    pub role: RoleName,
    /// Yapılandırmadaki sağlayıcı adı.
    pub provider: String,
    /// Model kimliği.
    pub model: String,
    /// Uç noktanın tabanı.
    pub base_url: String,
    /// Bildirilen yetenekler.
    pub capabilities: Vec<String>,
}

/// Rol adından modele.
#[derive(Debug)]
pub struct Router {
    roles: BTreeMap<RoleName, Role>,
}

/// Bir role bağlı sağlayıcı: aynı sağlayıcıyı paylaşan roller aynı `Arc`'ı tutar.
#[derive(Debug)]
struct Role {
    model: Arc<OpenAiModel>,
    info: ProviderInfo,
}

impl Router {
    /// Yapılandırmadaki her sağlayıcı için anahtarı okur ve istemciyi kurar.
    ///
    /// # Errors
    ///
    /// Bir sağlayıcının anahtar ortam değişkeni tanımsız/boşsa [`ProviderError::MissingKey`].
    pub fn from_config(
        config: &Config,
        env: &dyn EnvLookup,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, ProviderError> {
        Self::from_config_with_jitter(config, env, clock, Arc::new(RandomJitter))
    }

    /// [`Router::from_config`], jitter kaynağı verilerek (testlerde sabit).
    ///
    /// # Errors
    ///
    /// [`Router::from_config`] ile aynı.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Arc tutamakları devralınır: çağıran kendi payını bırakır"
    )]
    pub fn from_config_with_jitter(
        config: &Config,
        env: &dyn EnvLookup,
        clock: Arc<dyn Clock>,
        jitter: Arc<dyn Jitter>,
    ) -> Result<Self, ProviderError> {
        let mut models: BTreeMap<&ProviderName, Arc<OpenAiModel>> = BTreeMap::new();
        for (name, provider) in &config.providers {
            let key = read_key(name, provider, env)?;
            let model = OpenAiModel::new(provider, key, Arc::clone(&clock), Arc::clone(&jitter));
            models.insert(name, Arc::new(model));
        }
        let assigned = [
            (RoleName::Planner, Some(&config.roles.planner)),
            (RoleName::Fast, Some(&config.roles.fast)),
            (RoleName::Vision, config.roles.vision.as_ref()),
        ];
        let mut roles = BTreeMap::new();
        for (role, name) in assigned {
            let Some(name) = name else { continue };
            let (Some(provider), Some(model)) = (config.provider(name), models.get(name)) else {
                continue;
            };
            let info = ProviderInfo {
                role,
                provider: name.to_string(),
                model: provider.model.clone(),
                base_url: provider.base_url.to_string(),
                capabilities: provider
                    .capabilities
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            };
            roles.insert(
                role,
                Role {
                    model: Arc::clone(model),
                    info,
                },
            );
        }
        Ok(Self { roles })
    }

    /// Rolün modeli; rol atanmamışsa `None`.
    #[must_use]
    pub fn chat(&self, role: RoleName) -> Option<Arc<dyn ChatModel>> {
        self.roles
            .get(&role)
            .map(|bound| Arc::clone(&bound.model) as Arc<dyn ChatModel>)
    }

    /// Rolün passthrough ucu; rol atanmamışsa `None`.
    #[must_use]
    pub fn raw(&self, role: RoleName) -> Option<Arc<dyn RawChat>> {
        self.roles
            .get(&role)
            .map(|bound| Arc::clone(&bound.model) as Arc<dyn RawChat>)
    }

    /// Atanmış roller ([`RoleName::ALL`] sırasıyla).
    #[must_use]
    pub fn describe(&self) -> Vec<ProviderInfo> {
        RoleName::ALL
            .iter()
            .filter_map(|role| self.roles.get(role))
            .map(|bound| bound.info.clone())
            .collect()
    }
}

/// Anahtarı ortamdan okur; tanımsız, boş ya da UTF-8 olmayan değer açık hatadır.
fn read_key(
    name: &ProviderName,
    provider: &Provider,
    env: &dyn EnvLookup,
) -> Result<SecretString, ProviderError> {
    env.get(provider.api_key_env.as_str())
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .map(SecretString::from)
        .ok_or_else(|| ProviderError::MissingKey {
            provider: name.to_string(),
            env_var: provider.api_key_env.to_string(),
        })
}
