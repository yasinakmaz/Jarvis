//! Olay veriyolu: önce kayıpsız denetim, sonra canlı yayın (Tasarım 0005).

use std::fmt;
use std::num::NonZeroU16;
use std::sync::Arc;

use jarvis_types::{BoxFuture, Clock, EventSeq};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{Mutex, broadcast};

use crate::{Event, EventContext, EventKind};

/// Denetim kaydı. Yazılamazsa yayın başarısız olur (kayıpsız).
pub trait AuditSink: Send + Sync {
    /// Olayı kalıcı olarak ekler.
    fn append(&self, event: &Event) -> BoxFuture<'_, Result<(), AuditError>>;
}

/// Denetim kaydına yazılamadı.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("denetim kaydı yazılamadı: {0}")]
pub struct AuditError(pub String);

/// Yayın başarısız; olay hiçbir aboneye gitmedi.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PublishError {
    /// Denetim kaydı yazılamadı. Çağıran koşuyu `internal` ile durdurur.
    #[error(transparent)]
    Audit(#[from] AuditError),
    /// Sıra numaraları tükendi (`u64::MAX`).
    #[error("olay sıra numaraları tükendi")]
    SequenceExhausted,
}

/// Abonenin aldığı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    /// Sıradaki olay.
    Event(Arc<Event>),
    /// Abone yavaş kaldı; bu kadar olay kaçırıldı (sessiz kayıp yok).
    Lagged {
        /// Kaçırılan olay sayısı.
        missed: u64,
    },
}

/// Bir abonelik. Veriyolu düşerse `recv` `None` döner.
#[derive(Debug)]
pub struct Subscription {
    receiver: broadcast::Receiver<Arc<Event>>,
}

impl Subscription {
    /// Sıradaki olayı ya da kayıp bildirimini bekler.
    pub async fn recv(&mut self) -> Option<Received> {
        match self.receiver.recv().await {
            Ok(event) => Some(Received::Event(event)),
            Err(RecvError::Lagged(missed)) => Some(Received::Lagged { missed }),
            Err(RecvError::Closed) => None,
        }
    }
}

/// Olay veriyolu.
pub struct EventBus {
    sender: broadcast::Sender<Arc<Event>>,
    /// Sıradaki sıra numarası; kilit, denetim + yayın sırasını korur.
    next: Mutex<Option<EventSeq>>,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    secrets: Vec<String>,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("subscribers", &self.sender.receiver_count())
            .field("secrets", &self.secrets.len())
            .finish_non_exhaustive()
    }
}

impl EventBus {
    /// Yeni veriyolu. Kapasite, okumayan bir abone için tutulan olay sayısıdır (ikinin
    /// kuvvetine yukarı yuvarlanır); aşılınca abone `Lagged` alır. İlk sıra numarası
    /// [`EventSeq::FIRST`].
    #[must_use]
    pub fn new(capacity: NonZeroU16, audit: Arc<dyn AuditSink>, clock: Arc<dyn Clock>) -> Self {
        let (sender, _) = broadcast::channel(usize::from(capacity.get()));
        Self {
            sender,
            next: Mutex::new(Some(EventSeq::FIRST)),
            audit,
            clock,
            secrets: Vec::new(),
        }
    }

    /// Yeniden başlatmada denetim kaydındaki son numaradan devam etmek için.
    #[must_use]
    pub fn starting_at(self, next: EventSeq) -> Self {
        Self {
            next: Mutex::new(Some(next)),
            ..self
        }
    }

    /// Serbest metin alanlarında maskelenecek gizli değerler (ör. sağlayıcı anahtarları).
    #[must_use]
    pub fn with_secrets(self, secrets: Vec<String>) -> Self {
        Self { secrets, ..self }
    }

    /// Olayı numaralar, maskeler, denetime yazar ve abonelere yayınlar.
    ///
    /// Yayınlar sıralıdır: aboneler olayları sıra numarası sırasıyla görür. İptal güvenliği:
    /// gelecek yarıda bırakılırsa sıra numarasında boşluk olabilir, tekrar olmaz.
    ///
    /// # Errors
    ///
    /// Denetim yazılamazsa [`PublishError::Audit`]; bu durumda hiçbir abone olayı görmez.
    pub async fn publish(
        &self,
        ctx: EventContext,
        kind: EventKind,
    ) -> Result<Arc<Event>, PublishError> {
        // Kilit yayın bitene kadar tutulur: denetim ve abone sırası sıra numarasıyla aynı.
        let mut next = self.next.lock().await;
        let seq = (*next).ok_or(PublishError::SequenceExhausted)?;
        // Numara denetimden önce tüketilir: başarısızlık ya da iptal boşluk bırakır, tekrar değil.
        *next = seq.next();
        let secrets: Vec<&str> = self.secrets.iter().map(String::as_str).collect();
        let event = Arc::new(Event {
            seq,
            at: self.clock.now(),
            run_id: ctx.run_id,
            trace_id: ctx.trace_id,
            kind: kind.redacted(&secrets),
        });
        self.audit.append(&event).await?;
        // Abone yoksa `send` hata döner; bu normaldir (olay denetimde zaten var).
        let _ = self.sender.send(Arc::clone(&event));
        drop(next);
        Ok(event)
    }

    /// Yeni abone; yalnızca bundan sonra yayınlanan olayları görür.
    #[must_use]
    pub fn subscribe(&self) -> Subscription {
        Subscription {
            receiver: self.sender.subscribe(),
        }
    }
}
