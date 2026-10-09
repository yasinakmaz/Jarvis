//! Depolama hataları. `rusqlite` tipleri dışarı sızmaz; ayrıntı metin olarak taşınır.

use std::path::PathBuf;

use jarvis_types::SessionId;

/// Depolama hatası.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// Dosyadaki şema bu sürümün bildiğinden yeni; daemon başlamaz.
    #[error(
        "veritabanı şeması {found}, bu sürüm en çok {supported} destekliyor; Jarvis'i güncelleyin"
    )]
    NewerSchema {
        /// Dosyadaki sürüm.
        found: i64,
        /// Bu sürümün desteklediği.
        supported: i64,
    },
    /// Migrasyondan önceki yedek yazılamadı; migrasyon yapılmadı.
    #[error("yedek yazılamadı ({}): {detail}; migrasyon yapılmadı", path.display())]
    Backup {
        /// Yedek dosyası.
        path: PathBuf,
        /// Neden.
        detail: String,
    },
    /// Veritabanı dosyası `0600` değil.
    #[error("{} izinleri {mode:o}; 0600 olmalı (chmod 600)", path.display())]
    Permissions {
        /// Dosya.
        path: PathBuf,
        /// Bulunan izin bitleri.
        mode: u32,
    },
    /// Dosya sistemi hatası.
    #[error("dosya sistemi: {0}")]
    Io(String),
    /// `SQLite` hatası (denetim tetikleyicisinin reddi dahil).
    #[error("veritabanı: {0}")]
    Database(String),
    /// Kayıt okunamadı (bozuk satır).
    #[error("bozuk kayıt: {0}")]
    Corrupt(String),
    /// Oturum yok.
    #[error("oturum yok: {0}")]
    UnknownSession(SessionId),
    /// Değer `SQLite` tamsayısına sığmıyor.
    #[error("değer veritabanı aralığının dışında: {0}")]
    OutOfRange(&'static str),
    /// Aktör iş parçacığı durmuş; depolama kullanılamaz.
    #[error("depolama kapalı (aktör iş parçacığı durdu)")]
    Closed,
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
