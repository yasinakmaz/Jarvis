# ADR 0004: Sağlayıcı istemcisi `async-openai`

- **Durum:** Kabul edildi (bağımlılık eklenirken beyaz liste girişi ayrıca onaylanır)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §5

## Bağlam

OpenAI uyumlu sağlayıcılarla konuşmak için tipler ve istemci gerekiyor; aynı tipler API sözleşme testlerinde de kullanılacak.

## Karar

`jarvis-provider` `async-openai` kullanır; `base_url` yapılandırılabilir.

## Sonuçlar

Tek crate hem istemci hem tip sağlar. Crate M1'de eklenirken `cargo xtask crate-info async-openai` çıktısı bu ADR'ye eklenir ve `supply-chain/allowlist.toml`'a insan onayıyla girer.

## Reddedilen alternatifler

El yapımı HTTP istemcisi: tip bakımı ve uyumsuzluk riski.
