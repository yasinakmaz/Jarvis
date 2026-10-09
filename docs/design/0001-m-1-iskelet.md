# Tasarım 0001: M-1 iskelet ve kalite kapıları

- **Durum:** İnsan onayı bekliyor (bu PR)
- **Kilometre taşı:** M-1
- **İlgili ADR'ler:** 0015, 0016, 0018, 0020, 0021
- **Crate'ler:** tüm crate iskeletleri, `xtask`

## Amaç

Kod yazılmaya başlamadan önce, kötü kodun derlenmesini, commit edilmesini ve "bitti"
sayılmasını mekanik olarak engelleyen altyapıyı kurmak (Mimari §9, §13 M-1).

## Kapsam dışı

- Herhangi bir ürün işlevi (M1+). Crate'ler yalnızca belgelenmiş boş iskelettir;
  `jarvisd` ve `jarvis` ikilileri `--version` dışında açıkça başarısız olur.
- `jarvis-fixture` (aracı M0 kararı), `e2e-real`, `release`, Gungraun kapısı (M3), RPM (M0 #11).
- Branch protection'ın kendisi: GitHub ayarıdır, `docs/runbooks/github.md`'de adım adım.

## Bileşenler

| Bileşen | Yer |
| --- | --- |
| Çalışma alanı, lint tablosu, release profili | `Cargo.toml` |
| Onaylı lint/profil kopyası | `xtask/src/baseline/workspace.toml` |
| Lint eşikleri, biçim, toolchain | `clippy.toml`, `rustfmt.toml`, `rust-toolchain.toml` |
| Katman sözleşmesi | `architecture.toml` |
| Tedarik zinciri | `deny.toml`, `supply-chain/allowlist.toml` |
| Kapı aracı | `xtask/` (`cargo xtask ...`, `.cargo/config.toml`) |
| Hook'lar | `.claude/settings.json`, `.claude/hooks/` |
| CI | `.github/workflows/ci.yml`, `.github/CODEOWNERS`, `.github/dependabot.yml` |

## `cargo xtask verify` akışı

```mermaid
flowchart TD
    start([verify]) --> fast{--fast?}
    fast -- hayır --> tools[araçlar kurulu mu?] --> base[taban ref çöz] --> snap[çalışma ağacı anlık görüntüsü]
    snap --> cache{Stop önbelleği: aynı taban + ağaç yeşil mi?}
    cache -- evet --> ok([yeşil])
    cache -- hayır --> s1
    fast -- evet --> s1
    s1[1 rustfmt --check] --> s2[2 dosya/satır boyutu, mod.rs]
    s2 --> s3[3 architecture.toml + lint mirası + lint tablosu]
    s3 --> s4[4 clippy -D warnings --locked]
    s4 --> f{--fast?}
    f -- evet --> ok
    f -- hayır --> s5[5 beyaz liste + cargo-deny + machete + hack]
    s5 --> s6[6 nextest, llvm-cov ile] --> s7[7 değişen satır kapsaması >= %80]
    s7 --> s8[8 cargo-mutants --in-diff] --> s9[9 sıcak yol: bench yoksa atlandı]
    s9 --> s10[10 cargo doc -D warnings] --> mark[önbelleğe yaz] --> ok
    s1 & s2 & s3 & s4 & s5 & s6 & s7 & s8 & s9 & s10 -. ilk hata .-> red([kırmızı, adım adıyla])
```

## Hook akışı

```mermaid
sequenceDiagram
    participant AI as Claude Code
    participant Pre as pre-tool-use.sh (python3)
    participant Post as xtask hook post-tool-use
    participant Stop as xtask hook stop
    AI->>Pre: Write/Edit/Bash girdisi (JSON)
    alt korunan yol
        Pre-->>AI: çıkış 2 + gerekçe (araç çalışmaz)
    else
        Pre-->>AI: çıkış 0
    end
    AI->>Post: .rs/.toml yazıldı
    Post-->>AI: verify --fast kırmızıysa çıkış 2 + çıktı
    AI->>Stop: "bitti"
    Stop-->>AI: verify kırmızıysa çıkış 2 (durmaya izin yok)
```

## Hata durumları

| Durum | Davranış |
| --- | --- |
| Eksik cargo aracı | `verify` kurulum komutuyla kırmızı |
| Taban ref yok | `verify` kırmızı; `--base <ref>`/`JARVIS_BASE_REF`/`--base root` önerir |
| `Cargo.lock` güncel değil | clippy `--locked` kırmızı |
| `python3` yok / hook girdisi bozuk | PreToolUse engeller (fail-closed) |
| xtask derlenemiyor | Post/Stop hook'ları çıkış 2 ile engeller; PreToolUse etkilenmez |
| Bench hedefi eklenmiş | Gungraun kurulana kadar kırmızı |

## Test planı

| Seviye | Test | Oracle |
| --- | --- | --- |
| L1 | `xtask` birim testleri: boyut, mod.rs, mimari ihlali, lint mirası, lint tablosu, beyaz liste, diff ayrıştırma, lcov, kapsama eşiği, tarih, `--expect-red` yorumu, hook tetikleme | Bilinen girdiler |
| L5a | `jarvisd`/`jarvis` ikilileri gerçek süreç olarak: `--version` ve açık başarısızlık | Çıkış kodu + akışlar |
| M-1 kanıtı | `cargo xtask selftest`: 13 bozulma × ilgili kapı, kontrol kopyası yeşil; 14 hook senaryosu | Kapı sonucu, gerçek betik çıkış kodu |

## Yeni bağımlılıklar

`anyhow`, `serde`, `serde_json`, `toml` (yalnızca `xtask`) — ADR 0020.

## Açık sorular

1. Depo gizliliği (özel/açık) ve `main` dalının oluşturulması — `docs/runbooks/github.md`.
2. Hook'ların gerçek Claude Code kurulumunda exit 2 ile engellediği (M0 #12).
