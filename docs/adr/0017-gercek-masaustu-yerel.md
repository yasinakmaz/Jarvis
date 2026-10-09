# ADR 0017: Gerçek masaüstü testleri yerel sürüm kapısı

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §10 L5c

## Bağlam

L5c gerçek imleci oynatır; günlük kullanılan PC'de self-hosted runner güvenli değil (Equinor rehberi).

## Karar

`cargo xtask e2e-real` yerelde çalışır, sonucu commit SHA'sına bağlı bir kayda yazar; `cargo xtask release` kayıt yoksa etiket oluşturmaz (M3'te uygulanacak).

## Sonuçlar

Regresyonlar sürüm kesilene kadar fark edilmeyebilir; ikinci makine/VM ile kapatılabilir.

## Reddedilen alternatifler

Self-hosted runner.
