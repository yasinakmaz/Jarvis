//! Hız sınırı kovası (Tasarım 0006 test planı, L1). Oracle: `FakeClock` kayıtları.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use jarvis_provider::{ProviderError, TokenBucket};
use jarvis_testkit::FakeClock;
use jarvis_types::Timestamp;
use tokio_util::sync::CancellationToken;

const HALF_MINUTE_STEP: Duration = Duration::from_millis(1500);

#[expect(clippy::unwrap_used, reason = "sabit girdi; geçersizse test düşmeli")]
fn setup(rpm: u32) -> (Arc<FakeClock>, Arc<TokenBucket>) {
    let start = Timestamp::from_unix_millis(1_700_000_000_000).unwrap();
    let clock = Arc::new(FakeClock::new(start));
    let bucket = TokenBucket::new(NonZeroU32::new(rpm).unwrap(), clock.clone());
    (clock, Arc::new(bucket))
}

async fn take(bucket: &TokenBucket, times: u32) {
    let never = CancellationToken::new();
    for _ in 0..times {
        let outcome = tokio::time::timeout(Duration::from_secs(5), bucket.acquire(&never)).await;
        assert_eq!(outcome, Ok(Ok(())), "beklememesi gereken istek bekledi");
    }
}

/// Gerçek zamanlı kısa yoklama: bekleme kaydı görünene kadar.
#[expect(clippy::panic, reason = "yardımcı: koşul sağlanmazsa test düşmeli")]
async fn until_sleeps(clock: &FakeClock, count: usize) {
    for _ in 0..2000 {
        if clock.sleeps().len() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!(
        "{count} bekleme kaydı beklendi, görülen: {:?}",
        clock.sleeps()
    );
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "yardımcı: süre aşımında ya da görev çökerse test düşmeli"
)]
/// Görev 5 sn içinde bitmezse (bozuk kova beklemeyi hiç bitirmez) test düşer.
async fn finish(
    task: tokio::task::JoinHandle<Result<(), ProviderError>>,
) -> Result<(), ProviderError> {
    let joined = tokio::time::timeout(Duration::from_secs(5), task).await;
    joined.expect("görev 5 sn içinde bitmedi").unwrap()
}

fn spawn_acquire(
    bucket: &Arc<TokenBucket>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<Result<(), ProviderError>> {
    let bucket = Arc::clone(bucket);
    tokio::spawn(async move { bucket.acquire(&cancel).await })
}

#[tokio::test]
async fn full_burst_passes_without_sleeping_and_the_next_request_waits_one_interval() {
    let (clock, bucket) = setup(40);
    take(&bucket, 40).await;
    assert!(clock.sleeps().is_empty(), "ilk 40 istek beklemez");

    let waiting = spawn_acquire(&bucket, CancellationToken::new());
    until_sleeps(&clock, 1).await;
    assert_eq!(clock.sleeps(), [HALF_MINUTE_STEP]);
    assert!(
        !waiting.is_finished(),
        "41. istek saat ilerleyene kadar bekler"
    );
    clock.advance(HALF_MINUTE_STEP);
    assert_eq!(finish(waiting).await, Ok(()));
}

#[tokio::test]
async fn tokens_refill_with_time_up_to_the_capacity() {
    let (clock, bucket) = setup(40);
    take(&bucket, 40).await;
    clock.advance(Duration::from_secs(3));
    take(&bucket, 2).await;
    assert!(clock.sleeps().is_empty(), "3 sn = 2 belirteç");

    let waiting = spawn_acquire(&bucket, CancellationToken::new());
    until_sleeps(&clock, 1).await;
    assert_eq!(clock.sleeps(), [HALF_MINUTE_STEP]);
    clock.advance(HALF_MINUTE_STEP);
    finish(waiting).await.unwrap();

    clock.advance(Duration::from_mins(10));
    take(&bucket, 40).await;
    assert_eq!(
        clock.sleeps().len(),
        1,
        "uzun boşluktan sonra kapasite 40'ta kalır"
    );
    let extra = spawn_acquire(&bucket, CancellationToken::new());
    until_sleeps(&clock, 2).await;
    extra.abort();
}

#[tokio::test]
async fn one_request_per_minute_waits_a_full_minute() {
    let (clock, bucket) = setup(1);
    take(&bucket, 1).await;
    let waiting = spawn_acquire(&bucket, CancellationToken::new());
    until_sleeps(&clock, 1).await;
    assert_eq!(clock.sleeps(), [Duration::from_mins(1)]);
    clock.advance(Duration::from_mins(1));
    finish(waiting).await.unwrap();
}

#[tokio::test]
async fn cancelling_a_waiter_returns_cancelled_and_consumes_nothing() {
    let (clock, bucket) = setup(40);
    take(&bucket, 40).await;
    let cancel = CancellationToken::new();
    let waiting = spawn_acquire(&bucket, cancel.clone());
    until_sleeps(&clock, 1).await;
    cancel.cancel();
    assert_eq!(finish(waiting).await, Err(ProviderError::Cancelled));

    clock.advance(HALF_MINUTE_STEP);
    take(&bucket, 1).await;
    assert_eq!(
        clock.sleeps().len(),
        1,
        "iptal belirteç tüketmedi: bekleme yok"
    );
}

#[tokio::test]
async fn an_already_cancelled_token_is_refused_even_when_tokens_are_available() {
    let (clock, bucket) = setup(40);
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(bucket.acquire(&cancel).await, Err(ProviderError::Cancelled));
    take(&bucket, 40).await;
    assert!(
        clock.sleeps().is_empty(),
        "iptal edilen çağrı belirteç tüketmedi"
    );
}

#[tokio::test]
async fn two_waiters_are_served_one_interval_apart_never_together() {
    let (clock, bucket) = setup(40);
    take(&bucket, 40).await;
    let first = spawn_acquire(&bucket, CancellationToken::new());
    let second = spawn_acquire(&bucket, CancellationToken::new());
    until_sleeps(&clock, 2).await;
    clock.advance(HALF_MINUTE_STEP);
    until_sleeps(&clock, 3).await;
    let finished = [first.is_finished(), second.is_finished()];
    assert_eq!(
        finished.iter().filter(|done| **done).count(),
        1,
        "{finished:?}"
    );
    clock.advance(HALF_MINUTE_STEP);
    let (a, b) = (finish(first).await, finish(second).await);
    assert_eq!((a, b), (Ok(()), Ok(())));
}
