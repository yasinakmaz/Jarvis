//! `EventBus` (Tasarım 0005 test planı, L2). Oracle: denetim sahtesinin kaydı, abone kuyruğu
//! ve sıra numarası aritmetiği; veriyolunun kendi dönüşüne güvenilmez.

use std::num::NonZeroU16;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use jarvis_events::{
    AuditError, AuditSink, Event, EventBus, EventContext, EventKind, PublishError, Received,
};
use jarvis_types::{BoxFuture, Clock, ErrorCode, EventSeq, RunId, Timestamp, TraceId};

/// Hep aynı anı döndüren saat.
struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }

    fn sleep(&self, _duration: Duration) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

/// Yazılan olayları sırayla tutar; ilk `fail_first` çağrıyı reddeder.
#[derive(Default)]
struct RecordingAudit {
    events: Mutex<Vec<Event>>,
    fail_first: Mutex<usize>,
}

impl RecordingAudit {
    fn failing(times: usize) -> Self {
        Self {
            events: Mutex::default(),
            fail_first: Mutex::new(times),
        }
    }

    fn events(&self) -> Vec<Event> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}

impl AuditSink for RecordingAudit {
    fn append(&self, event: &Event) -> BoxFuture<'_, Result<(), AuditError>> {
        let event = event.clone();
        Box::pin(async move {
            let fail = self
                .fail_first
                .lock()
                .map(|mut remaining| {
                    let fail = *remaining > 0;
                    *remaining = remaining.saturating_sub(1);
                    fail
                })
                .map_err(|e| AuditError(e.to_string()))?;
            if fail {
                return Err(AuditError("disk dolu".to_owned()));
            }
            self.events
                .lock()
                .map_err(|e| AuditError(e.to_string()))?
                .push(event);
            Ok(())
        })
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "test sabiti; ayrıştırılamazsa test düşmeli"
)]
fn noon() -> Timestamp {
    Timestamp::parse_rfc3339("2026-10-09T12:00:00Z").unwrap()
}

fn bus(capacity: u16, audit: &Arc<RecordingAudit>) -> EventBus {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(noon()));
    let audit: Arc<dyn AuditSink> = audit.clone();
    let capacity = NonZeroU16::MIN.saturating_add(capacity.saturating_sub(1));
    EventBus::new(capacity, audit, clock)
}

fn ctx() -> EventContext {
    EventContext {
        run_id: Some(RunId::new()),
        trace_id: TraceId::new(),
    }
}

const fn halt(n: u32) -> EventKind {
    EventKind::Halt { cancelled_runs: n }
}

fn seqs(events: &[Event]) -> Vec<u64> {
    events.iter().map(|e| e.seq.get()).collect()
}

