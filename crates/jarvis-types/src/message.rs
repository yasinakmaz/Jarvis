//! Ajanın alan modelindeki mesajlar (Tasarım 0003, ADR 0029).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::ToolCallId;

/// Araç adının en fazla uzunluğu.
pub const MAX_TOOL_NAME_LEN: usize = 64;

/// Mesajın konuşmadaki rolü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Sistem istemi (İngilizce, ADR 0014).
    System,
    /// Kullanıcı girdisi.
    User,
    /// Modelin yanıtı.
    Assistant,
    /// Bir araç çağrısının sonucu.
    Tool,
}

/// Mesaj içeriği parçası.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Content {
    /// Düz metin.
    Text(String),
    /// PNG görüntü baytları (M3/M4).
    ImagePng(Vec<u8>),
}

/// İçeriğin güven düzeyi (Mimari §1 kural 5: araç çıktısı talimat değildir).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "trust", rename_all = "snake_case")]
pub enum Trust {
    /// Kullanıcıdan, sistemden veya modelin kendisinden gelen içerik.
    Trusted,
    /// Dış kaynaklı içerik (araç çıktısı, dosya, web sayfası); talimat olarak yorumlanmaz.
    Untrusted {
        /// İçeriğin kaynağı, ör. araç adı.
        source: String,
    },
}

/// Doğrulanmış araç adı: `^[a-z][a-z0-9_]{0,63}$`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ToolName(String);

/// Araç adı ayrıştırma hatası.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "geçersiz araç adı `{0}`: küçük harfle başlamalı, yalnızca a-z, 0-9 ve `_` içermeli, \
     en fazla {MAX_TOOL_NAME_LEN} karakter olmalı"
)]
pub struct NameError(pub String);

impl ToolName {
    /// Adı doğrulayarak oluşturur.
    ///
    /// # Errors
    ///
    /// Ad desene uymuyorsa [`NameError`] döner.
    pub fn parse(name: &str) -> Result<Self, NameError> {
        let mut chars = name.chars();
        let valid = name.len() <= MAX_TOOL_NAME_LEN
            && chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if valid {
            Ok(Self(name.to_owned()))
        } else {
            Err(NameError(name.to_owned()))
        }
    }

    /// Adın metni.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ToolName {
    type Error = NameError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::parse(&name)
    }
}

impl From<ToolName> for String {
    fn from(name: ToolName) -> Self {
        name.0
    }
}

/// Modelin istediği bir araç çağrısı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Sağlayıcının verdiği kimlik; sonuç mesajı bunu taşır.
    pub id: ToolCallId,
    /// Çağrılan araç.
    pub name: ToolName,
    /// JSON argümanlar (aracın şemasına göre ayrıca doğrulanır).
    pub arguments: serde_json::Value,
}

/// Konuşmadaki bir mesaj.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Rol.
    pub role: Role,
    /// İçerik parçaları.
    pub content: Vec<Content>,
    /// Yalnızca `Assistant` mesajlarında: istenen araç çağrıları.
    pub tool_calls: Vec<ToolCall>,
    /// Yalnızca `Tool` mesajlarında: yanıtlanan çağrının kimliği.
    pub tool_call_id: Option<ToolCallId>,
    /// İçeriğin güven düzeyi.
    pub trust: Trust,
}

impl Message {
    /// Tek metin parçalı, güvenilir bir mesaj.
    #[must_use]
    pub fn text(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![Content::Text(text.into())],
            tool_calls: Vec::new(),
            tool_call_id: None,
            trust: Trust::Trusted,
        }
    }

    /// Metin parçalarının birleşimi (görüntüler atlanır).
    #[must_use]
    pub fn joined_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| match part {
                Content::Text(text) => Some(text.as_str()),
                Content::ImagePng(_) => None,
            })
            .collect()
    }
}
