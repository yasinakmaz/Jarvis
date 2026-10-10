# ADR 0022: Kütüphane hata tipleri: `thiserror`

- **Durum:** Kabul edildi (2026-10-09, Yasin Akmaz onayı)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §9
- **Tasarım:** 0003–0010

## Bağlam

Kütüphane crate'leri çağırana eşlenebilir, tipli hatalar vermeli (API `ErrorCode`'a eşleme,
testlerde desen eşleme). `unwrap`/`panic` yasak olduğu için her hata yolu açık bir tip ister.

## Karar

Her crate kendi `Error` enum'unu `thiserror` ile tanımlar. `anyhow` yalnızca `xtask`'ta kalır;
`jarvisd` `main` dahil üretim kodunda kullanılmaz. Hata mesajları Türkçe, `redact`'ten geçer.

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `thiserror` | 2.0.21 | 2019-10-09 | 1,59 milyar |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Sonuçlar

Hata türleri derleme zamanında görünür; API eşlemesi kapsamlı `match` ile denetlenir.

## Reddedilen alternatifler

- Elle `impl Display/Error`: tekrar eden kod, 60 satır sınırını zorlar.
- `anyhow` kütüphanelerde: tip bilgisi kaybolur, `ErrorCode` eşlemesi metne dayanır.
- `snafu`: daha ağır; ek fayda yok.
