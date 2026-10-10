//! Bellek içi [`SessionRepo`]; davranışı `jarvis_session::Store` ile aynıdır (sözleşme testi).

use std::collections::BTreeMap;
use std::future::ready;
use std::sync::{Mutex, PoisonError};

use jarvis_session::{RunRecord, SessionRepo, StoreError};
use jarvis_types::{BoxFuture, Message, RunId, SessionId};

/// Bellek içi oturum deposu.
#[derive(Debug, Default)]
pub struct MemorySessions {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    sessions: BTreeMap<SessionId, Vec<Message>>,
    runs: Vec<RunRecord>,
    failure: Option<StoreError>,
}

impl MemorySessions {
    /// Boş depo.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// İşlemi kilit altında çalıştırır; enjekte edilmiş hata varsa onu döner.
    fn with<T>(
        &self,
        op: impl FnOnce(&mut State) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match &state.failure {
            Some(error) => Err(error.clone()),
            None => op(&mut state),
        }
    }

    /// Bundan sonraki her çağrı bu hatayı döner (`None`: normale dön). Hata yolu testleri için.
    pub fn fail_with(&self, error: Option<StoreError>) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .failure = error;
    }

    /// Kaydedilen koşular, ilk kayıt sırasıyla (aynı kimlik güncellenir).
    #[must_use]
    pub fn runs(&self) -> Vec<RunRecord> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .runs
            .clone()
    }
}

impl SessionRepo for MemorySessions {
    fn create_session(&self) -> BoxFuture<'_, Result<SessionId, StoreError>> {
        Box::pin(ready(self.with(|state| {
            let id = SessionId::new();
            state.sessions.insert(id, Vec::new());
            Ok(id)
        })))
    }

    fn messages(&self, id: SessionId) -> BoxFuture<'_, Result<Vec<Message>, StoreError>> {
        Box::pin(ready(self.with(|state| {
            state
                .sessions
                .get(&id)
                .cloned()
                .ok_or(StoreError::UnknownSession(id))
        })))
    }

    fn append_messages(
        &self,
        id: SessionId,
        _run: RunId,
        messages: Vec<Message>,
    ) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(ready(self.with(|state| {
            let stored = state
                .sessions
                .get_mut(&id)
                .ok_or(StoreError::UnknownSession(id))?;
            stored.extend(messages);
            Ok(())
        })))
    }

    fn record_run(&self, run: RunRecord) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(ready(self.with(|state| {
            match state.runs.iter_mut().find(|existing| existing.id == run.id) {
                Some(existing) => {
                    existing.finished_at = run.finished_at;
                    existing.outcome = run.outcome;
                }
                None => state.runs.push(run),
            }
            Ok(())
        })))
    }
}
