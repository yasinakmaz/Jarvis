# ADR 0010: Sessiz geri dönüş yok

- **Durum:** Kabul edildi
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §1 kural 4

## Bağlam

Hangi arka ucun çalıştığı bilinmezse doğrulama ve hata ayıklama imkansızlaşır.

## Karar

Reddedilen portal onayı ya da bozulan oturumda görev durur, `desktop.unavailable` olayı yayınlanır. Yedek yollar yalnızca açık yapılandırmayla devreye girer. Aynı ilke araçlara da uygulanır: eksik kalite aracı `verify`'ı kırmızı yapar, atlanan adım açıkça "atlandı" der.

## Sonuçlar

Daha fazla görünür hata; ama her sonuç izlenebilir.

## Reddedilen alternatifler

Otomatik arka uç değiştirme.
