# Tasarım 0011: M1 test altyapısı (`jarvis-testkit`, kaset, smoke eval)

- **Durum:** Onaylandı (2026-10-09, Yasin Akmaz)
- **Kilometre taşı:** M1 (PR 6, 7, 12)
- **İlgili ADR'ler:** 0028, 0031
- **Bağımlılıklar:** `jarvis-types`, `jarvis-provider`, `jarvis-session`, `jarvis-tools`,
  `tokio`, `axum` (kaset sunucusu), `reqwest` (yalnızca kayıt modu), `serde_json`, `tempfile`

## Amaç

Mimari §10 L2/L3/L6-smoke seviyelerinin sahte bağımlılıkları ve kaset tekrarı. Üretim
ikililerine girmez (`production = false`, ADR 0031).

## Bileşenler

```rust
/// Önceden yazılmış yanıtlar; aldığı her isteği kaydeder (oracle olarak kullanılır).
pub struct StubLlm { /* VecDeque<Scripted>, Mutex<Vec<ChatRequest>> */ }
pub enum Scripted { Reply(ChatResponse), Fail(ProviderError), WaitForCancel }
impl ChatModel for StubLlm { .. }   // senaryo biterse açık hata: "StubLlm: beklenmeyen çağrı #n"

/// Manuel ilerleyen saat; sleep yalnızca advance ile çözülür; tüm bekleme çağrıları kaydedilir.
pub struct FakeClock;  impl Clock for FakeClock { .. }
impl FakeClock { pub fn advance(&self, d: Duration); pub fn sleeps(&self) -> Vec<Duration>; }

/// Yapılandırılabilir risk, çıktı, doğrulama; yürütme ve doğrulama sayaçları.
pub struct FakeTool;  impl Tool for FakeTool { .. }
impl FakeTool { pub fn executions(&self) -> usize; pub fn verifications(&self) -> usize; }

/// Bellek içi SessionRepo.
pub struct MemorySessions;

/// OpenAI uç taklidi: 127.0.0.1:0, kaset dosyasından tekrar.
pub struct CassetteServer;
impl CassetteServer {
    pub async fn replay(path: &Path) -> Result<Self, CassetteError>;
    pub fn base_url(&self) -> String;
    pub fn requests(&self) -> Vec<serde_json::Value>;   // gelen gövdeler (oracle)
    pub fn assert_exhausted(&self) -> Result<(), CassetteError>; // kullanılmayan tur kalmadı
}
```

## Kaset biçimi (`tests/cassettes/<ad>.jsonl`, satır başına bir tur)

```json
{"turn":1,"request":{"method":"POST","path":"/v1/chat/completions","body":{...}},
 "response":{"status":200,"headers":{"content-type":"application/json"},"body":{...}}}
{"turn":2, "...": "...", "response":{"status":200,"stream":["{...}","{...}","[DONE]"]}}
```

- **Tur başına eşleme:** n. istek n. turla karşılaştırılır; gövde kanonik JSON olarak
  (`stream`, `model` dahil) eşit olmalı. Uyuşmazlık → 500 + fark özeti → test kırmızı.
  (Bir dal değişirse sonraki turlar bozulur; bu bilinçli: kaset yeniden kaydedilir.)
- **Eksik kaset/tur:** açık hata. CI'da kayıt modu yoktur; canlı API'ye gidilmez.
- **Kayıt modu:** `JARVIS_CASSETTE_MODE=record` + `NVIDIA_API_KEY` yalnızca yerelde.
  Sunucu vekil olur, gerçek uca iletir, turu yazar. Yazmadan önce `Authorization`,
  `Cookie`, `Set-Cookie`, `x-api-key` başlıkları düşürülür ve gövde `redact`'ten geçer.
  `CI` ortam değişkeni tanımlıyken kayıt modu reddedilir.
- Kasetler gözden geçirilen artefaktlardır; model/istem değişince bilerek yeniden kaydedilir.

## Smoke eval (L6, PR 12)

- `evals/smoke/*.toml`: 10 görev. Her görev: kullanıcı girdisi, kaset, sahte araç seti,
  yapısal beklentiler (çağrılan araçlar, çağrılmaması gerekenler, son yanıt deseni, olay
  sırası), "kasıtlı başarısız" bayrağı.
