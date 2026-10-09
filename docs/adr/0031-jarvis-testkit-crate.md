# ADR 0031: `jarvis-testkit` crate'i (üretim dışı)

- **Durum:** Kabul edildi (2026-10-09, Yasin Akmaz onayı)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §2, §10
- **Tasarım:** 0011

## Bağlam

`StubLlm`, `FakeClock`, `FakeTool`, bellek içi oturum deposu ve kaset sunucusu birden çok
crate'in testlerinde kullanılacak (Mimari §10 L2/L3/L6).

## Karar

Yeni crate `crates/jarvis-testkit`; katman sözleşmesinde `production = false`. İzinli
bağımlılıklar: `jarvis-types`, `jarvis-provider`, `jarvis-session`, `jarvis-tools`,
`jarvis-approval`. `jarvis-core`, `jarvis-api`, `jarvisd` ve `jarvis-provider` bunu yalnızca
`dev_allowed` olarak kullanır.

`jarvis-provider` ↔ testkit döngüsü: provider'ın testleri yalnızca provider tipine
dokunmayan öğeleri (`FakeClock`, `CassetteServer`) kullanır; aksi hâlde iki ayrı derlenmiş
provider örneği tip uyuşmazlığı yaratır. `StubLlm` core/api testlerinde kullanılır.

Kapı dosyası değişikliği gerektiğinden insan yaması: `docs/runbooks/pending/` (0003).

## Sonuçlar

Sahteler tek yerde; üretim kodunda test dalı yok. Mimari kapısı, bir üretim paketinin
testkit'e normal bağımlılığını reddeder.

## Reddedilen alternatifler

- Her crate'te `test-support` özelliği: sahteler üretim crate'lerinde yaşar, özellik
  birleşmesiyle üretime sızabilir.
- Sahteleri her test dosyasına kopyalamak: sürüklenme.
