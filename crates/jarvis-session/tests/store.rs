//! Oturum, mesaj, koşu ve denetim işlemleri (Tasarım 0007 test planı, L2/L5a). Oracle: elle
//! kurulmuş beklenen değerler, ayarlanabilir saat ve bağımsız `rusqlite` sorguları.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use jarvis_session::{
    AuditRow, Page, RunOutcome, RunRecord, SessionRepo, SessionSummary, Store, StoreError,
};
use jarvis_types::{
    BoxFuture, Clock, ErrorCode, EventSeq, Message, Role, RunId, SessionId, Timestamp, TraceId,
};
use rusqlite::Connection;

/// Testin ayarladığı saat (Unix milisaniyesi).
#[derive(Default)]
struct SetClock(AtomicI64);

impl SetClock {
    fn set(&self, millis: i64) {
        self.0.store(millis, Ordering::SeqCst);
    }
}

impl Clock for SetClock {
    #[expect(clippy::unwrap_used, reason = "testler aralık içi değer kullanır")]
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(self.0.load(Ordering::SeqCst)).unwrap()
    }

    fn sleep(&self, _duration: Duration) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    clock: Arc<SetClock>,
    store: Store,
}

#[expect(clippy::unwrap_used, reason = "kurulum başarısızsa test düşmeli")]
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jarvis.db");
    let clock = Arc::new(SetClock::default());
    let store = Store::open(&path, clock.clone()).unwrap();
    Fixture {
        _dir: dir,
        path,
        clock,
        store,
    }
}

#[expect(clippy::unwrap_used, reason = "sabit; geçersizse test düşmeli")]
fn at(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(millis).unwrap()
}

fn text(role: Role, body: &str) -> Message {
    Message::text(role, body)
}

fn conn(path: &Path) -> rusqlite::Result<Connection> {
    Connection::open(path)
}

#[tokio::test]
async fn session_lifecycle_keeps_summary_and_messages_in_step() {
    let f = fixture();
    f.clock.set(1_000);
    let id = f.store.create_session().await.unwrap();
    let empty = SessionSummary {
        id,
        created_at: at(1_000),
        updated_at: at(1_000),
        message_count: 0,
    };
    assert_eq!(f.store.session(id).await.unwrap(), Some(empty));

    f.clock.set(2_000);
    let batch = vec![text(Role::User, "merhaba"), text(Role::Assistant, "selam")];
    f.store
        .append_messages(id, RunId::new(), batch.clone())
        .await
        .unwrap();
    let summary = f.store.session(id).await.unwrap().unwrap();
    assert_eq!((summary.updated_at, summary.message_count), (at(2_000), 2));
    assert_eq!(summary.created_at, at(1_000));
    assert_eq!(f.store.messages(id).await.unwrap(), batch);

    assert!(f.store.delete_session(id).await.unwrap());
    assert_eq!(f.store.session(id).await.unwrap(), None);
    assert!(!f.store.delete_session(id).await.unwrap());
    assert_eq!(
        f.store.messages(id).await,
        Err(StoreError::UnknownSession(id))
    );
    let orphans: i64 = conn(&f.path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(orphans, 0, "mesajlar oturumla silinir");
}

#[tokio::test]
async fn appending_to_an_unknown_session_inserts_nothing() {
    let f = fixture();
    let ghost = SessionId::new();
    let result = f
        .store
        .append_messages(ghost, RunId::new(), vec![text(Role::User, "x")])
        .await;
    assert_eq!(result, Err(StoreError::UnknownSession(ghost)));
    assert_eq!(
        result.unwrap_err().to_string(),
        format!("oturum yok: {ghost}")
    );
    let rows: i64 = conn(&f.path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn sessions_are_listed_most_recently_updated_first() {
    let f = fixture();
    let mut ids = Vec::new();
    for millis in [1_000, 2_000, 3_000] {
        f.clock.set(millis);
        ids.push(f.store.create_session().await.unwrap());
    }
    f.clock.set(4_000);
    f.store
        .append_messages(ids[0], RunId::new(), vec![text(Role::User, "x")])
        .await
        .unwrap();

    let listed: Vec<SessionId> = f
        .store
        .list_sessions(Page::FIRST)
        .await
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(listed, [ids[0], ids[2], ids[1]]);
    let second = f
        .store
        .list_sessions(Page::new(1, 1).unwrap())
        .await
        .unwrap();
    assert_eq!(second.iter().map(|s| s.id).collect::<Vec<_>>(), [ids[2]]);
    assert_eq!(second.first().map(|s| s.message_count), Some(0));
    assert!(
        f.store
            .list_sessions(Page::new(5, 10).unwrap())
            .await
            .unwrap()
            .is_empty()
    );
}

#[test]
fn page_limits_are_validated() {
    assert_eq!(Page::new(0, 0), None);
    assert_eq!(Page::new(0, Page::MAX_LIMIT + 1), None);
    let page = Page::new(7, Page::MAX_LIMIT).unwrap();
    assert_eq!((page.offset(), page.limit()), (7, 100));
    assert_eq!((Page::FIRST.offset(), Page::FIRST.limit()), (0, 100));
    assert_eq!(Page::new(0, 1).map(Page::limit), Some(1));
}

#[tokio::test]
async fn timestamps_are_stored_with_millisecond_resolution() {
    let f = fixture();
    let clock = Arc::new(FixedClock(
        Timestamp::parse_rfc3339("2026-10-09T12:00:00.0015Z").unwrap(),
    ));
    drop(f.store);
    let store = Store::open(&f.path, clock).unwrap();
    let id = store.create_session().await.unwrap();
    let summary = store.session(id).await.unwrap().unwrap();
    assert_eq!(
        summary.created_at,
        Timestamp::parse_rfc3339("2026-10-09T12:00:00.001Z").unwrap()
    );
}

struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }

    fn sleep(&self, _duration: Duration) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

type RunRow = (
    String,
    Option<String>,
    String,
    i64,
    Option<i64>,
    Option<String>,
    String,
);

fn run_row(path: &Path, id: RunId) -> rusqlite::Result<RunRow> {
    conn(path)?.query_row("SELECT * FROM runs WHERE id = ?1", [id.to_string()], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
            r.get(6)?,
        ))
    })
}

