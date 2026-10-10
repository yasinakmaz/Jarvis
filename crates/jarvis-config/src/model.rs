//! Doğrulanmış yapılandırma: bu tiplerden biri elinizdeyse kuralların hepsi geçmiştir.

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::time::Duration;

use crate::{BaseUrl, Capability, EnvVarName, ProviderName};

/// Desteklenen tek şema sürümü.
pub const SCHEMA_VERSION: i64 = 1;

/// Tüm yapılandırma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// HTTP sunucusu.
    pub server: Server,
    /// Ajan döngüsü sınırları.
    pub limits: Limits,
    /// Sağlayıcılar, ada göre.
    pub providers: BTreeMap<ProviderName, Provider>,
    /// Rol → sağlayıcı ataması; her rol var olan bir sağlayıcıyı gösterir.
    pub roles: Roles,
}

impl Config {
    /// Rolün sağlayıcısı. Doğrulama her rolün var olduğunu garanti eder.
    #[must_use]
    pub fn provider(&self, name: &ProviderName) -> Option<&Provider> {
        self.providers.get(name)
    }
}

/// `[server]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Server {
    /// Dinleme adresi; her zaman loopback, port ≠ 0.
    pub listen: SocketAddr,
}

impl Server {
    /// `[server]` yazılmazsa kullanılan adres (belgelenmiş varsayılan, Tasarım 0004).
    pub const DEFAULT_LISTEN: &'static str = "127.0.0.1:7878";
}

/// `[limits]`: koşu başına sınırlar (Mimari §5). Alan yazılmazsa varsayılanlar geçerlidir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// En çok adım.
    pub max_steps: NonZeroU32,
    /// En uzun koşu süresi (saniye çözünürlüğünde).
    pub max_run: Duration,
    /// Aynı eylem + aynı sonucun ardışık tekrar sınırı.
    pub repeat_threshold: NonZeroU32,
}

impl Limits {
    /// `max_steps` varsayılanı.
    pub const DEFAULT_MAX_STEPS: u32 = 25;
    /// `max_run_seconds` varsayılanı.
    pub const DEFAULT_MAX_RUN_SECONDS: u32 = 600;
    /// `repeat_threshold` varsayılanı.
    pub const DEFAULT_REPEAT_THRESHOLD: u32 = 3;
}

/// `[providers.<ad>]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    /// `OpenAI` uyumlu uç noktanın tabanı.
    pub base_url: BaseUrl,
    /// Anahtarı taşıyan ortam değişkeninin adı (değer asla dosyada değil).
    pub api_key_env: EnvVarName,
    /// Model kimliği.
    pub model: String,
    /// Bildirilen yetenekler.
    pub capabilities: BTreeSet<Capability>,
    /// Dakikadaki en çok istek (yerel hız sınırı kovası).
    pub requests_per_minute: NonZeroU32,
    /// İstek zaman aşımı (saniye çözünürlüğünde, > 0).
    pub timeout: Duration,
}

/// `[roles]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roles {
    /// Planlayıcı; `tools` yeteneği şart.
    pub planner: ProviderName,
    /// Hızlı yardımcı model.
    pub fast: ProviderName,
    /// Görüntü modeli (M1'de isteğe bağlı); atanırsa `vision` yeteneği şart.
    pub vision: Option<ProviderName>,
}
