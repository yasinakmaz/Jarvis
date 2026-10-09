//! Oturum ve mesaj depolama (`SQLite`) ve migrasyonlar (Tasarım 0007).
//!
//! Oturumlar, mesajlar, koşular ve yalnızca eklemeli denetim kaydı tek `SQLite` dosyasındadır.
//! Migrasyonlar yalnızca ileri yönlüdür ve önce yedek alınır; daha yeni bir şema görülürse
//! açma reddedilir. `rusqlite` engelleyici olduğu için tüm erişim tek bir aktör iş
//! parçacığındadır; async taraf kanalla konuşur.

mod actor;
pub mod error;
pub mod model;
mod open;
mod queries;
pub mod repo;
pub mod store;

pub use error::StoreError;
pub use model::{AuditRow, Page, RunOutcome, RunRecord, SessionSummary};
pub use repo::SessionRepo;
pub use store::Store;

/// Bu sürümün şema sürümü.
pub const SCHEMA_VERSION: i64 = 1;
