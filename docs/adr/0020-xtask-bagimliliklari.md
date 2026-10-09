# ADR 0020: xtask bağımlılıkları (`anyhow`, `serde`, `serde_json`, `toml`)

- **Durum:** Kabul edildi (2026-10-09, Yasin Akmaz onayı)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §9 Katman 4

## Bağlam

`xtask`, `architecture.toml`, `supply-chain/allowlist.toml`, `Cargo.toml` (TOML) ile
`cargo metadata`, crates.io API ve Claude Code hook girdisini (JSON) ayrıştırmak zorunda.
Bu ayrıştırıcıları elle yazmak hem hataya açık hem de kapı kodunu büyütür.

## Karar

`xtask` yalnızca şu dört crate'e bağlanır; hiçbiri üretim ikililerine girmez
(`xtask` `production = false`, `architecture.toml`):

| Crate | Sürüm | Oluşturulma | Toplam indirme | Depo |
| --- | --- | --- | --- | --- |
| `anyhow` | 1.0.104 | 2019-10-05 | ~1,03 milyar | github.com/dtolnay/anyhow |
| `serde` | 1.0.229 | 2014-12-05 | ~1,50 milyar | github.com/serde-rs/serde |
| `serde_json` | 1.0.151 | 2015-08-07 | ~1,41 milyar | github.com/serde-rs/json |
| `toml` | 1.1.8 | 2014-11-11 | ~0,99 milyar | github.com/toml-rs/toml |

Değerler `cargo xtask crate-info anyhow serde serde_json toml` çıktısıdır (2026-10-09);
dördü de yaş ve indirme eşiklerini (180 gün, 100 000 indirme) geçer.

HTTP için crate eklenmez: `crate-info` sistemdeki `curl`'ü çağırır. CLI ayrıştırma elle
yapılır (`clap` eklenmez).

## Sonuçlar

- Kapı kodu küçük ve okunur kalır; tüm ayrıştırma tipli ve `deny_unknown_fields` ile katıdır.
- `serde`, `serde_json`, `toml` M1'de üretim crate'lerinde de kullanılacaktır; o zaman
  bu ADR'ye atıfla aynı beyaz liste girişleri geçerlidir. `anyhow` üretim kodunda
  kullanılmaz (kütüphanelerde tipli hatalar, `thiserror` kararı M1 ADR'sinde).

## Reddedilen alternatifler

- `cargo_metadata` crate'i: tek kullanım için ek geçişli bağımlılık; ihtiyaç duyulan alanlar
  birkaç `serde` yapısıyla okunuyor.
- `clap`: dört alt komut için gereksiz ağırlık.
- `ureq`/`reqwest`: crates.io sorgusu için `curl` yeterli; TLS yığını eklemek gereksiz.
