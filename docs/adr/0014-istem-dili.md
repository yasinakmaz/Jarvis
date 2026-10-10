# ADR 0014: Sistem istemleri İngilizce, girdi Türkçe

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §1 kural 9, §5

## Bağlam

Model kalitesi İngilizce istemlerde daha yüksek; kullanıcı Türkçe konuşuyor.

## Karar

Sistem istemleri İngilizcedir, depoda sürümlü dosyalar olarak durur ve `agent_version`'a dahil edilir.

## Sonuçlar

İstem değişikliği eval taban çizgisini değiştirir.

## Reddedilen alternatifler

Türkçe istemler.
