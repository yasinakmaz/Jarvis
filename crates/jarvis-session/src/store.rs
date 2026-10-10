//! Async tutamaç. Tüm `SQLite` erişimi tek aktör iş parçacığındadır (ADR 0024).

use std::path::Path;
use std::sync::Arc;

use jarvis_types::{Clock, EventSeq, Message, RunId, SessionId};

use crate::actor::Actor;
use crate::{AuditRow, Page, RunRecord, SessionSummary, StoreError, open, queries};

/// Depolama tutamacı; ucuzca kopyalanır. Son kopya düşünce aktör durur ve bağlantı kapanır.
#[derive(Clone)]
pub struct Store {
    actor: Actor,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").finish_non_exhaustive()
    }
}

impl Store {
    /// İzinleri denetler, gerekirse yedek alıp migrasyon yapar ve aktörü başlatır. Engelleyen
    /// bir çağrıdır (başlangıçta bir kez); async bağlamda `spawn_blocking` içinden çağırın.
    ///
    /// # Errors
    ///
    /// Daha yeni şema, geniş dosya izni, yedek ya da migrasyon hatası; hepsinde daemon
    /// başlamamalıdır.
    pub fn open(path: &Path, clock: Arc<dyn Clock>) -> Result<Self, StoreError> {
        let conn = open::prepare(path)?;
        Ok(Self {
            actor: Actor::spawn(conn)?,
            clock,
        })
    }

    /// Yeni, boş oturum.
    ///
    /// # Errors
    ///
    /// Veritabanı hatası ya da depolama kapalı.
    pub async fn create_session(&self) -> Result<SessionId, StoreError> {
        let id = SessionId::new();
        let now = self.clock.now();
        self.actor
            .call(move |conn| queries::create_session(conn, id, now))
            .await?;
        Ok(id)
    }

    /// Oturum özeti; yoksa `None`.
    ///
    /// # Errors
    ///
    /// Veritabanı hatası ya da depolama kapalı.
    pub async fn session(&self, id: SessionId) -> Result<Option<SessionSummary>, StoreError> {
        self.actor
            .call(move |conn| queries::session(conn, id))
            .await
    }

    /// Oturumlar, son güncellenen önce.
    ///
    /// # Errors
    ///
    /// Veritabanı hatası ya da depolama kapalı.
    pub async fn list_sessions(&self, page: Page) -> Result<Vec<SessionSummary>, StoreError> {
        self.actor
            .call(move |conn| queries::list_sessions(conn, page))
            .await
    }

    /// Oturumu ve mesajlarını siler (koşu ve denetim kayıtları kalır). Yoksa `false`.
    ///
    /// # Errors
    ///
    /// Veritabanı hatası ya da depolama kapalı.
    pub async fn delete_session(&self, id: SessionId) -> Result<bool, StoreError> {
        self.actor
            .call(move |conn| queries::delete_session(conn, id))
            .await
    }

    /// Oturumun mesajları, eklenme sırasıyla.
    ///
    /// # Errors
    ///
    /// Oturum yoksa [`StoreError::UnknownSession`].
    pub async fn messages(&self, id: SessionId) -> Result<Vec<Message>, StoreError> {
        self.actor
            .call(move |conn| queries::messages(conn, id))
            .await
    }

    /// Mesajları tek transaction'da sona ekler ve oturumun `updated_at`'ini günceller.
    ///
    /// # Errors
    ///
    /// Oturum yoksa [`StoreError::UnknownSession`]; hata olursa hiçbiri eklenmez.
    pub async fn append_messages(
        &self,
        id: SessionId,
        run: RunId,
        messages: Vec<Message>,
    ) -> Result<(), StoreError> {
        let now = self.clock.now();
        self.actor
            .call(move |conn| queries::append_messages(conn, id, run, &messages, now))
            .await
    }

    /// Koşu kaydı ekler; aynı kimlik varsa yalnızca bitiş zamanı ve sonucu günceller.
    ///
    /// # Errors
    ///
    /// Veritabanı hatası ya da depolama kapalı.
    pub async fn record_run(&self, run: RunRecord) -> Result<(), StoreError> {
        self.actor
            .call(move |conn| queries::record_run(conn, &run))
            .await
    }

    /// Denetim satırı ekler (yalnızca ekleme; aynı `seq` reddedilir).
    ///
    /// # Errors
    ///
    /// Veritabanı hatası, `seq` `i64`'e sığmıyorsa [`StoreError::OutOfRange`].
    pub async fn append_audit(&self, row: AuditRow) -> Result<(), StoreError> {
        self.actor
            .call(move |conn| queries::append_audit(conn, &row))
            .await
    }

    /// Olay veriyolunun yeniden başlatmada devam edeceği numara: son `seq` + 1 ya da
    /// [`EventSeq::FIRST`].
    ///
    /// # Errors
    ///
    /// Veritabanı hatası ya da depolama kapalı.
    pub async fn next_audit_seq(&self) -> Result<EventSeq, StoreError> {
        self.actor.call(|conn| queries::next_audit_seq(conn)).await
    }
}
