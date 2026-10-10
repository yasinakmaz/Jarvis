//! Sağlayıcı başına istek hızı sınırı: GCRA biçiminde belirteç kovası (Tasarım 0006).
//!
//! Dakikada `n` istek → her `60/n` saniyede bir belirteç, kova kapasitesi `n`. Saat enjekte
//! edilir; bekleme `Clock::sleep` ile yapılır ve iptal dinlenir.

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use jarvis_types::{Clock, Timestamp};
use tokio_util::sync::CancellationToken;

use crate::ProviderError;

/// Dakikadaki istek sayısını sınırlayan kova.
pub struct TokenBucket {
    interval: Duration,
    burst: Duration,
    clock: Arc<dyn Clock>,
    next: Mutex<Timestamp>,
}

impl TokenBucket {
    /// Dolu bir kova: ilk `requests_per_minute` istek beklemeden geçer.
    #[must_use]
    pub fn new(requests_per_minute: NonZeroU32, clock: Arc<dyn Clock>) -> Self {
        let rate = requests_per_minute.get();
        let minute = Duration::from_mins(1);
        let interval = minute.checked_div(rate).unwrap_or(minute);
        let now = clock.now();
        Self {
            interval,
            burst: interval.saturating_mul(rate.saturating_sub(1)),
            clock,
            next: Mutex::new(now),
        }
    }

    /// Bir belirteç alır; yoksa kalan süre kadar bekler.
    ///
    /// # Errors
    ///
    /// Beklerken (ya da başlamadan önce) `cancel` tetiklenirse [`ProviderError::Cancelled`];
    /// iptal edilen çağrı belirteç tüketmez.
    pub async fn acquire(&self, cancel: &CancellationToken) -> Result<(), ProviderError> {
        loop {
            if cancel.is_cancelled() {
                return Err(ProviderError::Cancelled);
            }
            let Err(wait) = self.try_take(self.clock.now()) else {
                return Ok(());
            };
            tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(ProviderError::Cancelled),
                () = self.clock.sleep(wait) => {}
            }
            // Sıfır süreli bekleme hemen çözülür; yürütücüye dönmeden dönmek diğer görevleri
            // (ve zaman aşımlarını) aç bırakırdı.
            tokio::task::yield_now().await;
        }
    }

    /// `Ok`: belirteç alındı. `Err(bekleme)`: belirteç `bekleme` sonra çıkar.
    ///
    /// `next`, kovanın "kuramsal varış" anıdır (GCRA): her kabul onu bir aralık ileri iter;
    /// `burst`'ten fazla öndeyse istek bekler.
    fn try_take(&self, now: Timestamp) -> Result<(), Duration> {
        let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
        let theoretical = (*next).max(now);
        let ahead = theoretical.saturating_duration_since(now);
        let outcome = if ahead > self.burst {
            Err(ahead.saturating_sub(self.burst))
        } else {
            *next = theoretical
                .checked_add(self.interval)
                .unwrap_or(theoretical);
            Ok(())
        };
        drop(next);
        outcome
    }
}
