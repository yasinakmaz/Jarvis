//! Depolanan kayıtların tipleri.

use jarvis_types::{ErrorCode, EventSeq, RunId, SessionId, Timestamp, TraceId};

/// Oturum özeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    /// Kimlik.
    pub id: SessionId,
    /// Oluşturulma (milisaniye çözünürlüğü).
    pub created_at: Timestamp,
    /// Son mesaj eklenme (milisaniye çözünürlüğü).
    pub updated_at: Timestamp,
    /// Mesaj sayısı.
    pub message_count: u64,
}

/// Sayfalama: en yeni güncellenen önce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    offset: u32,
    limit: u32,
}

impl Page {
    /// Sayfa başına en çok kayıt.
    pub const MAX_LIMIT: u32 = 100;
    /// İlk sayfa, en çok kayıtla.
    pub const FIRST: Self = Self {
        offset: 0,
        limit: Self::MAX_LIMIT,
    };

    /// `limit` 1..=[`Page::MAX_LIMIT`] değilse `None`.
    #[must_use]
    pub const fn new(offset: u32, limit: u32) -> Option<Self> {
        if limit == 0 || limit > Self::MAX_LIMIT {
            None
        } else {
            Some(Self { offset, limit })
        }
    }

    /// Atlanacak kayıt.
    #[must_use]
    pub const fn offset(self) -> u32 {
        self.offset
    }

    /// En çok kayıt.
    #[must_use]
    pub const fn limit(self) -> u32 {
        self.limit
    }
}

/// Koşunun sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// Yanıtla bitti.
    Finished,
    /// Hatayla bitti.
    Failed(ErrorCode),
}

impl RunOutcome {
    /// Veritabanındaki metin: `finished` ya da hata kodunun tel adı.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Finished => "finished",
            Self::Failed(code) => code.as_str(),
        }
    }
}

/// `runs` satırı. Aynı kimlikle tekrar kaydedilirse yalnızca bitiş alanları güncellenir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    /// Koşu.
    pub id: RunId,
    /// Oturumlu koşuysa oturum.
    pub session_id: Option<SessionId>,
    /// İz.
    pub trace_id: TraceId,
    /// Başlangıç.
    pub started_at: Timestamp,
    /// Bitiş (sürüyorsa `None`).
    pub finished_at: Option<Timestamp>,
    /// Sonuç (sürüyorsa `None`).
    pub outcome: Option<RunOutcome>,
    /// Kod + istem + araç sürümü kimliği.
    pub agent_version: String,
}

/// Denetim satırı. `jarvis-events`'e bağımlı olmamak için düz alanlar; dönüşüm `jarvisd`'de.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRow {
    /// Olay sıra numarası (benzersiz).
    pub seq: EventSeq,
    /// Olay zamanı.
    pub at: Timestamp,
    /// Koşu.
    pub run_id: Option<RunId>,
    /// İz.
    pub trace_id: TraceId,
    /// Olay türünün tel adı.
    pub kind: String,
    /// Olayın tam JSON'u (maskelenmiş).
    pub payload_json: String,
}
