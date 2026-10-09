# ADR 0021: Kalite kapısı uygulama ayrıntıları (M-1)

- **Durum:** Kabul edildi (insan onayı bekliyor: bu PR'ın gözden geçirilmesi)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §9, §13 M-1
- **Tasarım:** `docs/design/0001-m-1-iskelet.md`

## Bağlam

Mimari §9 kapıları tanımlıyor ama bazı uygulama ayrıntılarını açık bırakıyor: lint
tablosu kök `Cargo.toml`'da durduğu için dosya düzeyinde korunamaz; hook'ların
derleme durumundan bağımsız çalışması gerekir; diff tabanlı kapıların bir taban ref'e
ihtiyacı vardır.

## Karar

1. **Lint tablosu koruması.** Kök `Cargo.toml` yapay zekaya açıktır (üye/bağımlılık
   eklemek için), ama `[workspace.lints]` ve `[profile]` tabloları
   `xtask/src/baseline/workspace.toml` ile birebir aynı olmalıdır; fark `verify`'da
   kırmızıdır. Her crate `[lints]` olarak tam `workspace = true` taşımalıdır.
2. **Korunan yollar** (PreToolUse, çıkış 2): `clippy.toml`, `rustfmt.toml`,
   `rust-toolchain.toml`, `architecture.toml`, `deny.toml`, `xtask/`, `.github/`,
   `.claude/`, `.cargo/`, `supply-chain/`. Aynı liste `.github/CODEOWNERS`'ta.
3. **PreToolUse Python'dadır** (`.claude/hooks/pre_tool_use.py`): cargo derlemesine bağlı
   olsaydı bozuk bir `Cargo.toml` her aracı kilitler ve kendini düzeltemezdi.
   `python3` yoksa ya da girdi değerlendirilemezse engeller (fail-closed). Bash denetimi
   en iyi çabadır; asıl zorlama CODEOWNERS + branch protection + CI'dır.
4. **PostToolUse** yalnızca `.rs`/`.toml` yazımlarında `verify --fast` çalıştırır.
   **Stop** tam `verify` çalıştırır; aynı çalışma ağacı (git ağaç nesnesi) ve taban için
   önceki yeşil sonuç önbellekten kabul edilir.
5. **Taban ref:** `--base` > `JARVIS_BASE_REF` > `origin/main`; bulunamazsa kırmızı
   (sessiz geri dönüş yok). `--base root` boş ağaca göre karşılaştırır (ilk commit).
   Değişiklikler, izlenmeyen dosyalar dahil, geçici bir git index'iyle alınır.
6. **Diff kapsaması ve mutasyon yalnızca `crates/` altında** zorlanır; `xtask` kapı
   aracıdır, birim testleri vardır ama kapsama/mutasyon kapsamı dışındadır.
7. **Dosya boyutu:** sütun 0'daki ilk `#[cfg(test)]` satırından sonrası ve `tests/`,
   `benches/`, `tests.rs`, `*_tests.rs` dosyaları sayılmaz. Satır genişliği (100) tüm
   `.rs` dosyalarında zorlanır, çünkü `rustfmt` uzun dizge ve yorumları kırmaz.
8. **Eksik araç kırmızıdır:** `cargo-nextest`, `cargo-deny`, `cargo-llvm-cov`,
   `cargo-mutants`, `cargo-machete`, `cargo-hack` yoksa `verify` kurulum komutuyla durur.
9. **Performans kapısı:** bench hedefi yokken açıkça "atlandı" der; bench hedefi eklenip
   Gungraun karşılaştırması kurulmamışsa kırmızıdır (M3'te insan tarafından eklenir).
10. **`cargo-deny` çoğaltma** şimdilik uyarıdır; M1'de ağır bağımlılıklar gelince
    yeniden değerlendirilir. `uinput`/XTEST/xdo ve otomatik güncelleme crate'leri yasaktır.
11. **`missing_docs` deny**; `unreachable_pub` açılmadı (Clippy `redundant_pub_crate` ile
    çelişir, mimari belgede istenmiyor).
12. **M-1 kanıtı:** `cargo xtask selftest` çalışma alanının kopyalarına 13 bozulma
    uygular (uzun dosya/satır, mantıklı `mod.rs`, `unwrap`, `#[allow]`, `unsafe`, `panic`,
    yukarı bağımlılık, çekirdekte HTTP, tanımsız crate, beyaz liste dışı bağımlılık,
    gevşetilmiş lint tablosu, lint mirasının kaldırılması) ve her birinin reddedildiğini,
    ayrıca yazma engeli hook'unun 14 senaryoda doğru karar verdiğini doğrular. CI'da çalışır.

## Sonuçlar

Kapılar yapay zeka tarafından gevşetilemez; insan değişiklikleri CODEOWNERS ile görünür
olur. Bedeli: kapı değişikliği her zaman insan elinden geçer; tam `verify` dakikalar
sürer (kapsama + mutasyon), bu yüzden Stop önbelleği vardır.

## Reddedilen alternatifler

- Kök `Cargo.toml`'u tamamen korumak: yapay zeka crate/bağımlılık ekleyemezdi.
- PreToolUse'u `cargo xtask` ile yazmak: derleme bozulunca kilitlenme (yukarıda).
- Taban yoksa tüm depoya sessizce geri dönmek: ADR 0010'a aykırı.
