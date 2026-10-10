//! Mesaj ekle/oku gidiş-dönüşü (Tasarım 0007 test planı, L1 proptest). Oracle: eklenen
//! partilerin birleşimi; sıra korunur.

use std::sync::Arc;
use std::time::Duration;

use jarvis_session::Store;
use jarvis_types::{BoxFuture, Clock, Content, Message, Role, RunId, Timestamp, ToolCallId, Trust};
use proptest::prelude::*;

struct ZeroClock;

impl Clock for ZeroClock {
    #[expect(clippy::unwrap_used, reason = "sabit; geçersizse test düşmeli")]
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(0).unwrap()
    }

    fn sleep(&self, _duration: Duration) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

fn message() -> impl Strategy<Value = Message> {
    let role = prop_oneof![
        Just(Role::System),
        Just(Role::User),
        Just(Role::Assistant),
        Just(Role::Tool)
    ];
    let content = prop_oneof![
        ".{0,40}".prop_map(Content::Text),
        proptest::collection::vec(any::<u8>(), 0..32).prop_map(Content::ImagePng),
    ];
    let trust = prop_oneof![
        Just(Trust::Trusted),
        "[a-z_]{1,10}".prop_map(|source| Trust::Untrusted { source }),
    ];
    let call_id = proptest::option::of(
        "[a-zA-Z0-9_]{1,24}".prop_filter_map("geçerli kimlik", |id| ToolCallId::new(id).ok()),
    );
    (
        role,
        proptest::collection::vec(content, 0..3),
        call_id,
        trust,
    )
        .prop_map(|(role, content, tool_call_id, trust)| Message {
            role,
            content,
            tool_calls: Vec::new(),
            tool_call_id,
            trust,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn appended_batches_read_back_in_order(
        batches in proptest::collection::vec(proptest::collection::vec(message(), 0..4), 1..5)
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("jarvis.db"), Arc::new(ZeroClock)).unwrap();
        let read = runtime.block_on(async {
            let id = store.create_session().await.unwrap();
            for batch in &batches {
                store.append_messages(id, RunId::new(), batch.clone()).await.unwrap();
            }
            store.messages(id).await.unwrap()
        });
        let expected: Vec<Message> = batches.into_iter().flatten().collect();
        prop_assert_eq!(read, expected);
    }
}
