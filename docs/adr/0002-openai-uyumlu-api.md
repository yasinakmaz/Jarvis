# ADR 0002: OpenAI uyumlu API; yerel geçiş `base_url` ile

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §1 kural 2, §4, §5

## Bağlam

Bugün barındırılan modeller (build.nvidia.com), yarın yerel modeller (Ollama/llama.cpp) kullanılacak. Hem sunduğumuz API hem tükettiğimiz sağlayıcılar için standart bir sözleşme gerekiyor.

## Karar

`/v1` altında OpenAI sözleşmesi izlenir; `model: "jarvis"` sunucu tarafı ajan döngüsünü çalıştırır, rol adları sağlayıcıya yönlendirilir. Sağlayıcı değişimi yalnızca yapılandırmadaki `base_url` değişikliğidir.

## Sonuçlar

Üçüncü taraf istemciler doğrudan kullanılabilir; uyum `async-openai` istemcisiyle test edilir (L4). Jarvis'e özgü uçlar `/v1` dışında durur. `/v1` içinde yalnızca ekleyici değişiklik yapılır.

## Reddedilen alternatifler

Özel protokol: istemci yazma yükü, araç ekosistemi kaybı.
