# ADR 0005: Oturum `x-jarvis-session` başlığıyla, sunucuda

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §4

## Bağlam

İstemcilerin her istekte tüm geçmişi göndermesi hem maliyetli hem hataya açık; ajan durumu sunucuda.

## Karar

Oturum `x-jarvis-session` başlığıyla seçilir; başlık yoksa oturum geçicidir ve saklanmaz. Geçmiş SQLite'ta tutulur.

## Sonuçlar

İnce istemci. OpenAI istemcileri başlığı gönderemiyorsa geçici oturumla çalışır.

## Reddedilen alternatifler

İstemcinin tüm geçmişi göndermesi.