#[tokio::test]
async fn run_records_are_inserted_then_only_their_end_is_updated() {
    let f = fixture();
    let session = f.store.create_session().await.unwrap();
    let mut run = RunRecord {
        id: RunId::new(),
        session_id: Some(session),
        trace_id: TraceId::new(),
        started_at: at(10),
        finished_at: None,
        outcome: None,
        agent_version: "0.1.0+abc".to_owned(),
    };
    f.store.record_run(run.clone()).await.unwrap();
    let started = run_row(&f.path, run.id).unwrap();
    assert_eq!(
        started,
        (
            run.id.to_string(),
            Some(session.to_string()),
            run.trace_id.to_string(),
            10,
            None,
            None,
            "0.1.0+abc".to_owned()
        )
    );

    run.finished_at = Some(at(20));
    run.outcome = Some(RunOutcome::Failed(ErrorCode::Halted));
    run.agent_version = "değişmemeli".to_owned();
    run.started_at = at(99);
    f.store.record_run(run.clone()).await.unwrap();
    let finished = run_row(&f.path, run.id).unwrap();
    assert_eq!(
        (finished.3, finished.4, finished.5.as_deref()),
        (10, Some(20), Some("halted"))
    );
    assert_eq!(finished.6, "0.1.0+abc");

    let ephemeral = RunRecord {
        id: RunId::new(),
        session_id: None,
        outcome: Some(RunOutcome::Finished),
        finished_at: Some(at(30)),
        ..run
    };
    f.store.record_run(ephemeral.clone()).await.unwrap();
    let row = run_row(&f.path, ephemeral.id).unwrap();
    assert_eq!((row.1, row.5.as_deref()), (None, Some("finished")));
}

#[test]
fn run_outcomes_use_wire_names() {
    assert_eq!(RunOutcome::Finished.as_str(), "finished");
    for code in ErrorCode::ALL {
        assert_eq!(RunOutcome::Failed(code).as_str(), code.as_str());
    }
}

fn audit(seq: u64) -> AuditRow {
    AuditRow {
        seq: EventSeq::new(seq),
        at: at(5),
        run_id: Some(RunId::new()),
        trace_id: TraceId::new(),
        kind: "run.started".to_owned(),
        payload_json: "{\"type\":\"run.started\"}".to_owned(),
    }
}

#[tokio::test]
async fn audit_numbering_resumes_after_the_last_row_and_rejects_duplicates() {
    let f = fixture();
    assert_eq!(f.store.next_audit_seq().await.unwrap(), EventSeq::FIRST);
    f.store.append_audit(audit(0)).await.unwrap();
    f.store.append_audit(audit(5)).await.unwrap();
    assert_eq!(f.store.next_audit_seq().await.unwrap(), EventSeq::new(6));
    assert!(matches!(
        f.store.append_audit(audit(5)).await,
        Err(StoreError::Database(_))
    ));
    assert_eq!(
        f.store.append_audit(audit(u64::MAX)).await,
        Err(StoreError::OutOfRange("audit seq"))
    );
    let row = audit(7);
    f.store.append_audit(row.clone()).await.unwrap();
    let run_id: Option<String> = conn(&f.path)
        .unwrap()
        .query_row("SELECT run_id FROM audit_events WHERE seq = 7", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(run_id, row.run_id.map(|r| r.to_string()));
}

#[tokio::test]
async fn concurrent_appends_stay_atomic_and_contiguous() {
    let f = fixture();
    let id = f.store.create_session().await.unwrap();
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let store = f.store.clone();
            tokio::spawn(async move {
                let batch = vec![
                    text(Role::User, &format!("{i}-a")),
                    text(Role::User, &format!("{i}-b")),
                ];
                store.append_messages(id, RunId::new(), batch).await
            })
        })
        .collect();
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
    let all = f.store.messages(id).await.unwrap();
    assert_eq!(all.len(), 200);
    for pair in all.chunks(2) {
        let [a, b] = pair else {
            panic!("tek sayıda mesaj")
        };
        let first = a.joined_text();
        assert_eq!(
            b.joined_text(),
            first.replace("-a", "-b"),
            "toplu ekleme bölünmemeli"
        );
    }
}

#[tokio::test]
async fn store_works_behind_the_session_repo_trait() {
    let f = fixture();
    let repo: Arc<dyn SessionRepo> = Arc::new(f.store.clone());
    let id = repo.create_session().await.unwrap();
    repo.append_messages(id, RunId::new(), vec![text(Role::User, "a")])
        .await
        .unwrap();
    assert_eq!(
        repo.messages(id).await.unwrap(),
        vec![text(Role::User, "a")]
    );
    let run = RunRecord {
        id: RunId::new(),
        session_id: Some(id),
        trace_id: TraceId::new(),
        started_at: at(1),
        finished_at: None,
        outcome: None,
        agent_version: "v".to_owned(),
    };
    repo.record_run(run.clone()).await.unwrap();
    assert_eq!(run_row(&f.path, run.id).unwrap().6, "v");
    assert!(format!("{:?}", f.store).starts_with("Store"));
}
