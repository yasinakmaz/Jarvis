# ADR 0029: Alan modeli tel biçiminden ayrıdır

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §2, §4, §5
- **Tasarım:** 0002, 0003, 0006, 0009

## Bağlam

`async-openai` tipleri ajanın her katmanına yayılırsa `jarvis-core` sağlayıcı kütüphanesinin
sürüm değişikliklerine kilitlenir; ayrıca güven bilgisi (`Trust`) gibi alan kavramları tel
biçiminde yoktur.

## Karar

Ajan `jarvis-types` tipleriyle çalışır. OpenAI tel tipleri yalnızca iki kenarda:
`jarvis-provider` (istemci, `chat-completion` özelliği) ve `jarvis-api` (yalnızca tip
özellikleri `chat-completion-types`, `model-types`; HTTP istemcisi derlenmez). Passthrough
ham JSON (`byot`) ile yapılır; alan modeline dönüştürülmez.

## Sonuçlar

İki eşleme katmanı yazılır ve gidiş-dönüş testleriyle (proptest) korunur. `async-openai`
yükseltmesi yalnızca iki crate'i etkiler.

## Reddedilen alternatifler

- Tel tiplerini her yerde kullanmak: bağlılık ve güven bilgisinin kaybı.
- Tel tiplerini elle yazmak: ADR 0004'e aykırı, sözleşme sürüklenmesi.
