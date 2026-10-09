# Bekleyen kapı yamaları (insan uygular)

Kalite kapısı dosyaları (`xtask/`, `.github/`, `.claude/`, ...) yapay zeka tarafından
değiştirilemez (Mimari §9 Katman 5). Yapay zeka gerekli değişikliği burada yama olarak
önerir; insan inceler ve uygular:

```sh
git apply docs/runbooks/pending/<yama>.patch
cargo xtask verify && cargo xtask selftest
git rm docs/runbooks/pending/<yama>.patch
```

| Yama | Neden |
| --- | --- |
| `0001-cargo-run-ortam-sizintisi.patch` | `cargo xtask` `cargo run` ile çalıştığında `CARGO_PKG_*` / `CARGO_MANIFEST_DIR` alt araçlara sızıyor; `cargo-machete` bunları görünce argümanları yol sanıp çöküyor ve `verify` "eksik araç: cargo-machete" ile kırmızı oluyor. Derlenmiş ikili doğrudan çalıştırıldığında (`target/debug/xtask verify --base root`) 10/10 adım 17,6 sn'de yeşil. |
