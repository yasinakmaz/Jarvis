//! Üstel geri çekilme: `min(32 s, 0,5 s · 2^n) + jitter` (Tasarım 0006).

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Bir istek için en çok deneme sayısı.
pub const MAX_ATTEMPTS: u32 = 5;

/// İlk başarısızlıktan sonraki bekleme.
pub const BASE_DELAY: Duration = Duration::from_millis(500);

/// En uzun taban bekleme.
pub const MAX_DELAY: Duration = Duration::from_secs(32);

/// Beklemeye eklenen rastgelelik kaynağı. Testte sabitlenir.
pub trait Jitter: Send + Sync {
    /// `0..=max` aralığında bir süre.
    fn jitter(&self, max: Duration) -> Duration;
}

/// Sabit jitter (en çok `max`): deterministik testler ve jitter'sız çalışma için.
#[derive(Debug, Clone, Copy)]
pub struct FixedJitter(pub Duration);

impl FixedJitter {
    /// Jitter yok: bekleme tabana eşittir.
    pub const NONE: Self = Self(Duration::ZERO);
}

impl Jitter for FixedJitter {
    fn jitter(&self, max: Duration) -> Duration {
        self.0.min(max)
    }
}

/// Süreç başına rastgele tohumlu jitter (yalnızca standart kütüphane).
#[derive(Debug, Default)]
pub struct RandomJitter;

impl Jitter for RandomJitter {
    fn jitter(&self, max: Duration) -> Duration {
        static DRAWS: AtomicU64 = AtomicU64::new(0);
        let span = u64::try_from(max.as_nanos())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(DRAWS.fetch_add(1, Ordering::Relaxed));
        Duration::from_nanos(hasher.finish().checked_rem(span).unwrap_or(0))
    }
}

/// `failed` başarısız denemeden sonraki taban bekleme (`failed` ≥ 1; 0, 1 sayılır).
#[must_use]
pub fn base_delay(failed: u32) -> Duration {
    // 2^6 · 0,5 s = 32 s: daha büyük üslerin sonucu zaten sınırdadır.
    let doublings = failed.saturating_sub(1).min(6);
    BASE_DELAY
        .saturating_mul(2_u32.saturating_pow(doublings))
        .min(MAX_DELAY)
}

/// Taban bekleme + en çok tabanın dörtte biri kadar jitter.
#[must_use]
pub fn delay_for(failed: u32, jitter: &dyn Jitter) -> Duration {
    let base = base_delay(failed);
    let bound = base.checked_div(4).unwrap_or_default();
    base.saturating_add(jitter.jitter(bound).min(bound))
}

#[cfg(test)]
#[path = "backoff_tests.rs"]
mod tests;
