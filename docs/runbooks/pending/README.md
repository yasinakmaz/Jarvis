# Bekleyen kapı yamaları (insan uygular)

Kalite kapısı dosyaları (`xtask/`, `.github/`, `.claude/`, ...) yapay zeka tarafından
değiştirilemez (Mimari §9 Katman 5). Yapay zeka gerekli değişikliği burada yama olarak
önerir; insan inceler ve uygular:

```sh
git apply docs/runbooks/pending/<yama>.patch
cargo xtask verify && cargo xtask selftest
git rm docs/runbooks/pending/<yama>.patch
```

## Yetki kuralı (Yasin Akmaz, 2026-10-09)

- **Kapı hatasını düzelten** yamalar (kapıyı gevşetmeyen; yanlış pozitifi, çökmeyi veya
  yanlış çalışan bir adımı düzelten) yapay zeka tarafından sorulmadan uygulanabilir. Her biri
  yine bu tabloya kaydedilir ve PR'da görünür.
- **Kapıyı gevşeten ya da kapsamını değiştiren** her değişiklik (eşik, lint, yasak listesi,
  korunan yol, atlanan adım) için her seferinde açık insan onayı gerekir.

| Yama | Neden | Durum |
| --- | --- | --- |
| `0001-cargo-run-ortam-sizintisi.patch` | `cargo run`'ın `CARGO_PKG_*` / `CARGO_MANIFEST_DIR` değişkenleri alt araçlara sızıyor, `cargo-machete` çöküyordu | Uygulandı (2026-10-09, kullanıcının açık izniyle) |
| `0002-m1-allowlist.patch` | M1 bağımlılıklarını (21 crate) beyaz listeye ekler; her biri bir ADR'ye bağlı (0004, 0022–0028) | Uygulandı (2026-10-09, kullanıcının açık izniyle) |
| `0003-m1-architecture.patch` | `jarvis-testkit` (`production = false`) ve provider/core/api/jarvisd için `dev_allowed` (ADR 0031) | Uygulandı (2026-10-09, kullanıcının açık izniyle) |
| `0004-cargo-hack-no-dev-deps.patch` | `cargo hack --no-dev-deps` dev-bağımlılıkları manifestten geçici silip `Cargo.lock`'u değiştiriyor, `--locked` ile çakışıyordu (ilk dev-bağımlılıkla ortaya çıktı); resolver 3'te bayrak gereksiz | Uygulandı (2026-10-09, kullanıcının açık izniyle) |
| `0005-selftest-bagimlilik-tablosu.patch` | Selftest bağımlılık bozulmaları ikinci bir `[dependencies]` başlığı ekleyip TOML'u bozuyordu (crate kendi tablosunu edinince); satır artık var olan tabloya ekleniyor | Uygulandı (2026-10-09, kullanıcının açık izniyle) |
| `0006-selftest-cargo-fetch.patch` | Selftest kopyaları `--offline` derlendiği için taze CI konteynerinde yeni bağımlılıklar bulunamıyordu; selftest başında `cargo fetch --locked` | Uygulandı (2026-10-09, kapı hatası kuralıyla); boş `CARGO_HOME` ile yeniden üretilip düzeltildiği doğrulandı |

Uygulamadan önce kontrol (yalnızca okur): yamalar `git apply --check` ile temiz; bellekte
uygulandığında `architecture.toml` geçerli TOML, allowlist'teki her ADR mevcut.
