# Jarvis

KDE Plasma Wayland masaüstünde süreç çalıştırabilen ve fare/klavye ile bilgisayar
kullanabilen tek bir Rust servisi (`jarvisd`) ve OpenAI uyumlu HTTP API'si.

- Mimari: [`docs/architecture.md`](docs/architecture.md)
- Kararlar: [`docs/adr/`](docs/adr/) · Tasarımlar: [`docs/design/`](docs/design/)
- Katkı ve yapay zeka kuralları: [`AGENTS.md`](AGENTS.md)

## Durum

**M-1 İskelet** — çalışma alanı, kalite kapıları (`cargo xtask verify`), Claude Code hook'ları
ve CI hazır. Ürün işlevi henüz yok; sıradaki adımlar M0 fizibilite ve M1 çekirdek + API.

## Geliştirme

```sh
cargo binstall cargo-nextest cargo-deny cargo-llvm-cov cargo-mutants cargo-machete cargo-hack
cargo xtask verify          # tüm kapılar (taban: origin/main)
cargo xtask verify --fast   # fmt, boyut, mimari, clippy
cargo xtask selftest        # kapıların bozuk kodu reddettiğinin kanıtı
```

Hedef ortam: Fedora 44, KDE Plasma 6.7.5 (Wayland), Rust 1.97.0 (`rust-toolchain.toml`).
