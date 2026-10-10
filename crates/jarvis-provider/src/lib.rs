//! `OpenAI` uyumlu sağlayıcı istemcisi, rol yönlendirme, hız sınırı ve geri çekilme
//! (Tasarım 0006).
//!
//! Ajan yalnızca [`ChatModel`] ve [`RawChat`] görür; `OpenAI` tel tipleri bu crate'in dışına
//! çıkmaz (ADR 0029).

mod backoff;
mod bucket;
mod classify;
mod client;
mod error;
mod model;
mod router;
mod stream;
mod wire;

pub use backoff::{BASE_DELAY, FixedJitter, Jitter, MAX_ATTEMPTS, MAX_DELAY, RandomJitter};
pub use bucket::TokenBucket;
pub use client::OpenAiModel;
pub use error::ProviderError;
pub use model::{
    ChatModel, ChatRequest, ChatResponse, FinishReason, NoObserver, RawChat, RawResponse,
    RetryNotice, RetryObserver, RetryReason, RoleName, Usage,
};
pub use router::{ProviderInfo, Router};
