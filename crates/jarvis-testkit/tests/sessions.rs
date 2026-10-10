//! `MemorySessions` (Tasarım 0011). Oracle: aynı senaryonun gerçek `jarvis_session::Store`
//! üzerindeki sonucu (sözleşme testi) ve elle kurulmuş beklenen koşu kayıtları.

use std::sync::Arc;

use jarvis_session::{RunOutcome, RunRecord, SessionRepo, Store, StoreError};
use jarvis_testkit::{FakeClock, MemorySessions};
use jarvis_types::{ErrorCode, Message, Role, RunId, SessionId, Timestamp, TraceId};

#[expect(clippy::unwrap_used, reason = "sabit; geçersizse test düşmeli")]
fn at(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(millis).unwrap()
}

type Transcript = Vec<Result<Vec<Message>, StoreError>>;

/// Her adımın gözlenebilir sonucu; kimlikler dışarıdan verildiği için iki uygulamada aynıdır.
async fn scenario(repo: &dyn SessionRepo, ghost: SessionId) -> Result<Transcript, StoreError> {
    let a = repo.create_session().await?;
    let b = repo.create_session().await?;
    assert_ne!(a, b);
    let user = Message::text(Role::User, "merhaba");
    let reply = Message::text(Role::Assistant, "selam");
    let mut out = vec![repo.messages(a).await];
    let appended = repo
        .append_messages(a, RunId::new(), vec![user.clone(), reply])
        .await;
    out.push(appended.map(|()| Vec::new()));
    let appended = repo.append_messages(a, RunId::new(), vec![user]).await;
    out.push(appended.map(|()| Vec::new()));
    out.push(repo.messages(a).await);
    out.push(repo.messages(b).await);
    out.push(repo.messages(ghost).await);
    let ghost_append = repo.append_messages(ghost, RunId::new(), Vec::new()).await;
    out.push(ghost_append.map(|()| Vec::new()));
    Ok(out)
}

#[tokio::test]
async fn memory_sessions_behave_like_the_sqlite_store() {
    let ghost = SessionId::new();
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(FakeClock::new(at(0)));
    let store = Store::open(&dir.path().join("jarvis.db"), clock).unwrap();
    let expected = scenario(&store, ghost).await.unwrap();
    let actual = scenario(&MemorySessions::new(), ghost).await.unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        expected.get(3).map(|r| r.as_ref().map(Vec::len)),
        Some(Ok(3))
    );
}

fn run(id: RunId) -> RunRecord {
    RunRecord {
        id,
        session_id: None,
        trace_id: TraceId::new(),
        started_at: at(10),
        finished_at: None,
        outcome: None,
        agent_version: "v1".to_owned(),
    }
}

#[tokio::test]
async fn runs_are_recorded_and_only_their_end_is_updated() {
    let repo = MemorySessions::new();
    let first = run(RunId::new());
    let second = run(RunId::new());
    repo.record_run(first.clone()).await.unwrap();
    repo.record_run(second.clone()).await.unwrap();
    let finished = RunRecord {
        finished_at: Some(at(20)),
        outcome: Some(RunOutcome::Failed(ErrorCode::LimitReached)),
        started_at: at(99),
        agent_version: "değişmemeli".to_owned(),
        ..first.clone()
    };
    repo.record_run(finished).await.unwrap();
    let expected_first = RunRecord {
        finished_at: Some(at(20)),
        outcome: Some(RunOutcome::Failed(ErrorCode::LimitReached)),
        ..first
    };
    assert_eq!(repo.runs(), [expected_first, second]);
}

#[tokio::test]
async fn injected_failure_applies_to_every_call_until_cleared() {
    let repo = MemorySessions::new();
    let id = repo.create_session().await.unwrap();
    repo.fail_with(Some(StoreError::Database("disk dolu".to_owned())));
    let failure = Err(StoreError::Database("disk dolu".to_owned()));
    assert_eq!(repo.create_session().await.map(|_| ()), failure);
    assert_eq!(repo.messages(id).await.map(|_| ()), failure);
    assert_eq!(
        repo.append_messages(id, RunId::new(), Vec::new()).await,
        failure
    );
    assert_eq!(repo.record_run(run(RunId::new())).await, failure);
    assert!(repo.runs().is_empty(), "başarısız kayıt saklanmaz");
    repo.fail_with(None);
    assert_eq!(repo.messages(id).await, Ok(Vec::new()));
}
