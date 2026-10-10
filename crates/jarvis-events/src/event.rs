//! Olay tipleri. Tel adları Mimari §3 olay tablosuyla aynıdır.

use jarvis_types::{
    ErrorCode, EventSeq, RiskLevel, RunId, SessionId, Timestamp, ToolCall, ToolCallId, ToolName,
    TraceId, Verification, redact,
};
use serde::{Deserialize, Serialize};

/// Yayınlanmış bir olay. Sıra numarası ve zaman damgasını veriyolu atar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Veriyolunda benzersiz, artan sıra numarası.
    pub seq: EventSeq,
    /// Yayın zamanı (enjekte edilen saatten).
    pub at: Timestamp,
    /// Koşuya ait değilse (ör. `halt`) `None`.
    pub run_id: Option<RunId>,
    /// İz kimliği; günlük, olay ve denetimde aynı.
    pub trace_id: TraceId,
    /// Olayın türü ve verisi; tel adı `type` alanındadır.
    #[serde(flatten)]
    pub kind: EventKind,
}

/// Yayıncının verdiği bağlam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventContext {
    /// Koşu kimliği.
    pub run_id: Option<RunId>,
    /// İz kimliği.
    pub trace_id: TraceId,
}

/// Bir koşu adımının evresi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StepPhase {
    /// Model çağrısı (planlama).
    Plan,
    /// Araç yürütme.
    Act,
    /// Bağımsız doğrulama.
    Verify,
    /// Sağlayıcı geri çekilmesi (429/5xx) sonrası bekleme.
    Retry,
}

/// `tool.requested` içindeki çağrı özeti. Argümanlar kısaltılmış JSON metnidir; tam hâli
/// denetim kaydında değil oturum geçmişindedir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallSummary {
    /// Çağrı kimliği.
    pub id: ToolCallId,
    /// Araç adı.
    pub name: ToolName,
    /// Argümanların sıkıştırılmış JSON'u, en çok [`ToolCallSummary::MAX_ARGUMENTS`] karakter;
    /// kesildiyse sonunda `…` vardır.
    pub arguments: String,
}

impl ToolCallSummary {
    /// Argüman önizlemesinin en çok karakter sayısı.
    pub const MAX_ARGUMENTS: usize = 512;

    /// Çağrıdan özet çıkarır.
    #[must_use]
    pub fn from_call(call: &ToolCall) -> Self {
        let full = call.arguments.to_string();
        let arguments = match full.char_indices().nth(Self::MAX_ARGUMENTS) {
            Some((cut, _)) => format!("{}…", full.get(..cut).unwrap_or_default()),
            None => full,
        };
        Self {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments,
        }
    }
}

/// Olay türleri (Mimari §3). Onay olayları (`approval.*`) M2'de eklenir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum EventKind {
    /// Koşu başladı.
    #[serde(rename = "run.started")]
    RunStarted {
        /// Oturumlu koşuysa oturum.
        session_id: Option<SessionId>,
    },
    /// Planlama/yürütme adımı.
    #[serde(rename = "run.step")]
    RunStep {
        /// Adım numarası (1'den).
        step: u32,
        /// Evre.
        phase: StepPhase,
        /// İnsan için kısa açıklama (maskelenir).
        detail: String,
    },
    /// Koşu başarıyla bitti.
    #[serde(rename = "run.finished")]
    RunFinished {
        /// Toplam adım.
        steps: u32,
    },
    /// Koşu hatayla bitti.
    #[serde(rename = "run.failed")]
    RunFailed {
        /// API hata kodu.
        code: ErrorCode,
        /// Açıklama (maskelenir).
        message: String,
    },
    /// Araç çağrısı istendi.
    #[serde(rename = "tool.requested")]
    ToolRequested {
        /// Çağrı özeti (argümanlar maskelenir).
        call: ToolCallSummary,
        /// Bu girdiyle hesaplanan risk.
        risk: RiskLevel,
    },
    /// Araç çağrısı bitti ve bağımsız gözlemci sonucu biliniyor.
    #[serde(rename = "tool.completed")]
    ToolCompleted {
        /// Çağrı kimliği.
        call_id: ToolCallId,
        /// Doğrulama sonucu (neden metni maskelenir).
        verification: Verification,
    },
    /// Masaüstü arka ucu kullanılamıyor; başka arka uca geçilmez.
    #[serde(rename = "desktop.unavailable")]
    DesktopUnavailable {
        /// Arka uç adı.
        backend: String,
        /// Neden (maskelenir).
        reason: String,
    },
    /// Acil durdurma.
    #[serde(rename = "halt")]
    Halt {
        /// İptal edilen koşu sayısı.
        cancelled_runs: u32,
    },
}

impl EventKind {
    /// Tel adı (`type` alanı ve SSE `event:` satırı).
    #[must_use]
    pub const fn wire_name(&self) -> &'static str {
        match self {
            Self::RunStarted { .. } => "run.started",
            Self::RunStep { .. } => "run.step",
            Self::RunFinished { .. } => "run.finished",
            Self::RunFailed { .. } => "run.failed",
            Self::ToolRequested { .. } => "tool.requested",
            Self::ToolCompleted { .. } => "tool.completed",
            Self::DesktopUnavailable { .. } => "desktop.unavailable",
            Self::Halt { .. } => "halt",
        }
    }

    /// Serbest metin alanlarını maskeler (`jarvis_types::redact`).
    #[must_use]
    pub fn redacted(self, secrets: &[&str]) -> Self {
        let mask = |text: String| redact(&text, secrets);
        match self {
            Self::RunStep {
                step,
                phase,
                detail,
            } => Self::RunStep {
                step,
                phase,
                detail: mask(detail),
            },
            Self::RunFailed { code, message } => Self::RunFailed {
                code,
                message: mask(message),
            },
            Self::ToolRequested { call, risk } => Self::ToolRequested {
                call: ToolCallSummary {
                    arguments: mask(call.arguments),
                    ..call
                },
                risk,
            },
            Self::ToolCompleted {
                call_id,
                verification,
            } => {
                let verification = match verification {
                    Verification::Unverifiable { reason } => Verification::Unverifiable {
                        reason: mask(reason),
                    },
                    Verification::Failed { reason } => Verification::Failed {
                        reason: mask(reason),
                    },
                    Verification::Verified => Verification::Verified,
                };
                Self::ToolCompleted {
                    call_id,
                    verification,
                }
            }
            Self::DesktopUnavailable { backend, reason } => Self::DesktopUnavailable {
                backend: mask(backend),
                reason: mask(reason),
            },
            other @ (Self::RunStarted { .. } | Self::RunFinished { .. } | Self::Halt { .. }) => {
                other
            }
        }
    }
}
