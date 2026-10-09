//! Kimlik tipleri (Tasarım 0003, ADR 0026).
//!
//! Koşu, oturum ve iz kimlikleri UUID v7'dir: zamana göre sıralıdır ve kanonik metin olarak
//! serileştirilir. Araç çağrısı kimliği sağlayıcıdan gelir ve opak bir dizgedir.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Araç çağrısı kimliğinin en fazla uzunluğu (karakter).
pub const MAX_TOOL_CALL_ID_LEN: usize = 256;

/// Kimlik ayrıştırma hataları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// Metin UUID değil.
    #[error("geçersiz UUID: `{0}`")]
    InvalidUuid(String),
    /// UUID geçerli ama sürüm 7 değil.
    #[error("UUID sürüm 7 değil: `{0}`")]
    NotV7(String),
    /// Araç çağrısı kimliği boş ya da çok uzun.
    #[error("araç çağrısı kimliği {len} karakter; 1 ile {MAX_TOOL_CALL_ID_LEN} arasında olmalı")]
    InvalidToolCallId {
        /// Gelen kimliğin karakter sayısı.
        len: usize,
    },
}

fn parse_v7(text: &str) -> Result<Uuid, IdError> {
    let uuid = Uuid::try_parse(text).map_err(|_| IdError::InvalidUuid(text.to_owned()))?;
    if uuid.get_version_num() == 7 {
        Ok(uuid)
    } else {
        Err(IdError::NotV7(text.to_owned()))
    }
}

macro_rules! uuid_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Uuid);

        impl $name {
            /// Yeni, zamana göre sıralı (UUID v7) bir kimlik üretir.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Kanonik metinden ayrıştırır.
            ///
            /// # Errors
            ///
            /// Metin UUID değilse ya da UUID sürüm 7 değilse [`IdError`] döner.
            pub fn parse(text: &str) -> Result<Self, IdError> {
                parse_v7(text).map(Self)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::parse(text)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(text: String) -> Result<Self, Self::Error> {
                Self::parse(&text)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.to_string()
            }
        }
    };
}

uuid_id!(RunId, "Bir ajan koşusunun kimliği.");
uuid_id!(
    SessionId,
    "Kalıcı bir oturumun kimliği (`x-jarvis-session`)."
);
uuid_id!(
    TraceId,
    "Günlük, olay, denetim ve eval kayıtlarını bağlayan iz kimliği."
);

/// Sağlayıcının verdiği araç çağrısı kimliği (opak dizge).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ToolCallId(String);

impl ToolCallId {
    /// Kimliği doğrulayarak oluşturur.
    ///
    /// # Errors
    ///
    /// Kimlik boşsa ya da [`MAX_TOOL_CALL_ID_LEN`] karakterden uzunsa
    /// [`IdError::InvalidToolCallId`] döner.
    pub fn new(id: String) -> Result<Self, IdError> {
        let len = id.chars().count();
        if (1..=MAX_TOOL_CALL_ID_LEN).contains(&len) {
            Ok(Self(id))
        } else {
            Err(IdError::InvalidToolCallId { len })
        }
    }

    /// Kimliğin metni.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ToolCallId {
    type Error = IdError;

    fn try_from(id: String) -> Result<Self, Self::Error> {
        Self::new(id)
    }
}

impl From<ToolCallId> for String {
    fn from(id: ToolCallId) -> Self {
        id.0
    }
}

/// Olay veriyolu içindeki monoton sıra numarası.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventSeq(u64);

impl EventSeq {
    /// İlk sıra numarası.
    pub const FIRST: Self = Self(0);

    /// Ham değerden oluşturur.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Ham değer.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Bir sonraki sıra numarası; taşmada `None`.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl fmt::Display for EventSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
