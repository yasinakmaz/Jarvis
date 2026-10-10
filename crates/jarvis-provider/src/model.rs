//! Ajanın gördüğü sağlayıcı arayüzü ve alan tipleri (Tasarım 0006).

use std::fmt;
use std::time::Duration;

use futures_util::stream::BoxStream;
use jarvis_types::{BoxFuture, Message, ToolSpec};
use tokio_util::sync::CancellationToken;

use crate::ProviderError;

/// Yapılandırmadaki rol adı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoleName {
    /// Planlayıcı: araç çağıran ana model.
    Planner,
    /// Görüntü modeli.
    Vision,
    /// Hızlı yardımcı model.
    Fast,
}

impl RoleName {
    /// Tüm roller.
    pub const ALL: [Self; 3] = [Self::Planner, Self::Vision, Self::Fast];

    /// API'deki `model` adı ve yapılandırmadaki anahtar.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Vision => "vision",
            Self::Fast => "fast",
        }
    }

    /// `as_str` çıktısından geri çevirir; bilinmeyen ad `None`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == name)
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Modele gönderilen istek.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatRequest {
    /// Konuşma.
    pub messages: Vec<Message>,
    /// Modele sunulan araçlar.
    pub tools: Vec<ToolSpec>,
    /// Üretilecek en çok belirteç.
    pub max_tokens: Option<u32>,
}

/// Modelin neden durduğu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// Doğal bitiş.
    Stop,
    /// Belirteç sınırına ulaşıldı.
    Length,
    /// Model araç çağırmak istiyor.
    ToolCalls,
    /// İçerik süzgeci devreye girdi.
    ContentFilter,
}

/// Belirteç kullanımı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    /// İstemdeki belirteçler.
    pub prompt_tokens: u32,
    /// Üretilen belirteçler.
    pub completion_tokens: u32,
    /// Toplam.
    pub total_tokens: u32,
}

/// Modelin yanıtı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatResponse {
    /// Asistan mesajı (metin ve/veya araç çağrıları).
    pub message: Message,
    /// Bitiş nedeni.
    pub finish: FinishReason,
    /// Sağlayıcı bildirdiyse kullanım.
    pub usage: Option<Usage>,
    /// Yanıtı üreten modelin adı (sağlayıcının bildirdiği).
    pub model: String,
}

/// Yeniden deneme öncesi bekleme nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryReason {
    /// 429.
    RateLimited,
    /// 5xx, zaman aşımı veya ağ hatası.
    Unavailable,
}

/// Bir deneme başarısız oldu ve yeniden denenecek.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryNotice {
    /// Başarısız olan denemenin sırası (1'den başlar).
    pub attempt: u32,
    /// Bir sonraki denemeden önce beklenecek süre.
    pub delay: Duration,
    /// Neden.
    pub reason: RetryReason,
}

/// Geri çekilmeyi gözlemler; `jarvis-core` bunu `run.step` olayına çevirir.
pub trait RetryObserver: Send + Sync {
    /// Bekleme başlamadan önce çağrılır.
    fn on_retry(&self, notice: RetryNotice);
}

/// Hiçbir şey yapmayan gözlemci.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoObserver;

impl RetryObserver for NoObserver {
    fn on_retry(&self, _notice: RetryNotice) {}
}

/// Ajanın gördüğü tek model arayüzü. Gerçeği [`crate::OpenAiModel`], sahtesi
/// `jarvis_testkit::StubLlm`.
pub trait ChatModel: Send + Sync {
    /// Tek bir tamamlama ister; hız sınırı, geri çekilme ve zaman aşımı uygulanır.
    fn complete<'a>(
        &'a self,
        request: ChatRequest,
        cancel: CancellationToken,
        observer: &'a dyn RetryObserver,
    ) -> BoxFuture<'a, Result<ChatResponse, ProviderError>>;
}

/// `/v1/chat/completions` passthrough yanıtı.
pub enum RawResponse {
    /// Tek JSON gövdesi.
    Json(serde_json::Value),
    /// `data:` parçaları (`[DONE]` hariç); hata ya da iptal son öğe olur.
    Stream(BoxStream<'static, Result<serde_json::Value, ProviderError>>),
}

impl fmt::Debug for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(value) => f.debug_tuple("Json").field(value).finish(),
            Self::Stream(_) => f.write_str("Stream(..)"),
        }
    }
}

/// Ham tel JSON'u ileten uç (passthrough).
pub trait RawChat: Send + Sync {
    /// Gövdeyi (modelin adı sağlayıcınınkiyle değiştirilerek) iletir; `stream: true` ise
    /// [`RawResponse::Stream`] döner.
    fn forward<'a>(
        &'a self,
        body: serde_json::Value,
        cancel: CancellationToken,
        observer: &'a dyn RetryObserver,
    ) -> BoxFuture<'a, Result<RawResponse, ProviderError>>;
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
