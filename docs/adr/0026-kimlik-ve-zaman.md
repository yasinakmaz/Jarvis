# ADR 0026: Kimlikler `uuid` v7, zaman `time`

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §3, §12
- **Tasarım:** 0003

## Bağlam

`run_id`, `trace_id`, oturum kimlikleri benzersiz ve zamana göre sıralanabilir olmalı;
zaman damgaları UTC ve RFC 3339.

## Karar

`uuid` (`v7`, `serde`) ve `time` (`serde`, `formatting`, `parsing`). Zaman her zaman `Clock`
trait'inden alınır (testte `FakeClock`).

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `uuid` | 1.27.0 | 2014-11-11 | 865 milyon |
| `time` | 0.3.55 | 2014-11-13 | 958 milyon |
| `jiff` | 0.2.38 | 2024-02-17 | 219 milyon |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Sonuçlar

UUID v7 SQLite'ta metin olarak sıralı; denetim kaydı ve günlükler aynı biçimi paylaşır.

## Reddedilen alternatifler

- `jiff`: modern, ama yalnızca UTC damgası ve RFC 3339 gerekiyor; `time` daha olgun.
- `chrono`: daha geniş API, geçmiş güvenlik bildirimleri.
- `ulid`: UUID v7 aynı sıralama özelliğini standart biçimde veriyor.
