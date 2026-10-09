# ADR 0003: Ajan döngüsü kendi mantığımızda

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §3

## Bağlam

Onay, masaüstü kilidi, iptal ve bağımsız doğrulama ajan döngüsünün her adımına gömülü olmalı.

## Karar

Ajan döngüsü `jarvis-core` içinde kendimiz yazılır (gözlem → planlama → risk → onay → yürütme → doğrulama → döngü denetimi).

## Sonuçlar

Tam denetim ve test edilebilirlik (StubLlm/FakeDesktop/FakeClock ile L2). Bakım yükü bizde.

## Reddedilen alternatifler

Hazır ajan çerçeveleri: onay/kilit/iptal semantiğini dışarıdan zorlamak gerekir; sessiz davranış riski.
