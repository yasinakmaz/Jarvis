# ADR 0023: Async çalışma zamanı ve gözlemlenebilirlik

- **Durum:** Kabul edildi (2026-10-09, Yasin Akmaz onayı)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §3, §12
- **Tasarım:** 0005, 0008, 0010

## Bağlam

Tek daemon, tek `tokio` çalışma zamanı (Mimari §3); iptal belirteci her bekleme noktasında;
JSON günlükler journald'a (§12).

## Karar

`tokio` (rt-multi-thread, sync, time, signal, net), `tokio-util` (`CancellationToken`),
`tokio-stream` (broadcast → SSE akışı), `futures-util` (`Stream` birleştiricileri),
`tracing` + `tracing-subscriber` (`json`, `env-filter`). Özellikler crate başına en küçük
kümeyle açılır; `cargo-hack --each-feature` denetler.

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `tokio` | 1.53.2 | 2016-07-01 | 1,05 milyar |
| `tokio-util` | 0.7.20 | 2018-02-01 | 837 milyon |
| `tokio-stream` | 0.1.19 | 2020-12-03 | 508 milyon |
| `futures-util` | 0.3.34 | 2018-03-05 | 988 milyon |
| `tracing` | 0.1.44 | 2017-11-27 | 915 milyon |
| `tracing-subscriber` | 0.3.23 | 2019-06-27 | 655 milyon |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Sonuçlar

`async-openai` ve `axum` zaten `tokio` üzerinde; ek çalışma zamanı yok. Her koşu bir
`tracing` span'ı taşır (`run_id`, `trace_id`).

## Reddedilen alternatifler

- `async-std`/`smol`: ekosistem uyumsuzluğu.
- `log` + `env_logger`: yapılandırılmış alan ve span yok.
- OpenTelemetry: v1 kapsamı dışında (§12).
