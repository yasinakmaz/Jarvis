# ADR 0019: RPM paketi; otomatik güncelleme yok

- **Durum:** Öneri — M0 #11 ile doğrulanacak
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §11

## Bağlam

ScreenShot2 yetkisi ve portal onayı sabit ikili yoluna ve `.desktop` kimliğine bağlı. Daemon masaüstünü kontrol ettiği için otomatik güncelleme tedarik zinciri saldırısını masaüstü yetkisine çevirir.

## Karar

RPM paketi (`cargo-generate-rpm`, yedek: `rpmbuild` spec). Güncelleme elle; `deny.toml` `self_update` gibi crate'leri yasaklar.

## Sonuçlar

Sabit `/usr/bin/jarvisd` yolu; dev/prod ayrı kimlik.

## Reddedilen alternatifler

Tarball + betik, otomatik güncelleme.
