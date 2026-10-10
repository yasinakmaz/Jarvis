//! `OpenAI` uyumlu sağlayıcı istemcisi: hız sınırı, geri çekilme, zaman aşımı, iptal.
//!
//! `async-openai`'nin kendi yeniden deneme katmanı **kapalıdır** (`ReqwestService` doğrudan
//! takılır): bekleme ve deneme sayısı bu dosyada, enjekte edilen saatle yönetilir; böylece
//! her bekleme gözlenebilir ve iptal edilebilir.

use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::middleware::ReqwestService;
use jarvis_config::Provider;
use jarvis_types::{BoxFuture, Clock};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::backoff::{MAX_ATTEMPTS, delay_for};
use crate::classify::{Outcome, Retry, from_openai};
use crate::wire::{from_wire_response, to_wire_request};
use crate::{
    ChatModel, ChatRequest, ChatResponse, Jitter, ProviderError, RawChat, RawResponse, RetryNotice,
    RetryObserver, RetryReason, TokenBucket, stream,
};

/// Tek bir sağlayıcıya bağlı model: hem [`ChatModel`] hem [`RawChat`].
pub struct OpenAiModel {
    client: Client<OpenAIConfig>,
    model: String,
    key: SecretString,
    bucket: TokenBucket,
    clock: Arc<dyn Clock>,
    jitter: Arc<dyn Jitter>,
    timeout: Duration,
}

impl OpenAiModel {
    /// Yapılandırılmış sağlayıcı için istemci kurar. `key` yalnızca burada ve istemcinin
    /// yetkilendirme başlığında yaşar. Kütüphanenin ortam okuması (`OPENAI_*`) kapatılır:
    /// kuruluş/proje başlıkları üçüncü taraf sağlayıcıya sızmasın diye boş verilir.
    #[must_use]
    pub fn new(
        provider: &Provider,
        key: SecretString,
        clock: Arc<dyn Clock>,
        jitter: Arc<dyn Jitter>,
    ) -> Self {
        let config = OpenAIConfig::new()
            .with_api_base(provider.base_url.as_str().trim_end_matches('/'))
            .with_api_key(key.expose_secret())
            .with_org_id("")
            .with_project_id("");
        let client = Client::with_config(config).with_http_service(ReqwestService::default());
        Self {
            client,
            model: provider.model.clone(),
            bucket: TokenBucket::new(provider.requests_per_minute, Arc::clone(&clock)),
            key,
            clock,
            jitter,
            timeout: provider.timeout,
        }
    }

    /// İstek zaman aşımını değiştirir (testlerde kısa süre için).
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Hız sınırı → istek (zaman aşımıyla) → sınıflama → bekleme döngüsü.
    async fn with_retry<T, F, Fut>(
        &self,
        cancel: &CancellationToken,
        observer: &dyn RetryObserver,
        attempt: F,
    ) -> Result<T, ProviderError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, OpenAIError>>,
    {
        let mut failed: u32 = 0;
        loop {
            self.bucket.acquire(cancel).await?;
            let outcome = tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(ProviderError::Cancelled),
                result = tokio::time::timeout(self.timeout, attempt()) => match result {
                    Ok(Ok(value)) => return Ok(value),
                    Ok(Err(error)) => from_openai(error, &[self.key.expose_secret()]),
                    Err(_) => Outcome::Retry(Retry {
                        reason: RetryReason::Unavailable,
                        status: None,
                        message: "zaman aşımı".to_owned(),
                    }),
                },
            };
            let Outcome::Retry(retry) = outcome else {
                return Err(outcome.into_final());
            };
            failed = failed.saturating_add(1);
            if failed >= MAX_ATTEMPTS {
                return Err(Outcome::Retry(retry).into_final());
            }
            let delay = delay_for(failed, &*self.jitter);
            tracing::warn!(attempt = failed, ?delay, reason = ?retry.reason, status = ?retry.status,
                "sağlayıcı isteği başarısız; yeniden denenecek");
            observer.on_retry(RetryNotice {
                attempt: failed,
                delay,
                reason: retry.reason,
            });
            tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(ProviderError::Cancelled),
                () = self.clock.sleep(delay) => {}
            }
        }
    }
}

impl fmt::Debug for OpenAiModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiModel")
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl ChatModel for OpenAiModel {
    fn complete<'a>(
        &'a self,
        request: ChatRequest,
        cancel: CancellationToken,
        observer: &'a dyn RetryObserver,
    ) -> BoxFuture<'a, Result<ChatResponse, ProviderError>> {
        Box::pin(async move {
            let wire = to_wire_request(&self.model, &request)?;
            let response = self
                .with_retry(&cancel, observer, || async {
                    let chat = self.client.chat();
                    chat.create(wire.clone()).await
                })
                .await?;
            from_wire_response(response)
        })
    }
}

impl RawChat for OpenAiModel {
    fn forward<'a>(
        &'a self,
        mut body: Value,
        cancel: CancellationToken,
        observer: &'a dyn RetryObserver,
    ) -> BoxFuture<'a, Result<RawResponse, ProviderError>> {
        Box::pin(async move {
            let object = body.as_object_mut().ok_or_else(|| {
                ProviderError::InvalidRequest("istek gövdesi bir JSON nesnesi olmalı".to_owned())
            })?;
            object.insert("model".to_owned(), Value::String(self.model.clone()));
            if object.get("stream") == Some(&Value::Bool(true)) {
                let upstream = self
                    .with_retry(&cancel, observer, || async {
                        let chat = self.client.chat();
                        chat.create_stream_byot::<Value, Value>(body.clone()).await
                    })
                    .await?;
                let guarded = stream::guard(upstream, cancel, self.timeout, self.key.clone());
                return Ok(RawResponse::Stream(guarded));
            }
            let json = self
                .with_retry(&cancel, observer, || async {
                    let chat = self.client.chat();
                    chat.create_byot::<Value, Value>(body.clone()).await
                })
                .await?;
            Ok(RawResponse::Json(json))
        })
    }
}
