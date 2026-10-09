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

### Crate denetimi (2026-10-09, `cargo xtask crate-info async-openai`)

| Crate | Sürüm | Oluşturulma | Toplam indirme | Depo |
| --- | --- | --- | --- | --- |
| `async-openai` | 0.42.1 | 2022-12-02 | ~9,1 milyon | github.com/64bit/async-openai |

Eşikleri (180 gün, 100 000 indirme) geçiyor. Özellikler: varsayılan `rustls` (native-tls yok);
`jarvis-provider` → `chat-completion`, `model`, `byot` (passthrough için ham JSON);
`jarvis-api` → yalnızca `chat-completion-types`, `model-types` (HTTP istemcisi derlenmez,
ADR 0029).

## Reddedilen alternatifler

El yapımı HTTP istemcisi: tip bakımı ve uyumsuzluk riski.
