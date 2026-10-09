# ADR 0030: Otomatik sağlayıcı değiştirme yok

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §5, §14 açık kararlar
- **Tasarım:** 0006

## Bağlam

§5 öneri olarak "v1'de otomatik sağlayıcı değiştirme yok" dedi; §14'te açık karar.

## Karar

Bir rol tek bir sağlayıcıya bağlıdır. Denemeler tükendikten sonra da yanıt yoksa koşu
`provider_unavailable` (veya `provider_rejected`) ile biter; başka sağlayıcıya geçilmez.
Geçiş, yapılandırma değişikliği + yeniden başlatmadır.

## Sonuçlar

ADR 0010 (sessiz geri dönüş yok) ile tutarlı; eval sonuçları hangi modelin çalıştığı
konusunda belirsizlik taşımaz. Bedeli: NVIDIA kesintisinde koşular başarısız olur.

## Reddedilen alternatifler

- Sıralı sağlayıcı listesi: farklı modeller farklı araç çağrısı davranışı gösterir; eval
  taban çizgisini ve hata ayıklamayı bozar.
