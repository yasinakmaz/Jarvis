# ADR 0013: Uygulama gelene kadar terminal istemcisi

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §13 M2

## Bağlam

Onay akışı uygulama (M5) gelmeden test edilmeli.

## Karar

`jarvis` terminal istemcisi olayları gösterir ve onayları alır; yalnızca HTTP konuşur (`jarvis-core`'a bağlanamaz, `architecture.toml`).

## Sonuçlar

Onay akışı erken sınanır.

## Reddedilen alternatifler

Onaysız çalışma.
