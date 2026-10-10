//! Önceden yazılmış yanıtlarla çalışan model sahtesi (Tasarım 0011).

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use jarvis_provider::{ChatModel, ChatRequest, ChatResponse, ProviderError, RetryObserver};
use jarvis_types::BoxFuture;
use tokio_util::sync::CancellationToken;

/// Bir çağrıya verilecek cevap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scripted {
    /// Bu yanıtı döndür.
    Reply(ChatResponse),
    /// Bu hatayı döndür.
    Fail(ProviderError),
    /// İptal edilene kadar bekle, sonra `Cancelled` döndür.
    WaitForCancel,
}

/// Senaryolu model. Aldığı her isteği kaydeder (oracle).
#[derive(Debug)]
pub struct StubLlm {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    script: VecDeque<Scripted>,
    seen: Vec<ChatRequest>,
}

impl StubLlm {
    /// Senaryo sırayla tüketilir.
    #[must_use]
    pub fn new(script: impl IntoIterator<Item = Scripted>) -> Self {
        Self {
            state: Mutex::new(State {
                script: script.into_iter().collect(),
                seen: Vec::new(),
            }),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Şimdiye kadar alınan istekler, çağrı sırasıyla.
    #[must_use]
    pub fn requests(&self) -> Vec<ChatRequest> {
        self.state().seen.clone()
    }

    /// Henüz tüketilmemiş senaryo adımı sayısı.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.state().script.len()
    }
}

impl ChatModel for StubLlm {
    /// İstek, çağrı anında kaydedilir; senaryo adımı da o anda alınır.
    fn complete<'a>(
        &'a self,
        request: ChatRequest,
        cancel: CancellationToken,
        _observer: &'a dyn RetryObserver,
    ) -> BoxFuture<'a, Result<ChatResponse, ProviderError>> {
        let (step, number) = {
            let mut state = self.state();
            state.seen.push(request);
            (state.script.pop_front(), state.seen.len())
        };
        Box::pin(async move {
            match step {
                Some(Scripted::Reply(response)) => Ok(response),
                Some(Scripted::Fail(error)) => Err(error),
                Some(Scripted::WaitForCancel) => {
                    cancel.cancelled().await;
                    Err(ProviderError::Cancelled)
                }
                None => Err(ProviderError::InvalidRequest(format!(
                    "StubLlm: beklenmeyen çağrı #{number}"
                ))),
            }
        })
    }
}
