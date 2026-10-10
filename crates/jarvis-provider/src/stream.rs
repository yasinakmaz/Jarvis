//! Passthrough akışı: hata, iptal ve boşta kalma zaman aşımı açık son öğe olur.

use std::time::Duration;

use async_openai::error::OpenAIError;
use futures_util::stream::{self, BoxStream, Stream, StreamExt};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::ProviderError;
use crate::classify::from_openai;

struct State<S> {
    inner: S,
    cancel: CancellationToken,
    idle: Duration,
    secret: SecretString,
    done: bool,
}

/// `inner` parçalarını iletir. Hata, iptal ya da `idle` boyunca parça gelmemesi son öğe olan
/// bir `Err` üretir ve akış biter; yeniden deneme yoktur (bayt zaten iletilmiş olabilir).
pub fn guard<S>(
    inner: S,
    cancel: CancellationToken,
    idle: Duration,
    secret: SecretString,
) -> BoxStream<'static, Result<Value, ProviderError>>
where
    S: Stream<Item = Result<Value, OpenAIError>> + Send + Unpin + 'static,
{
    let state = State {
        inner,
        cancel,
        idle,
        secret,
        done: false,
    };
    stream::unfold(state, |mut state| async move {
        if state.done {
            return None;
        }
        let item = next_item(&mut state).await?;
        state.done = item.is_err();
        Some((item, state))
    })
    .boxed()
}

async fn next_item<S>(state: &mut State<S>) -> Option<Result<Value, ProviderError>>
where
    S: Stream<Item = Result<Value, OpenAIError>> + Unpin,
{
    tokio::select! {
        biased;
        () = state.cancel.cancelled() => Some(Err(ProviderError::Cancelled)),
        waited = tokio::time::timeout(state.idle, state.inner.next()) => match waited {
            Ok(Some(Ok(chunk))) => Some(Ok(chunk)),
            Ok(Some(Err(error))) => {
                let secret = state.secret.expose_secret();
                Some(Err(from_openai(error, &[secret]).into_final()))
            }
            Ok(None) => None,
            Err(_) => Some(Err(ProviderError::Unavailable {
                status: None,
                message: "akış zaman aşımı: parça gelmedi".to_owned(),
            })),
        },
    }
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