- En az 5 görev kasıtlı olarak başarısızdır (ör. kasette model `destructive` araç ister);
  harness'in güvensiz eylemi yakaladığını kanıtlar: bu görevlerde beklenen sonuç "harness
  kırmızı" dır.
- Koşucu `jarvis-testkit` içinde bir entegrasyon testidir (`cargo nextest run -p jarvis-testkit
  smoke`), < 2 dk. Kapı: güvensiz eylem sayısı = 0 (sıfır tolerans).
- Kayıt: her görev için `trace_id`, `agent_version`, araç sınırı kayıtları JSON olarak
  `target/evals/` altına.

## Test planı

| Test | Oracle |
| --- | --- |
| StubLlm senaryo dışı çağrıda açık hata | Hata metni |
| FakeClock: `sleep(5s)` yalnızca `advance(5s)` sonrası çözülür | Gelecek durumu |
| Kaset: gövde uyuşmazlığı → 500 ve fark | HTTP + metin |
| Kaset kaydı `Authorization` içermez (kayıt modu, yerel sahte üst uç ile) | Dosya taraması |
| Kaset: `CI=1` iken kayıt modu reddedilir | Hata |
| Smoke: kasıtlı başarısız görevlerin hepsi kırmızı, diğerleri yeşil | Eval raporu |

## Uygulama notları (PR 6)

- **Bölünmüş teslim:** `StubLlm` `jarvis-provider`'ın `ChatModel` trait'ine, `FakeTool`
  `jarvis-tools`'un `Tool` trait'ine bağlıdır; bu trait'ler henüz yok. PR 6 yalnızca bugün
  kurulabilenleri içerir: `FakeClock` ve `MemorySessions`. `StubLlm` ve `CassetteServer`
  PR 7 (provider) ile, `FakeTool` PR 8 (tools) ile eklenir; tasarımdaki sözleşmeler
  değişmez.
- **`FakeClock::new(start)`:** bekleme süresi `sleep` **çağrıldığı** andan ölçülür; sıfır süre
  hemen çözülür; `advance` yalnızca süresi dolanları uyandırır (başka iş parçacığındaki görev
  dahil). Aralık dışına (yıl 9999 sonrası) ilerletme saati değiştirmez.
- **`MemorySessions`:** `jarvis_session::Store` ile aynı gözlenebilir davranış; bunu aynı
  senaryoyu iki uygulamada çalıştıran sözleşme testi kanıtlar (oracle gerçek SQLite).
  Ek olarak `runs()` (oracle) ve `fail_with(Some(hata))` (hata yolu testleri).

## Uygulama notları (PR 7)

- **`StubLlm`** `jarvis-provider`'ın `ChatModel` trait'ini uygular; isteği **çağrı anında**
  kaydeder ve senaryo adımını o anda alır. Senaryo bitince `ProviderError::InvalidRequest(
  "StubLlm: beklenmeyen çağrı #n")` döner (tasarımdaki açık hata).
- **`CassetteServer`** bu PR'da yalnızca **tekrar modu**: kayıt modu (`reqwest`, canlı API)
  PR 12'ye kalır. Eklenenler: yanıtta `text` (JSON olmayan gövde, ör. HTML hata sayfası) ve
  `delay_ms` (gerçek zamanlı gecikme; zaman aşımı/iptal testleri), `Recorded` (başlık dahil
  oracle), `replay_str` (bellekteki kaset). Uyuşmazlık, tükenme ve kullanılmayan tur
  `assert_exhausted` ile kırmızıdır; uyuşmazlık yanıtı 500 + ilk farkın JSON işaretçisidir.
- **Test kilitlenmesi:** uyuşmayan tur 500 döner ve 500 yeniden denenir; saat ilerlemezse bu
  sonsuza dek beklerdi. Sağlayıcı testleri bu yüzden gerçek zamanda 10/20 sn'lik bekçiyle
  (`within`/`drive`) çalışır ve süre aşımında nedeniyle birlikte düşer.
