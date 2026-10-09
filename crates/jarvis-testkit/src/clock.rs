//! Elle ilerleyen saat: `sleep` yalnızca [`FakeClock::advance`] ile çözülür.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use jarvis_types::{BoxFuture, Clock, Timestamp};
use tokio::sync::oneshot;

/// Elle ilerleyen saat. Her `sleep` çağrısı kaydedilir (oracle: [`FakeClock::sleeps`]).
///
/// Bekleme süresi `sleep` **çağrıldığı** andan ölçülür (gelecek yoklandığı andan değil).
/// Saat düşerse bekleyen gelecekler çözülür.
#[derive(Debug)]
pub struct FakeClock {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    now: Timestamp,
    sleeps: Vec<Duration>,
    waiters: Vec<(Timestamp, oneshot::Sender<()>)>,
}

impl FakeClock {
    /// `start` anında duran saat.
    #[must_use]
    pub const fn new(start: Timestamp) -> Self {
        Self {
            state: Mutex::new(State {
                now: start,
                sleeps: Vec::new(),
                waiters: Vec::new(),
            }),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Saati ilerletir ve süresi dolan bekleyenleri uyandırır. Desteklenen aralığın (yıl 9999)
    /// ötesine ilerletme yok sayılmaz: saat yerinde kalır ve kimse uyanmaz.
    pub fn advance(&self, by: Duration) {
        let mut state = self.state();
        let Some(now) = state.now.checked_add(by) else {
            return;
        };
        state.now = now;
        let (due, waiting) = std::mem::take(&mut state.waiters)
            .into_iter()
            .partition::<Vec<_>, _>(|(deadline, _)| *deadline <= now);
        state.waiters = waiting;
        drop(state);
        for (_, wake) in due {
            // Alıcı düştüyse (gelecek bırakıldı) uyandırılacak kimse yoktur.
            let _ = wake.send(());
        }
    }

    /// Şimdiye kadar istenen bekleme süreleri, çağrı sırasıyla.
    #[must_use]
    pub fn sleeps(&self) -> Vec<Duration> {
        self.state().sleeps.clone()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        self.state().now
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'_, ()> {
        match self.register(duration) {
            Wait::Ready => Box::pin(std::future::ready(())),
            Wait::Until(woken) => Box::pin(async move {
                // Gönderen yalnızca uyandırırken ya da saat düşerken kapanır.
                let _ = woken.await;
            }),
            Wait::Never => Box::pin(std::future::pending()),
        }
    }
}

/// Bir `sleep` çağrısının sonucu.
enum Wait {
    /// Süre sıfır.
    Ready,
    /// `advance` son ana ulaşınca uyanır.
    Until(oneshot::Receiver<()>),
    /// Son an desteklenen aralığın dışında; hiçbir zaman gelmez.
    Never,
}

impl FakeClock {
    /// Beklemeyi kaydeder; kilit dönüşten önce bırakılır.
    fn register(&self, duration: Duration) -> Wait {
        let mut state = self.state();
        state.sleeps.push(duration);
        let now = state.now;
        let wait = match now.checked_add(duration) {
            Some(deadline) if deadline <= now => Wait::Ready,
            Some(deadline) => {
                let (wake, woken) = oneshot::channel();
                state.waiters.push((deadline, wake));
                Wait::Until(woken)
            }
            None => Wait::Never,
        };
        drop(state);
        wait
    }
}
