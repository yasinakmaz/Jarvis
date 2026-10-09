//! `jarvis-core`'un gördüğü dar arayüz; sahtesi `jarvis-testkit`'te.

use jarvis_types::{BoxFuture, Message, RunId, SessionId};

use crate::{RunRecord, Store, StoreError};

/// Ajan döngüsünün oturum ihtiyaçları.
pub trait SessionRepo: Send + Sync {
    /// Yeni oturum.
    fn create_session(&self) -> BoxFuture<'_, Result<SessionId, StoreError>>;
    /// Oturumun mesajları.
    fn messages(&self, id: SessionId) -> BoxFuture<'_, Result<Vec<Message>, StoreError>>;
    /// Mesajları sona ekler.
    fn append_messages(
        &self,
        id: SessionId,
        run: RunId,
        messages: Vec<Message>,
    ) -> BoxFuture<'_, Result<(), StoreError>>;
    /// Koşu kaydı.
    fn record_run(&self, run: RunRecord) -> BoxFuture<'_, Result<(), StoreError>>;
}

impl SessionRepo for Store {
    fn create_session(&self) -> BoxFuture<'_, Result<SessionId, StoreError>> {
        Box::pin(Self::create_session(self))
    }

    fn messages(&self, id: SessionId) -> BoxFuture<'_, Result<Vec<Message>, StoreError>> {
        Box::pin(Self::messages(self, id))
    }

    fn append_messages(
        &self,
        id: SessionId,
        run: RunId,
        messages: Vec<Message>,
    ) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(Self::append_messages(self, id, run, messages))
    }

    fn record_run(&self, run: RunRecord) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(Self::record_run(self, run))
    }
}
