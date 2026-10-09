# ADR 0009: KWin özel EIS: varsayılan KAPALI, yalnızca test

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §7, §10 L5b

## Bağlam

KWin'in özel EIS arayüzü sanal oturumda (L5b) girdi testine izin veriyor ama dahili bir API.

## Karar

KWin EIS arka ucu yalnızca `test-backend` feature'ıyla derlenir; üretim ikilisinde bu kod yoktur.

## Sonuçlar

L5b testleri mümkün; üretim kararlılığı dahili API'ye bağlanmaz.

## Reddedilen alternatifler

Üretimde birincil yol olarak KWin EIS.
