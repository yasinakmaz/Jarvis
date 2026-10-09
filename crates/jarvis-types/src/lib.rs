//! Saf veri tipleri, kimlikler ve hata tipleri.
//!
//! Tüm katmanların paylaştığı tipler (Tasarım 0003). G/Ç yok, async çalışma zamanı yok,
//! iç crate bağımlılığı yok (`architecture.toml`). `OpenAI` tel biçimi burada değil, yalnızca
//! `jarvis-provider` ve `jarvis-api` kenarlarındadır (ADR 0029).

pub mod error;
pub mod ids;
pub mod message;
pub mod redact;
pub mod time;
pub mod tool;

pub use error::ErrorCode;
pub use ids::{EventSeq, IdError, RunId, SessionId, ToolCallId, TraceId};
pub use message::{Content, Message, NameError, Role, ToolCall, ToolName, Trust};
pub use redact::{MASK, redact};
pub use time::{BoxFuture, Clock, TimeError, Timestamp};
pub use tool::{RiskLevel, ToolSpec, Verification};
