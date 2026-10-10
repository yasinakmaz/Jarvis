//! Zaman damgaları ve enjekte edilen saat (Tasarım 0003, ADR 0026).
//!
//! Zaman her zaman [`Clock`] üzerinden alınır; testlerde sahte saat kullanılır.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// `Send` bir kutulanmış gelecek; trait nesnelerinde async dönüş tipi.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Enjekte edilen saat. Gerçeği `jarvisd`'de, sahtesi `jarvis-testkit`'te.
pub trait Clock: Send + Sync {
    /// Şimdiki zaman (UTC).
    fn now(&self) -> Timestamp;

    /// `duration` kadar bekler.
    fn sleep(&self, duration: Duration) -> BoxFuture<'_, ()>;
}

/// Zaman hataları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimeError {
    /// Metin RFC 3339 değil.
    #[error("RFC 3339 zaman damgası ayrıştırılamadı: `{0}`")]
    Parse(String),
    /// Değer desteklenen aralığın (yıl 0–9999) dışında.
    #[error("zaman damgası desteklenen aralığın (yıl 0–9999) dışında")]
    OutOfRange,
}

/// UTC zaman damgası; serde ve `Display` biçimi RFC 3339.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(OffsetDateTime);

const NANOS_PER_MILLI: i128 = 1_000_000;
const NANOS_PER_SEC: u128 = 1_000_000_000;

impl Timestamp {
    /// UTC'ye çevirir ve yıl 0–9999 aralığını zorlar (RFC 3339'un yazabildiği aralık).
    fn validated(value: OffsetDateTime) -> Result<Self, TimeError> {
        let utc = value
            .checked_to_offset(UtcOffset::UTC)
            .ok_or(TimeError::OutOfRange)?;
        if (0..=9999).contains(&utc.year()) {
            Ok(Self(utc))
        } else {
            Err(TimeError::OutOfRange)
        }
    }

    /// Unix çağından bu yana milisaniyeden oluşturur.
    ///
    /// # Errors
    ///
    /// Değer yıl 0–9999 aralığının dışındaysa [`TimeError::OutOfRange`] döner.
    pub fn from_unix_millis(millis: i64) -> Result<Self, TimeError> {
        let nanos = i128::from(millis)
            .checked_mul(NANOS_PER_MILLI)
            .ok_or(TimeError::OutOfRange)?;
        let value =
            OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|_| TimeError::OutOfRange)?;
        Self::validated(value)
    }

    /// Unix çağından bu yana milisaniye (aşağı yuvarlanır).
    #[must_use]
    pub fn unix_millis(self) -> i64 {
        // Yapıcılar yıl 0–9999 aralığını zorladığı için sonuç her zaman i64'e sığar.
        self.0
            .unix_timestamp_nanos()
            .checked_div_euclid(NANOS_PER_MILLI)
            .and_then(|millis| i64::try_from(millis).ok())
            .unwrap_or(i64::MAX)
    }

    /// RFC 3339 metninden ayrıştırır; sonuç UTC'ye çevrilir.
    ///
    /// # Errors
    ///
    /// Metin RFC 3339 değilse [`TimeError::Parse`], aralık dışındaysa
    /// [`TimeError::OutOfRange`] döner.
    pub fn parse_rfc3339(text: &str) -> Result<Self, TimeError> {
        let value =
            OffsetDateTime::parse(text, &Rfc3339).map_err(|_| TimeError::Parse(text.to_owned()))?;
        Self::validated(value)
    }

    /// RFC 3339 metni.
    ///
    /// # Errors
    ///
    /// Yalnızca desteklenen aralık dışında oluşabilir; yapıcılar bunu engeller.
    pub fn to_rfc3339(self) -> Result<String, TimeError> {
        self.0.format(&Rfc3339).map_err(|_| TimeError::OutOfRange)
    }

    /// `duration` sonrası; aralık dışına taşarsa `None`.
    #[must_use]
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        let duration = time::Duration::try_from(duration).ok()?;
        let value = self.0.checked_add(duration)?;
        Self::validated(value).ok()
    }

    /// `earlier`'dan bu yana geçen süre; `earlier` daha sonraysa sıfır.
    #[must_use]
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        let diff = self
            .0
            .unix_timestamp_nanos()
            .checked_sub(earlier.0.unix_timestamp_nanos())
            .and_then(|nanos| u128::try_from(nanos).ok())
            .unwrap_or(0);
        let secs = diff.checked_div(NANOS_PER_SEC).unwrap_or(0);
        let subsec = diff.checked_rem(NANOS_PER_SEC).unwrap_or(0);
        Duration::new(
            u64::try_from(secs).unwrap_or(u64::MAX),
            u32::try_from(subsec).unwrap_or(0),
        )
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self.to_rfc3339().map_err(|_| fmt::Error)?;
        f.write_str(&text)
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let text = self.to_rfc3339().map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&text)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse_rfc3339(&text).map_err(serde::de::Error::custom)
    }
}
