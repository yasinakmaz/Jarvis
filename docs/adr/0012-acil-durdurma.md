# ADR 0012: Acil durdurma: `POST /halt` (+ M3'te KDE kısayolu)

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §3

## Bağlam

Kaçak döngü veya yanlış girdiye karşı her zaman erişilebilir bir kesici gerekiyor.

## Karar

`POST /halt` tüm koşuları iptal eder, kilidi bırakır, kuyruğu boşaltır, `halt` olayı yayınlar. KDE global kısayolu M3'te denenecek (M0 #13).

## Sonuçlar

Kesici API üzerinden her istemciden erişilebilir.

## Reddedilen alternatifler

Yalnızca uygulama düğmesi.
