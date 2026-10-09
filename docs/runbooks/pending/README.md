# Bekleyen kapı yamaları (insan uygular)

Kalite kapısı dosyaları (`xtask/`, `.github/`, `.claude/`, ...) yapay zeka tarafından
değiştirilemez (Mimari §9 Katman 5). Yapay zeka gerekli değişikliği burada yama olarak
önerir; insan inceler ve uygular:

```sh
git apply docs/runbooks/pending/<yama>.patch
cargo xtask verify && cargo xtask selftest
git rm docs/runbooks/pending/<yama>.patch
```

| Yama | Neden | Durum |
| --- | --- | --- |
| `0001-cargo-run-ortam-sizintisi.patch` | `cargo run`'ın `CARGO_PKG_*` / `CARGO_MANIFEST_DIR` değişkenleri alt araçlara sızıyor, `cargo-machete` çöküyordu | Uygulandı (2026-10-09, kullanıcının açık izniyle) |
| `0002-m1-allowlist.patch` | M1 bağımlılıklarını (21 crate) beyaz listeye ekler; her biri bir ADR'ye bağlı (0004, 0022–0028) | M1 tasarımları onaylanınca uygulanacak |
| `0003-m1-architecture.patch` | `jarvis-testkit` (`production = false`) ve provider/core/api/jarvisd için `dev_allowed` (ADR 0031) | M1 tasarımları onaylanınca uygulanacak |

Uygulamadan önce kontrol (yalnızca okur): yamalar `git apply --check` ile temiz; bellekte
uygulandığında `architecture.toml` geçerli TOML, allowlist'teki her ADR mevcut.
