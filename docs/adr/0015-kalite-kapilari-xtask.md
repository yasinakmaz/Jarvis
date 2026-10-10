# ADR 0015: Kalite kapıları `xtask` ile; `unsafe` forbid; `allow_attributes` deny

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §9

## Bağlam

Yapay zekaya "iyi yaz" demek işe yaramaz; kötü kodun derlenmesi, commit edilmesi ve "bitti" sayılması mekanik olarak imkansız olmalı.

## Karar

Tüm kapılar `cargo xtask verify`'a bağlıdır. `[workspace.lints]` tek tablodur; `unsafe_code = "forbid"`, `allow_attributes(_without_reason)` deny. Clippy `restriction` grubunun tamamı değil, seçilmiş liste açıktır. Ayrıntılar ADR 0021.

## Sonuçlar

Uyarılar `#[allow]` ile susturulamaz; gerekçeli `#[expect]` zorunlu.

## Reddedilen alternatifler

`restriction` grubunun tamamı (kendi içinde çelişkili), `cargo-pup` (sabit nightly).
