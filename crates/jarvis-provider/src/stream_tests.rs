use std::time::Duration;

use async_openai::error::OpenAIError;
use futures_util::StreamExt;
use futures_util::stream;
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::guard;
use crate::ProviderError;

const IDLE: Duration = Duration::from_millis(80);

fn secret() -> SecretString {
    SecretString::from("sk-secret".to_owned())
}

async fn collect(
    inner: impl futures_util::Stream<Item = Result<Value, OpenAIError>> + Send + Unpin + 'static,
    cancel: CancellationToken,
) -> Vec<Result<Value, ProviderError>> {
    // Üst sınır: bozuk bir koruma sonsuz akış üretse bile test biter ve düşer.
    guard(inner, cancel, IDLE, secret())
        .take(20)
        .collect()
        .await
}

#[tokio::test]
async fn chunks_pass_through_and_the_stream_ends_with_the_inner_one() {
    let inner = stream::iter(vec![Ok(json!({"n": 1})), Ok(json!({"n": 2}))]);
    let items = collect(inner, CancellationToken::new()).await;
    assert_eq!(items, [Ok(json!({"n": 1})), Ok(json!({"n": 2}))]);
}

#[tokio::test]
async fn the_first_error_is_the_last_item_and_hides_the_key() {
    let inner = stream::iter(vec![
        Ok(json!({"n": 1})),
        Err(OpenAIError::InvalidArgument(
            "anahtar sk-secret sızdı".to_owned(),
        )),
        Ok(json!({"n": 3})),
    ]);
    let items = collect(inner, CancellationToken::new()).await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items.first(), Some(&Ok(json!({"n": 1}))));
    let Some(Err(ProviderError::InvalidRequest(message))) = items.get(1) else {
        panic!("InvalidRequest bekleniyordu: {items:?}");
    };
    assert!(
        !message.contains("sk-secret") && message.contains("***"),
        "{message}"
    );
}

#[tokio::test]
async fn cancellation_ends_a_stalled_stream_with_cancelled() {
    let cancel = CancellationToken::new();
    let inner = stream::pending::<Result<Value, OpenAIError>>();
    let mut guarded = guard(inner, cancel.clone(), Duration::from_secs(30), secret());
    cancel.cancel();
    assert_eq!(guarded.next().await, Some(Err(ProviderError::Cancelled)));
    assert_eq!(guarded.next().await, None);
}

#[tokio::test]
async fn silence_longer_than_the_idle_limit_is_an_unavailable_error() {
    let inner = stream::pending::<Result<Value, OpenAIError>>();
    let items = collect(inner, CancellationToken::new()).await;
    assert!(
        matches!(items.as_slice(), [Err(ProviderError::Unavailable { status: None, message })]
            if message.contains("zaman aşımı")),
        "{items:?}"
    );
}
