# M0 doğrulama listesi

Mimari §14. Her madde gerçek Fedora 44 / Plasma 6.7.5 Wayland oturumunda, `spikes/`
altındaki bağımsız denemelerle sınanır. Sonuç "geçti / kaldı" + kanıt (komut çıktısı,
ekran görüntüsü, ölçüm) olarak yazılır; ardından ilgili ADR ve `docs/architecture.md`
güncellenir.

| # | Doğrulanacak | Başarısız olursa | Spike | Sonuç | Tarih | Kanıt / not |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Portal: `CreateSession` + `SelectDevices` (`persist_mode=2`) + `Start`; tek seferlik onay; yeniden başlatmada restore | KWin EIS opt-in ADR'si yeniden açılır | `spikes/portal-session` | bekliyor | | |
| 2 | `reis`: el sıkışma, ping yanıtı, mutlak tıklama, `sync` onayı | libei doğrudan veya başka crate | `spikes/reis-input` | bekliyor | | |
| 3 | Türkçe Q: `ğ ü ş ı ö ç İ` (xkbcommon); Klipper yedeği ve pano geri yükleme | Pano yolu birincil | `spikes/keymap-tr` | bekliyor | | |
| 4 | ScreenShot2 yetkisi: `.desktop` + `Exec=` eşleşmesi, `kbuildsycoca6` | Screenshot portalı / Spectacle birincil | `spikes/screenshot2` | bekliyor | | |
| 5 | AT-SPI: Firefox, Dolphin, Kate, Chromium; koordinat düzeltmesi; kısmi ölçekleme | Koordinat + VLM ağırlık kazanır | `spikes/atspi-tree` | bekliyor | | |
| 6 | KWin scripting: pencere listesi, etkinleştirme, istemci başlangıcı, sonuç servisi | Alternatif pencere erişimi | `spikes/kwin-script` | bekliyor | | |
| 7 | KWin EIS'in 6.7.5'te çağrılabilirliği | L5b yalnızca süreç/pencere testleri | `spikes/kwin-eis` | bekliyor | | |
| 8 | Odaklanmamış pencereye tıklama ve etkinleştirme sonrası bekleme | Her tıklamadan önce etkinleştirme | `spikes/reis-input` | bekliyor | | |
| 9 | Portalda `Notify*` mutlak işaretçi, akışsız | EIS bölge tabanlı yol | `spikes/portal-session` | bekliyor | | |
| 10 | Sanal KWin'de ScreenShot2 veri veriyor mu | Ekran testleri L5c'ye kayar | `spikes/kwin-virtual` | bekliyor | | |
| 11 | RPM: `cargo-generate-rpm`, kurulum, dev/prod kimlik ayrımı | `rpmbuild` spec | `spikes/rpm` | bekliyor | | |
| 12 | Claude Code hook davranışı (exit 2 engelleme, `Stop`) | `pre-commit` + CI zorlaması | `.claude/` (aşağıda) | bekliyor | | |
| 13 | KDE global kısayoluyla `halt` (M3) | `POST /halt` yeterli | `spikes/halt-shortcut` | bekliyor | | |

## #12 için el ile deneme

Depo kökünde Claude Code açıkken:

1. "clippy.toml'daki 60'ı 600 yap" deyin → `ENGELLENDİ: clippy.toml ...` görülmeli, dosya
   değişmemeli (`git diff --exit-code clippy.toml`).
2. `crates/jarvis-types/src/lib.rs`'e `unwrap()` içeren bir fonksiyon yazdırın →
   PostToolUse clippy hatasını modele geri beslemeli.
3. Hata düzeltilmeden "bitti" denmesini isteyin → Stop hook'u durmayı engellemeli.

Otomatik karşılığı (betik düzeyi): `cargo xtask selftest` hook senaryoları.
