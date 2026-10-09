# GitHub kurulumu (insan adımları)

Bu ayarlar koddan yapılamaz; depo sahibi bir kez uygular (Mimari §11).

## 1. Depo gizliliği

Settings → General → Danger Zone → visibility: **Private** (mimari varsayımı).
Self-hosted runner kullanılmadığı için (ADR 0017) açık depo da güvenlidir, ancak
karar §14 açık kararlarındadır.

## 2. `main` dalı

İlk iş `claude/...` dalında geldi. `main`'i bu dalın onaylanmış hâlinden oluşturun ve
varsayılan dal yapın:

```sh
git fetch origin
git push origin origin/<iskelet-dalı>:refs/heads/main
```

Settings → General → Default branch → `main`.

## 3. Branch protection (`main`)

Settings → Branches → Add rule (veya Rulesets):

- Require a pull request before merging (onay sayısı 0 ya da 1; tek geliştirici).
- Require review from Code Owners (CODEOWNERS korunan dosyalar için).
- Require status checks to pass: `verify`, `selftest`, `build`.
- Require branches to be up to date before merging.
- Do not allow bypassing the above settings; force push ve silme kapalı.

## 4. Actions ayarları

- Settings → Actions → General → Fork pull request workflows: **çalıştırma**.
- Workflow permissions: **Read repository contents** (varsayılan en düşük yetki).
- Settings → Environments → `nvidia` ortamı (M1): yalnızca `main` dalı, secret
  `NVIDIA_API_KEY`.

## 5. Yerel araçlar

```sh
cargo binstall cargo-nextest cargo-deny cargo-llvm-cov cargo-mutants cargo-machete cargo-hack
cargo xtask verify --base origin/main
cargo xtask selftest
```
