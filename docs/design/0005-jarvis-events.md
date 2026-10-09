# Tasarım 0005: `jarvis-events`

- **Durum:** Onaylandı (2026-10-09, Yasin Akmaz)
- **Kilometre taşı:** M1 (PR 4)
- **İlgili ADR'ler:** 0010, 0023
- **Bağımlılıklar:** `jarvis-types`, `serde`, `serde_json`, `tokio` (sync), `thiserror`

## Amaç

Tüm koşu olaylarını tek yerden yayınlamak: canlı izleyicilere (SSE, Mimari §3) ve **kayıpsız**
denetim kaydına. Her olay `run_id`, `trace_id`, zaman damgası ve monoton sıra numarası taşır.

## Kapsam dışı

HTTP/SSE taşıması (`jarvis-api`), veritabanına yazma (`jarvis-session`; adaptör `jarvisd`'de).

## Tipler

```rust
pub struct Event {
    pub seq: EventSeq, pub at: Timestamp,
    pub run_id: Option<RunId>, pub trace_id: TraceId,
    pub kind: EventKind,
}
#[serde(tag = "type")]  // tel adları Mimari §3 tablosuyla aynı
pub enum EventKind {
    #[serde(rename = "run.started")]  RunStarted { session_id: Option<SessionId> },
    #[serde(rename = "run.step")]     RunStep { step: u32, phase: StepPhase, detail: String },
    #[serde(rename = "run.finished")] RunFinished { steps: u32 },
    #[serde(rename = "run.failed")]   RunFailed { code: ErrorCode, message: String },
    #[serde(rename = "tool.requested")] ToolRequested { call: ToolCallSummary, risk: RiskLevel },
    #[serde(rename = "tool.completed")] ToolCompleted { call_id: ToolCallId,
                                                         verification: Verification },
    #[serde(rename = "approval.requested")] ApprovalRequested { .. }, // M2
    #[serde(rename = "approval.resolved")]  ApprovalResolved { .. },  // M2
    #[serde(rename = "desktop.unavailable")] DesktopUnavailable { backend: String, reason: String },
    #[serde(rename = "halt")] Halt { cancelled_runs: u32 },
}

/// Denetim kaydı. Yazılamazsa yayın başarısız olur (kayıpsız).
pub trait AuditSink: Send + Sync {
    fn append(&self, event: &Event) -> BoxFuture<'_, Result<(), AuditError>>;
}

pub struct EventBus { /* broadcast::Sender<Arc<Event>>, AtomicU64, Arc<dyn AuditSink>, Clock */ }
impl EventBus {
    pub fn new(capacity: usize, audit: Arc<dyn AuditSink>, clock: Arc<dyn Clock>) -> Self;
    pub async fn publish(&self, ctx: EventContext, kind: EventKind)
        -> Result<Arc<Event>, PublishError>;
    pub fn subscribe(&self) -> Subscription;
}
pub enum Received { Event(Arc<Event>), Lagged { missed: u64 } }
impl Subscription { pub async fn recv(&mut self) -> Option<Received>; }

/// SSE çerçevesi (saf): `id: <seq>\nevent: <type>\ndata: <json>\n\n`
pub fn sse_frame(event: &Event) -> Result<String, serde_json::Error>;
```

## Algoritma

```mermaid
sequenceDiagram
    participant P as Yayıncı (core)
    participant B as EventBus
    participant A as AuditSink (SQLite)
    participant S as Aboneler (SSE)
    P->>B: publish(ctx, kind)
    B->>B: seq = fetch_add(1), at = clock.now()
    B->>A: append(event).await
    alt denetim yazılamadı
        A-->>B: AuditError
        B-->>P: PublishError::Audit (koşu run.failed/internal ile durur, /readyz 503)
    else
        B->>S: broadcast (yavaş abone → Lagged{missed}, kayıp açıkça bildirilir)
        B-->>P: Arc<Event>
    end
```

Denetim önce, yayın sonra: izleyicinin gördüğü her olay denetimde de vardır. Yavaş bir SSE
istemcisi yayıncıyı bloklamaz; kaçırdığı olay sayısını `Lagged` ile öğrenir (sessiz kayıp yok),
SSE'de `event: events.lagged` olarak iletilir. İstemci `Last-Event-ID` ile denetim kaydından
tamamlayabilir (M2'de `GET /events?since=` olarak).

## Hata durumları

| Durum | Davranış |
| --- | --- |
| Denetim yazımı başarısız | `publish` hata döner; çağıran koşuyu `internal` ile durdurur; `/readyz` 503 |
| Abone yavaş | `Lagged { missed }` |
| Abone yok | Normal (broadcast hata saymaz) |
| Gizli değer | Olay içerikleri tiplidir; serbest metin alanları (`detail`, `message`) `jarvis-types::redact` maskeleme fonksiyonundan geçer |

## Test planı

| Seviye | Test | Oracle |
| --- | --- | --- |
| L1 | `sse_frame` biçimi; çok satırlı veride her satır `data:` önekli | Elle yazılmış çerçeve |
| L1 | Tüm `EventKind` tel adları Mimari §3 tablosuyla aynı | Elle yazılmış tablo ve JSON |
| L2 | Sıra numarası eşzamanlı yayında benzersiz ve artan | Sıralama özelliği |
| L2 | Kapasite 4, 10 olay, abone okumuyor → `Lagged { missed: 6 }` | Aritmetik |
| L2 | Sahte `AuditSink` hata verir → `publish` hata döner, abone olayı görmez | Abone kuyruğu boş |

## Uygulama notları (PR 4; onaylandı 2026-10-09, Yasin Akmaz)

Onaylı tasarımı netleştiren kararlar; hiçbiri bir kuralı gevşetmez.

- **Sıra ve kilit:** `publish` bir async kilidi sıra numarası atamadan yayına kadar tutar.
  Böylece eşzamanlı yayında da denetim ve aboneler olayları sıra numarası sırasıyla görür.
  Yayınlar sıralanır; denetim zaten tek yazarlıdır (ADR 0024).
- **Boşluk, asla tekrar:** numara denetimden önce tüketilir. Denetim hatası ya da yarıda
  bırakılan (`drop`) bir `publish` sıra numarasında boşluk bırakabilir; aynı numara iki kez
  verilmez. Abone boşluğu numaralardan görebilir.
- **Yeniden başlatma:** `EventBus::starting_at(seq)` denetim kaydındaki son numaradan devam
  eder (`jarvisd` bağlar). `u64::MAX` kullanıldıktan sonra `PublishError::SequenceExhausted`;
  sarma yok.
- **Kapasite:** `NonZeroU16` (tokio `broadcast` 0'da panikler); ikinin kuvvetine yukarı
  yuvarlanır. Test planındaki "kapasite 4 → `Lagged { missed: 6 }`" birebir sağlanır.
- **Maskeleme:** `with_secrets` ile verilen değerler ve `Bearer <token>` denetimden **önce**
  maskelenir. Alanlar: `run.step.detail`, `run.failed.message`, `tool.requested.call.arguments`,
  `tool.completed` doğrulama nedeni, `desktop.unavailable.backend`/`reason`. `Debug`
  gizli değerleri değil yalnızca sayısını gösterir.
- **`StepPhase`:** `plan`, `act`, `verify`, `retry` (sağlayıcı geri çekilmesi, Mimari §3).
- **`ToolCallSummary`:** argümanların sıkıştırılmış JSON'u, en çok 512 karakter (bayt değil);
  kesilirse sonuna `…` eklenir.
- **Onay olayları:** `approval.requested`/`approval.resolved` M2'de eklenir; `EventKind`
  `#[non_exhaustive]`.
- **Tel biçimi:** olay JSON'u düzdür (`seq`, `at`, `run_id`, `trace_id`, `type`, alanlar).
  Kayıp bildirimi `event: events.lagged` + `data: {"missed":N}`; `id:` taşımaz, istemcinin
  `Last-Event-ID`'si ilerlemez.
- **`insta` yok:** tel adları ve çerçeveler elle yazılmış beklenen değerlerle sınanır
  (Tasarım 0004'teki gerekçe).
