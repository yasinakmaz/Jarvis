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
