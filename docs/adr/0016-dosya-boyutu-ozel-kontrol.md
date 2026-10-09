# ADR 0016: Dosya boyutu kontrolü özel yazılır

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §9 Katman 2

## Bağlam

Clippy'de kararlı bir dosya uzunluğu lint'i yok.

## Karar

`xtask` içinde rustc `tidy` benzeri kontrol: dosya 300 satır (testler hariç), satır 100 sütun, `mod.rs` yalnızca `pub mod`/`pub use`, crate başına ~15 dosyada uyarı. İstisna: `// xtask: allow-long-file: <gerekçe>`.

## Sonuçlar

Hata mesajları ne yapılacağını söyler (yapay zekaya yönelik düzeltme talimatı).

## Reddedilen alternatifler

Üçüncü taraf lint beklemek.
