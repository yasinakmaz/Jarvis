# ADR 0024: SQLite sürücüsü: `rusqlite` (bundled) + tek yazar aktör

- **Durum:** Öneri (insan onayı bekliyor)
- **Tarih:** 2026-10-09
- **Mimari:** `docs/architecture.md` §8, §14 açık kararlar
- **Tasarım:** 0007

## Bağlam

Mimari §8 seçimi M1 başına bıraktı. Ölçütler: derleme zamanı sorgu denetimi, async uyumu,
bağımlılık ağırlığı.

| Ölçüt | `rusqlite` | `sqlx` |
| --- | --- | --- |
| Derleme zamanı sorgu denetimi | Yok (testlerle telafi) | Var (`query!`; `.sqlx/` çevrimdışı verisi veya `DATABASE_URL`) |
| Async | Hayır; aktör iş parçacığı gerekir | Evet |
| Bağımlılık ağırlığı | Küçük; `bundled` ile SQLite gömülü | Büyük (proc-macro, çoklu sürücü altyapısı) |
| Tek kullanıcı, tek dosya, düşük hacim | Uygun | Fazlası |
| Derleme süresi | Kısa | Uzun |

| Crate | Sürüm | Oluşturulma | Toplam indirme |
| --- | --- | --- | --- |
| `rusqlite` | 0.40.2 | 2014-11-21 | 121 milyon |
| `sqlx` | 0.9.0 | 2019-06-06 | 161 milyon |

Kaynak: `cargo xtask crate-info` (2026-10-09); hepsi eşikleri (180 gün, 100 000 indirme) geçiyor.

## Karar (öneri)

`rusqlite`, `bundled` özelliğiyle (sistem `libsqlite3` sürümünden bağımsız; CI konteyneri ve
masaüstü aynı SQLite'ı kullanır). Tüm erişim tek bir aktör iş parçacığında; async taraf
kanalla konuşur. Derleme zamanı denetiminin yerine her sorgu L5a testlerinde gerçek SQLite'a
karşı çalışır; değişen satır kapsaması ve mutasyon kapısı bunu zorlar.

## Sonuçlar

Tek yazar → kilit çekişmesi yok. Sorgu yazım hataları derleme değil test zamanında yakalanır.

## Reddedilen alternatifler

- `sqlx`: derleme zamanı denetimi değerli; ama `.sqlx/` bakımı, proc-macro ağırlığı ve tek
  kullanıcılı yerel daemon için gereksiz async havuz.
- `spawn_blocking` ile çağrı başına bağlantı: çoklu yazar, kilit çekişmesi.
