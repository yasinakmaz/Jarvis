# ADR 0011: Türkçe Q keysym duyarlı yazım

- **Durum:** Öneri — M0 #3 ile doğrulanacak
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §7

## Bağlam

Kullanıcı Türkçe Q klavye kullanıyor; `ğ ü ş ı ö ç İ` birinci sınıf desteklenmeli.

## Karar

Karakter, kullanıcının gerçek keymap'inden (keycode + değiştirici, `xkbcommon`) bulunur. Haritada olmayan karakter Klipper panosu + Ctrl+V ile yazılır.

## Sonuçlar

Doğruluk `jarvis-fixture` ile bağımsız olarak doğrulanır (L5c).

## Reddedilen alternatifler

Yalnızca ABD düzeni.
