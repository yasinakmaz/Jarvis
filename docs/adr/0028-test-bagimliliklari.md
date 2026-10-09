# ADR 0028: Test bağımlılıkları: `proptest`, `insta`, `tempfile`; testkit'te `reqwest`

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §10
- **Tasarım:** 0003–0011

## Bağlam

Mimari §10 L1 özellik testleri, anı görüntüleri, gerçek dosya sistemi testleri ve kaset kayıt
modu (gerçek uca HTTP) istiyor.

## Karar

Yalnızca `[dev-dependencies]` olarak: `proptest`, `insta` (`json`), `tempfile`.
`reqwest` (rustls, json) yalnızca `jarvis-testkit`'te kaset **kayıt** modu için; zaten
`async-openai`'ın geçişli bağımlılığıdır, yeni kod yüzeyi eklemez.

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `proptest` | 1.11.0 | 2017-06-18 | 206 milyon |
| `insta` | 1.49.0 | 2019-01-13 | 109 milyon |
| `tempfile` | 3.27.0 | 2015-04-14 | 873 milyon |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Sonuçlar

Anı görüntüsü değişiklikleri `cargo insta review` ile bilinçli onay ister (Mimari §4).

## Reddedilen alternatifler

- `quickcheck`: daralma ve strateji API'si `proptest`'ten zayıf.
- `wiremock`: kaset biçimini ve tur başına eşlemeyi kendimiz denetlemek istiyoruz; `axum`
  zaten var.
