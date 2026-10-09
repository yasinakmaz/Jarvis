# ADR 0027: Gizli değerler `secrecy`; API token'ı sabit dosya + açık döndürme

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §4, §8, §14 açık kararlar
- **Tasarım:** 0006, 0010

## Bağlam

§14 açık kararı: token dosyasının yeri ve döndürülmesi. §8: anahtarlar günlüğe, olaya, hataya
yazılmaz.

## Karar

- Sağlayıcı anahtarları bellekte `secrecy::SecretString` (Debug maskeli, bırakılırken
  sıfırlanır); metin yollarında `jarvis-types::redact`.
- Token: `~/.config/jarvis/token`, `0600`, `getrandom` ile 32 bayt → onaltılık. Döndürme
  yalnızca açık komutla: `jarvisd --rotate-token` (+ servis yeniden başlatma). Otomatik
  döndürme yok (istemcileri sessizce kırar). İzni geniş dosyayla daemon başlamaz.

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `secrecy` | 0.10.3 | 2018-10-04 | 176 milyon |
| `getrandom` | 0.4.3 | 2019-01-19 | 2,21 milyar |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Sonuçlar

Tek, denetlenebilir token yolu; istemciler (`jarvis`, uygulama) aynı dosyayı okur.

## Reddedilen alternatifler

- Dönen kısa ömürlü token: tek kullanıcılı loopback için karmaşıklık > fayda.
- `rand`: yalnızca rastgele bayt gerekiyor; `getrandom` daha küçük.
- Secret Service / KWallet: §8'e göre uygulama aşamasına bırakıldı.