#[tokio::test]
async fn publish_audits_then_broadcasts_the_same_event() {
    let audit = Arc::new(RecordingAudit::default());
    let bus = bus(16, &audit);
    let mut sub = bus.subscribe();
    let ctx = ctx();
    let published = bus.publish(ctx, halt(2)).await.unwrap();

    let expected = Event {
        seq: EventSeq::FIRST,
        at: noon(),
        run_id: ctx.run_id,
        trace_id: ctx.trace_id,
        kind: halt(2),
    };
    assert_eq!(*published, expected);
    assert_eq!(audit.events(), vec![expected.clone()]);
    assert_eq!(sub.recv().await, Some(Received::Event(Arc::new(expected))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sequence_numbers_are_unique_and_increasing_under_concurrency() {
    const TASKS: u32 = 8;
    const PER_TASK: u32 = 50;
    let audit = Arc::new(RecordingAudit::default());
    let bus = Arc::new(bus(1024, &audit));
    let mut sub = bus.subscribe();
    let handles: Vec<_> = (0..TASKS)
        .map(|_| {
            let bus = Arc::clone(&bus);
            tokio::spawn(async move {
                for i in 0..PER_TASK {
                    bus.publish(ctx(), halt(i)).await.unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.await.unwrap();
    }
    let total = u64::from(TASKS * PER_TASK);
    let expected: Vec<u64> = (0..total).collect();
    assert_eq!(seqs(&audit.events()), expected);
    let mut seen = Vec::new();
    for _ in 0..total {
        match sub.recv().await {
            Some(Received::Event(event)) => seen.push(event.seq.get()),
            other => panic!("beklenmeyen: {other:?}"),
        }
    }
    assert_eq!(seen, expected);
}

#[tokio::test]
async fn slow_subscriber_is_told_how_many_it_missed() {
    let audit = Arc::new(RecordingAudit::default());
    let bus = bus(4, &audit);
    let mut sub = bus.subscribe();
    for i in 0..10 {
        bus.publish(ctx(), halt(i)).await.unwrap();
    }
    assert_eq!(sub.recv().await, Some(Received::Lagged { missed: 6 }));
    for expected in 6..10 {
        match sub.recv().await {
            Some(Received::Event(event)) => assert_eq!(event.seq.get(), expected),
            other => panic!("beklenmeyen: {other:?}"),
        }
    }
    assert_eq!(audit.events().len(), 10, "denetim kayıpsızdır");
}

#[tokio::test]
async fn failed_audit_is_not_broadcast_and_leaves_a_gap_not_a_duplicate() {
    let audit = Arc::new(RecordingAudit::failing(1));
    let bus = bus(16, &audit);
    let mut sub = bus.subscribe();
    let error = bus.publish(ctx(), halt(1)).await.unwrap_err();
    assert_eq!(
        error,
        PublishError::Audit(AuditError("disk dolu".to_owned()))
    );
    assert_eq!(error.to_string(), "denetim kaydı yazılamadı: disk dolu");
    let second = bus.publish(ctx(), halt(2)).await.unwrap();
    assert_eq!(second.seq, EventSeq::new(1));
    drop(bus);
    assert_eq!(sub.recv().await, Some(Received::Event(second)));
    assert_eq!(sub.recv().await, None, "başarısız olay kuyruğa girmemeli");
    assert_eq!(seqs(&audit.events()), vec![1]);
}

#[tokio::test]
async fn numbering_continues_from_the_given_start_and_never_wraps() {
    let audit = Arc::new(RecordingAudit::default());
    let resumed = bus(16, &audit).starting_at(EventSeq::new(41));
    assert_eq!(
        resumed.publish(ctx(), halt(0)).await.unwrap().seq,
        EventSeq::new(41)
    );
    assert_eq!(
        resumed.publish(ctx(), halt(0)).await.unwrap().seq,
        EventSeq::new(42)
    );

    let last = bus(16, &audit).starting_at(EventSeq::new(u64::MAX));
    assert_eq!(
        last.publish(ctx(), halt(0)).await.unwrap().seq.get(),
        u64::MAX
    );
    let exhausted = last.publish(ctx(), halt(0)).await.unwrap_err();
    assert_eq!(exhausted, PublishError::SequenceExhausted);
    assert_eq!(seqs(&audit.events()), vec![41, 42, u64::MAX]);
}

#[tokio::test]
async fn secrets_are_masked_before_audit_and_broadcast() {
    let audit = Arc::new(RecordingAudit::default());
    let bus = bus(16, &audit).with_secrets(vec!["nvapi-123".to_owned()]);
    assert!(!format!("{bus:?}").contains("nvapi-123"));
    let mut sub = bus.subscribe();
    let kind = EventKind::RunFailed {
        code: ErrorCode::ProviderRejected,
        message: "anahtar nvapi-123 reddedildi".to_owned(),
    };
    bus.publish(ctx(), kind).await.unwrap();
    let masked = EventKind::RunFailed {
        code: ErrorCode::ProviderRejected,
        message: "anahtar *** reddedildi".to_owned(),
    };
    assert_eq!(
        audit.events().first().map(|e| e.kind.clone()),
        Some(masked.clone())
    );
    match sub.recv().await {
        Some(Received::Event(event)) => assert_eq!(event.kind, masked),
        other => panic!("beklenmeyen: {other:?}"),
    }
}

#[tokio::test]
async fn publishing_without_subscribers_succeeds_and_dropped_bus_ends_subscriptions() {
    let audit = Arc::new(RecordingAudit::default());
    let bus = bus(16, &audit);
    bus.publish(ctx(), halt(0)).await.unwrap();
    let mut sub = bus.subscribe();
    assert!(format!("{bus:?}").contains("subscribers: 1"));
    drop(bus);
    assert_eq!(sub.recv().await, None);
    assert_eq!(audit.events().len(), 1);
}
