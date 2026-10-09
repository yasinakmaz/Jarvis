# ADR 0025: HTTP katmanı: `axum`, `tower`, `utoipa`; systemd: `sd-notify`

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §4, §11
- **Tasarım:** 0009, 0010

## Bağlam

Mimari §2 `axum`'u belirledi; §4 OpenAPI belgesinin koddan üretilmesini; §11 `Type=notify` +
watchdog istiyor.

## Karar

`axum` (http1, json, tokio, query) + `tower` (gövde sınırı, zaman aşımı katmanları);
`utoipa` ile OpenAPI 3.1 (`axum_extras`); `sd-notify` ile `READY=1`, `WATCHDOG=1`,
`STOPPING=1`. `tower-http` eklenmez; gereken iki katman `tower` ile yazılır.

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `axum` | 0.8.9 | 2021-07-22 | 511 milyon |
| `tower` | 0.5.3 | 2016-12-23 | 722 milyon |
| `utoipa` | 6.0.0 | 2022-01-27 | 52 milyon |
| `sd-notify` | 0.5.0 | 2019-09-20 | 15,5 milyon |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Sonuçlar

OpenAPI belgesi işleyicilerle aynı kaynaktan; L4 testi depodaki `openapi.json` ile eşitliği
denetler. `axum` yalnızca `jarvis-api`'de.

## Reddedilen alternatifler

- `actix-web`/`warp`: §2 kararıyla uyumsuz.
- Elle yazılmış OpenAPI: kodla sürüklenir.
- `libsystemd`: daha geniş yüzey; yalnızca bildirim gerekiyor.
