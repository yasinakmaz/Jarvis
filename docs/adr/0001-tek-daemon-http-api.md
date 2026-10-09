# ADR 0001: Tek daemon (`jarvisd`) ve HTTP API

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §1, §2, §4

## Bağlam

Ajanın masaüstünü, süreçleri ve onay akışını yöneten tek bir yetki noktası olmalı. Uygulama (M5) ve terminal istemcisi aynı yeteneklere erişmeli.

## Karar

Tüm ajan mantığı tek `jarvisd` sürecinde, tek bir `tokio` çalışma zamanında çalışır. Dış dünya yalnızca `127.0.0.1` üzerindeki HTTP API ile konuşur. Sınırlar süreçler arasında değil crate'ler arasındadır.

## Sonuçlar

Tek yönetim, tek denetim kaydı, tek onay yolu. İstemciler ince kalır. Bir crate'teki hata tüm daemon'u etkileyebilir; bu yüzden `panic = "abort"` + systemd `Restart=on-failure` + yasaklı `unwrap`/`panic`.

## Reddedilen alternatifler

Ayrı süreçler (masaüstü, ajan, API): IPC karmaşıklığı ve yetki dağılması. gRPC: OpenAI uyumluluğu ve istemci ekosistemi kaybı.
