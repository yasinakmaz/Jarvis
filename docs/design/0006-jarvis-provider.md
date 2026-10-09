# Tasarım 0006: `jarvis-provider`

- **Durum:** Onaylandı (2026-10-09, Yasin Akmaz)
- **Kilometre taşı:** M1 (PR 7)
- **İlgili ADR'ler:** 0002, 0004, 0027, 0029, 0030
- **Bağımlılıklar:** `jarvis-types`, `jarvis-config`, `async-openai` (`chat-completion`, `model`,
  `byot`, varsayılan rustls), `secrecy`, `tokio`, `tokio-util`, `futures-util`, `tracing`,
  `serde_json`, `thiserror`

## Amaç

Rol adıyla (`planner`, `vision`, `fast`) OpenAI uyumlu sağlayıcılara ulaşmak: alan tipleri ↔
tel tipleri eşlemesi, sağlayıcı başına hız sınırı, yeniden denenebilir hatalarda üstel geri
çekilme, iptal. Barındırılan ↔ yerel geçiş yalnızca `base_url` (ADR 0002).

## Kapsam dışı

Otomatik sağlayıcı değiştirme (ADR 0030). Görüntü girdisi eşlemesi (M4). Maliyet hesabı.

## Tipler

```rust
pub enum RoleName { Planner, Vision, Fast }

pub struct ChatRequest { pub messages: Vec<Message>, pub tools: Vec<ToolSpec>,
                         pub max_tokens: Option<u32> }
pub struct ChatResponse { pub message: Message, pub finish: FinishReason,
                          pub usage: Option<Usage>, pub model: String }

/// Ajanın gördüğü tek arayüz. Gerçeği OpenAiModel, sahtesi testkit::StubLlm.
pub trait ChatModel: Send + Sync {
    fn complete(&self, request: ChatRequest, cancel: CancellationToken)
        -> BoxFuture<'_, Result<ChatResponse, ProviderError>>;
}

/// `/v1/chat/completions` passthrough için ham tel JSON'u (byot).
pub enum RawResponse { Json(serde_json::Value),
                       Stream(BoxStream<'static, Result<serde_json::Value, ProviderError>>) }
pub trait RawChat: Send + Sync {
    fn forward(&self, body: serde_json::Value, cancel: CancellationToken)
        -> BoxFuture<'_, Result<RawResponse, ProviderError>>;
}

pub struct Router { /* RoleName → Arc<Bound> (ChatModel + RawChat + bilgi) */ }
impl Router {
    pub fn from_config(config: &Config, env: &dyn SecretLookup, clock: Arc<dyn Clock>)
        -> Result<Self, ProviderError>;              // anahtar burada okunur, SecretString olur
    pub fn chat(&self, role: RoleName) -> Option<Arc<dyn ChatModel>>;
    pub fn raw(&self, role: RoleName) -> Option<Arc<dyn RawChat>>;
    pub fn describe(&self) -> Vec<ProviderInfo>;    // /providers için; anahtar yok
}

pub enum ProviderError {
    RateLimited { retry_after: Option<Duration> },  // 429, denemeler tükendi
    Unavailable { status: Option<u16>, message: String }, // 5xx/zaman aşımı/ağ, tükendi
    Rejected { status: u16, message: String },     // 400/401/403/404: denenmez
    InvalidResponse(String),                        // eşlenemeyen yanıt
    Cancelled,
}
```

## Algoritma: çağrı yolu

```mermaid
flowchart TD
    s[complete] --> m[alan → tel eşleme]
    m --> tb{token bucket: izin var mı}
    tb -- yok --> w[clock.sleep kalan süre, iptal dinlenir] --> tb
    tb -- var --> h[HTTP isteği, timeout]
    h --> c{sonuç}
    c -- 2xx --> map[tel → alan eşleme; bozuksa InvalidResponse] --> ok([yanıt])
    c -- 429 / 5xx / zaman aşımı / ağ --> r{deneme < 5?}
    r -- evet --> b["bekle: min(32s, 0.5s·2^n) + jitter, Retry-After varsa o"] --> ev[run.step: geri çekilme bilgisi] --> tb
    r -- hayır --> err([RateLimited / Unavailable])
    c -- 400/401/403/404 --> rej([Rejected, denenmez])
    w & b -. iptal .-> can([Cancelled])
```

- Jitter deterministik değildir; testte `Clock` + enjekte edilen rastgele kaynak sabitlenir.
- Geri çekilme bildirimi: provider olay veriyolunu bilmez; `ChatModel`'e verilen
  `RetryObserver` geri çağrısıyla core `run.step` yayınlar.
- Araç çağrısı argümanları JSON metin olarak gelir; ayrıştırılamazsa `InvalidResponse` değil,
  ajana "geçersiz argüman" araç sonucu olarak döner (core'da; model kendini düzeltebilir).

## Gizli değerler

- Anahtar `SecretString`'de tutulur; `Debug`/`Display` maskelidir.
- `jarvis-types::redact(text, secrets)` (saf fonksiyon, tüm katmanlar kullanır) bilinen anahtar değerlerini ve `Bearer ...` desenlerini
  `***` ile değiştirir; hata mesajları ve olay metinleri bundan geçer.

## Kaset tekrarı (L3)

`jarvis-testkit::CassetteServer` yerel `127.0.0.1:0` üzerinde OpenAI ucu taklidi yapar; testte
sağlayıcının `base_url`'i ona yönelir — üretim kodunda kaset dalı yoktur. Ayrıntı: tasarım 0011.

## Hata durumları

| Durum | Davranış |
| --- | --- |
| 429, Retry-After | O kadar bekle; 5 denemede `RateLimited` |
| 401 | Hemen `Rejected`; mesaj anahtarı içermez |
| Yanıtta ne metin ne araç çağrısı | `InvalidResponse` |
| Rol atanmamış (`vision`) | `Router::chat` → `None`; çağıran `provider_unavailable` döner |
| İptal | Bekleme ve istek sırasında; `Cancelled` |

## Test planı

| Seviye | Test | Oracle |
| --- | --- | --- |
| L1 proptest | Alan → tel → alan eşleme gidiş-dönüşü (mesaj, araç çağrısı, araç sonucu) | Eşitlik |
| L1 | Token bucket: 40/dk, FakeClock ile 41. istek 1,5 sn bekler | Saat |
| L2 | Kaset: 429, 429, 200 → 2 bekleme (0,5 s, 1 s ± jitter=0), başarı | FakeClock kayıtları |
| L2 | 401 → tek istek, `Rejected`, anahtar hata metninde yok | İstek sayısı, metin taraması |
| L2 | Bekleme sırasında iptal → `Cancelled`, ek istek yok | İstek sayısı |
| L3 | Kaydedilmiş NVIDIA yanıtları (metin, araç çağrısı, akış) | Kaset |
| L1 | `redact` anahtarı ve `Bearer` desenini maskeler | Metin |
