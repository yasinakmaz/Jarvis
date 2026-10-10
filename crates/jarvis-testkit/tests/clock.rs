//! `FakeClock` (Tasarım 0011 test planı). Oracle: geleceğin elle yoklanan durumu ve kayıtlı
//! bekleme listesi.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use jarvis_testkit::FakeClock;
use jarvis_types::{Clock, Timestamp};

#[expect(clippy::unwrap_used, reason = "sabit; geçersizse test düşmeli")]
fn start() -> Timestamp {
    Timestamp::parse_rfc3339("2026-10-09T12:00:00Z").unwrap()
}

fn ready(future: &mut Pin<Box<dyn Future<Output = ()> + Send + '_>>) -> bool {
    let mut cx = Context::from_waker(Waker::noop());
    future.as_mut().poll(&mut cx) == Poll::Ready(())
}

#[test]
fn sleep_resolves_only_after_the_clock_reaches_its_deadline() {
    let clock = FakeClock::new(start());
    let mut sleep = clock.sleep(Duration::from_secs(5));
    assert!(!ready(&mut sleep), "zaman ilerlemeden çözülmemeli");
    clock.advance(Duration::from_secs(4));
    assert!(!ready(&mut sleep));
    clock.advance(Duration::from_secs(1));
    assert!(ready(&mut sleep));
    assert_eq!(
        clock.now(),
        start().checked_add(Duration::from_secs(5)).unwrap()
    );
}

#[test]
fn zero_sleep_is_immediately_ready_and_every_sleep_is_recorded() {
    let clock = FakeClock::new(start());
    let mut zero = clock.sleep(Duration::ZERO);
    assert!(ready(&mut zero));
    let _later = clock.sleep(Duration::from_millis(250));
    assert_eq!(clock.sleeps(), [Duration::ZERO, Duration::from_millis(250)]);
    assert_eq!(clock.now(), start(), "sleep saati ilerletmez");
}

#[test]
fn advancing_wakes_only_the_sleepers_that_are_due() {
    let clock = FakeClock::new(start());
    let mut short = clock.sleep(Duration::from_secs(1));
    let mut long = clock.sleep(Duration::from_secs(10));
    drop(clock.sleep(Duration::from_secs(2)));
    clock.advance(Duration::from_secs(3));
    assert!(ready(&mut short));
    assert!(!ready(&mut long));
    clock.advance(Duration::from_secs(7));
    assert!(ready(&mut long));
}

#[test]
fn deadline_is_measured_from_the_moment_sleep_is_called() {
    let clock = FakeClock::new(start());
    clock.advance(Duration::from_secs(100));
    let mut sleep = clock.sleep(Duration::from_secs(5));
    clock.advance(Duration::from_secs(4));
    assert!(!ready(&mut sleep));
    clock.advance(Duration::from_secs(1));
    assert!(ready(&mut sleep));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn advancing_wakes_a_task_waiting_on_another_thread() {
    let clock = Arc::new(FakeClock::new(start()));
    let woke = Arc::new(AtomicBool::new(false));
    let (task_clock, task_woke) = (Arc::clone(&clock), Arc::clone(&woke));
    let (registered_tx, registered_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let sleep = task_clock.sleep(Duration::from_secs(30));
        registered_tx.send(()).unwrap();
        sleep.await;
        task_woke.store(true, Ordering::SeqCst);
    });
    registered_rx.await.unwrap();
    assert!(!woke.load(Ordering::SeqCst));
    clock.advance(Duration::from_secs(30));
    // Gerçek zamanlı üst sınır: uyandırma çalışmazsa test asılı kalmaz, açıkça düşer.
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("advance bekleyen görevi uyandırmadı")
        .unwrap();
    assert!(woke.load(Ordering::SeqCst));
}
