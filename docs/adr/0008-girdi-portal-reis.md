# ADR 0008: Girdi: XDG RemoteDesktop portalı + `reis`

- **Durum:** Öneri — M0 #1, #2 ile doğrulanacak
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §7

## Bağlam

Wayland'de girdi enjeksiyonu için kullanıcı onaylı, kalıcı ve denetlenebilir bir yol gerekiyor.

## Karar

`CreateSession` → `SelectDevices` (klavye+fare, `persist_mode=2`) → `Start` → `ConnectToEIS` → libei (`reis`). `uinput`, `ydotool`, `fake_input`, XTEST kullanılmaz; `deny.toml` bu crate'leri yasaklar.

## Sonuçlar

Tek seferlik kullanıcı onayı + restore. Portal oturumu D-Bus bağlantısıyla öldüğünden bağlantı daemon ömrü boyunca açık kalır.

## Reddedilen alternatifler

`uinput`/`ydotool`: root/grup yetkisi, onaysız. `fake_input`: KWin dahili. XTEST: Wayland'de yok.
