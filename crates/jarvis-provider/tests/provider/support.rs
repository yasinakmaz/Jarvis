//! Sağlayıcı testlerinin ortak yardımcıları.

use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use jarvis_config::{Config, Provider, ProviderName};
use jarvis_provider::{FixedJitter, Jitter, OpenAiModel, RetryNotice, RetryObserver};
use jarvis_testkit::FakeClock;
use jarvis_types::Timestamp;
use secrecy::SecretString;

pub const KEY: &str = "sk-test-SECRET-0123456789";
pub const MODEL: &str = "meta/test-model";

#[expect(clippy::unwrap_used, reason = "sabit girdi; geçersizse test düşmeli")]
pub fn config(base_url: &str, requests_per_minute: u32) -> Config {
    let text = format!(
        r#"schema_version = 1
[providers.test]
base_url = "{base_url}"
api_key_env = "TEST_KEY"
model = "{MODEL}"
capabilities = ["tools", "streaming"]
requests_per_minute = {requests_per_minute}
timeout_seconds = 30
[roles]
planner = "test"
fast = "test"
"#
    );
    let env = std::collections::BTreeMap::from([("TEST_KEY".to_owned(), KEY.to_owned())]);
    jarvis_config::parse(&text, &env).unwrap()
}

/// `planner`/`fast` → `test`; `vision` → `eye` (ikinci uç, kendi model adıyla).
#[expect(clippy::unwrap_used, reason = "sabit girdi; geçersizse test düşmeli")]
pub fn config_with_vision(base_url: &str, vision_url: &str) -> Config {
    let text = format!(
        r#"schema_version = 1
[providers.test]
base_url = "{base_url}"
api_key_env = "TEST_KEY"
model = "{MODEL}"
capabilities = ["tools"]
requests_per_minute = 2
timeout_seconds = 30
[providers.eye]
base_url = "{vision_url}"
api_key_env = "EYE_KEY"
model = "vision/eye-model"
capabilities = ["vision"]
requests_per_minute = 600
timeout_seconds = 30
[roles]
planner = "test"
fast = "test"
vision = "eye"
"#
    );
    let env = env_with_keys();
    jarvis_config::parse(&text, &env).unwrap()
}

pub fn env_with_keys() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        ("TEST_KEY".to_owned(), KEY.to_owned()),
        ("EYE_KEY".to_owned(), "sk-eye-KEY-987".to_owned()),
    ])
}

#[expect(clippy::unwrap_used, reason = "sabit girdi; geçersizse test düşmeli")]
pub fn provider(base_url: &str, requests_per_minute: u32) -> Provider {
    let config = config(base_url, requests_per_minute);
    config
        .provider(&ProviderName::parse("test").unwrap())
        .unwrap()
        .clone()
}

#[expect(clippy::unwrap_used, reason = "sabit girdi; geçersizse test düşmeli")]
pub fn fake_clock() -> Arc<FakeClock> {
    Arc::new(FakeClock::new(
        Timestamp::from_unix_millis(1_700_000_000_000).unwrap(),
    ))
}

pub fn model_with(
    base_url: &str,
    requests_per_minute: u32,
    clock: &Arc<FakeClock>,
    jitter: Arc<dyn Jitter>,
) -> OpenAiModel {
    OpenAiModel::new(
        &provider(base_url, requests_per_minute),
        SecretString::from(KEY.to_owned()),
        clock.clone(),
        jitter,
    )
}

pub fn model(base_url: &str, clock: &Arc<FakeClock>) -> OpenAiModel {
    model_with(base_url, 600, clock, Arc::new(FixedJitter::NONE))
}

/// Yeniden deneme bildirimlerini toplar.
#[derive(Default)]
pub struct Notices(Mutex<Vec<RetryNotice>>);

impl Notices {
    pub fn all(&self) -> Vec<RetryNotice> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl RetryObserver for Notices {
    fn on_retry(&self, notice: RetryNotice) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(notice);
    }
}

/// `future`'ı çalıştırırken saati sürer: her yeni bekleme kaydında saati o kadar ilerletir.
/// 20 sn gerçek zamanda bitmezse test düşer (takılı kalmış bir bekleme).
#[expect(clippy::expect_used, reason = "yardımcı: sabit toplama")]
pub async fn drive<F: Future>(clock: &FakeClock, future: F) -> F::Output {
    let deadline = tokio::time::Instant::now()
        .checked_add(Duration::from_secs(20))
        .expect("20 sn eklenebilir");
    tokio::pin!(future);
    let mut handled = 0;
    loop {
        tokio::select! {
            output = &mut future => return output,
            () = tokio::time::sleep(Duration::from_millis(2)) => {
                assert!(tokio::time::Instant::now() < deadline, "takıldı: {:?}", clock.sleeps());
                let sleeps = clock.sleeps();
                for wait in sleeps.iter().skip(handled) {
                    clock.advance(*wait);
                }
                handled = sleeps.len();
            }
        }
    }
}

/// Gerçek zamanlı kısa yoklama: koşul sağlanana kadar.
#[expect(clippy::panic, reason = "yardımcı: koşul sağlanmazsa test düşmeli")]
pub async fn until(mut condition: impl FnMut() -> bool) {
    for _ in 0..3000 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    panic!("koşul sağlanmadı");
}

/// Yanıtsız kalan bir çağrı testi kilitlemesin: 10 sn gerçek zamanda bitmezse düşer.
/// (Uyuşmayan kaset turu 500 döner; 500 yeniden denenir ve saat ilerlemezse sonsuza dek bekler.)
#[expect(clippy::panic, reason = "yardımcı: süre aşılırsa test düşmeli")]
pub async fn within<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .unwrap_or_else(|_| panic!("çağrı 10 sn içinde bitmedi (kaset uyuşmazlığı olabilir)"))
}
