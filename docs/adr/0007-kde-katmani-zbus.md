# ADR 0007: KDE katmanını kendimiz yazıyoruz (`zbus`)

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §7

## Bağlam

KDE Plasma Wayland'de bilgisayar kullanımı için olgun bir kütüphane yok.

## Karar

Masaüstü katmanı `zbus` üzerinde yazılır; `reis`, `xkbcommon`, `atspi` yardımcıdır. Her yetenek bir trait arkasındadır ve sonuç hangi arka ucun çalıştığını raporlar.

## Sonuçlar

Tam denetim; M0'da doğrulanacak çok sayıda varsayım (docs/m0/checklist.md).

## Reddedilen alternatifler

Hazır karışık masaüstü kütüphaneleri: X11/uinput varsayımları, sessiz geri dönüş.
