# Kasetler

Bu dizindeki `nvidia-*.jsonl` dosyaları **elle yazılmıştır**: NVIDIA'nın OpenAI uyumlu ucunun
(`integrate.api.nvidia.com/v1`) belgelenmiş yanıt biçimini (`tool_calls: []`, `stop_reason`,
`prompt_logprobs`, `chatcmpl-tool-…` kimlikleri, SSE parçaları) taklit eder; canlı kayıt değildir.

Canlı kayıt PR 12'de (`JARVIS_CASSETTE_MODE=record`, yalnızca yerelde) alınacak ve bu dosyaların
yerini alacaktır. O zamana dek bu testler "NVIDIA biçimli" olarak adlandırılır; sağlayıcıyla
gerçek uyum kanıtı sayılmaz.

Biçim: Tasarım 0011 (satır başına bir tur). İstek gövdesindeki `model`, testlerde
`meta/test-model`dir (yapılandırmadaki model); yanıttaki `model` NVIDIA'nın bildirdiğidir.
