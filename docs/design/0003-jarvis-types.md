# Tasarım 0003: `jarvis-types`

- **Durum:** İnsan onayı bekliyor
- **Kilometre taşı:** M1 (PR 2)
- **İlgili ADR'ler:** 0022, 0026, 0029
- **Bağımlılıklar:** `serde`, `serde_json`, `thiserror`, `uuid` (v7, serde), `time` (serde, formatting)

## Amaç

Tüm katmanların paylaştığı saf veri tipleri. G/Ç yok, async çalışma zamanı yok, iç crate
bağımlılığı yok (`architecture.toml`).

## Kapsam dışı

Tel biçimi (OpenAI JSON) tipleri; onlar `async-openai`'dan gelir ve yalnızca provider/api'de
kullanılır (ADR 0029). Doğrulama mantığı (config'de), depolama (session'da).

## Tipler

```rust
// ids.rs — hepsi UUID v7 (zamana göre sıralı), Display = kanonik metin, serde = dizge.
pub struct RunId(Uuid);      pub struct SessionId(Uuid);
pub struct TraceId(Uuid);    pub struct ToolCallId(String); // sağlayıcının verdiği kimlik
pub struct EventSeq(u64);    // veriyolu içinde monoton

impl RunId { pub fn new() -> Self; pub fn parse(s: &str) -> Result<Self, IdError>; }

// time.rs
pub struct Timestamp(OffsetDateTime); // her zaman UTC; serde = RFC 3339
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
    /// Belirtilen süre kadar bekler. Gerçeği `jarvisd`'de (tokio), sahtesi testkit'te.
    fn sleep(&self, duration: Duration) -> BoxFuture<'_, ()>;
}
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// message.rs
pub enum Role { System, User, Assistant, Tool }
pub enum Content { Text(String), ImagePng(Vec<u8>) }  // görüntü M3/M4, tip şimdiden
pub enum Trust { Trusted, Untrusted { source: String } } // Mimari §1 kural 5
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<ToolCallId>,
    pub trust: Trust,
}
pub struct ToolCall { pub id: ToolCallId, pub name: ToolName, pub arguments: serde_json::Value }
pub struct ToolName(String); // ^[a-z][a-z0-9_]{0,63}$, ayrıştırılarak oluşturulur

// tool.rs
pub enum RiskLevel { Read, Act, Sensitive, Destructive } // Ord: Read < ... < Destructive
pub struct ToolSpec {
    pub name: ToolName,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema (draft 2020-12 alt kümesi)
    pub risk: RiskLevel,               // girdiden hesaplanan risk: Tool::risk_for (0008)
}
pub enum Verification { Verified, Unverifiable { reason: String }, Failed { reason: String } }

// error.rs — API'nin `code` alanında taşınan kodlar (Mimari §4)
#[non_exhaustive]
pub enum ErrorCode {
    ApprovalDenied, DesktopUnavailable, LimitReached, Halted,
    ProviderUnavailable, ProviderRejected, InvalidRequest, Unauthorized, NotFound, Internal,
}
impl ErrorCode { pub const fn as_str(self) -> &'static str; } // "approval_denied" ...
```

```rust
// redact.rs — saf; günlük, olay ve hata metinleri buradan geçer (Mimari §6, §12)
/// Bilinen gizli değerleri ve `Bearer <...>` desenini `***` ile değiştirir.
pub fn redact(text: &str, secrets: &[&str]) -> String;
```

Kurallar: alanları yalnızca doğrulama gerektirmeyen tipler `pub` alan taşır; `ToolName` gibi
değişmezli tipler `parse` ile oluşturulur (`TryFrom<String>`, serde `try_from`).

## Hata durumları

| Durum | Davranış |
| --- | --- |
| Geçersiz UUID / `ToolName` | `IdError` / `NameError` (`thiserror`), panik yok |
| Zaman taşması | `Timestamp` yalnızca `Clock`'tan ve ayrıştırmadan üretilir; aritmetik `checked_*` |

## Test planı

| Seviye | Test | Oracle |
| --- | --- | --- |
| L1 proptest | Her tip için serde gidiş-dönüş (`T → JSON → T` eşit) | Eşitlik özelliği |
| L1 proptest | `ToolName::parse` yalnızca desene uyan dizgeleri kabul eder | Düzenli ifade (testte bağımsız) |
| L1 | `ErrorCode::as_str` tablosu Mimari §4 adlarıyla aynı | Sabit tablo |
| L1 | `RiskLevel` sıralaması | Bilinen sıra |
| L1 | UUID v7 kimlikler oluşturma sırasıyla artar | Sıralama |

## Açık sorular

Yok.
