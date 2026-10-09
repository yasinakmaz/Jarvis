# ADR 0006: Onaylar yalnızca uygulamada (veya terminal istemcisinde)

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §1 kural 6, §6

## Bağlam

Onay yolu ne kadar çoksa atlatma yüzeyi o kadar büyür.

## Karar

Onaylar yalnızca `GET /approvals` / `POST /approvals/{id}` üzerinden, uygulama veya terminal istemcisiyle verilir. KDE bildirimi gönderilmez. Onay gösterilen argümanların hash'ine bağlıdır.

## Sonuçlar

Tek, denetlenebilir yol. Kullanıcı istemciyi açık tutmalı; zaman aşımı ret sayılır.

## Reddedilen alternatifler

KDE bildirim düğmeleri: ikinci yol, bildirim sahteciliği riski.
