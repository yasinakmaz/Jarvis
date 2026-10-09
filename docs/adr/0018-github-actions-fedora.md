# ADR 0018: GitHub Actions, `fedora:44` konteyneri

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §11

## Bağlam

CI, hedef sistemle aynı glibc ve kütüphane sürümlerinde derlemeli.

## Karar

GitHub Actions; işler `fedora:44` konteynerinde. Pipeline yalnızca `xtask` komutlarını çağırır. Üçüncü taraf action'lar tam commit SHA'sına sabitlenir; `GITHUB_TOKEN` en düşük yetkiyle.

## Sonuçlar

Yerelde `cargo xtask verify` ne diyorsa CI da onu der.

## Reddedilen alternatifler

Başka CI sunucusu.
