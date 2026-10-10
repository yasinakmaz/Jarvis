//! Araç tanımı, risk seviyesi ve doğrulama sonucu (Mimari §6).

use serde::{Deserialize, Serialize};

use crate::message::ToolName;

/// Risk seviyesi; sıralama `Read < Act < Sensitive < Destructive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Okuma: doğrudan çalışır, kaydedilir.
    Read,
    /// Eylem: politikaya göre.
    Act,
    /// Hassas: onay gerekir.
    Sensitive,
    /// Yıkıcı: ayrıntılı özetle onay gerekir.
    Destructive,
}

/// Modele ve `/tools` ucuna yayınlanan araç tanımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Araç adı.
    pub name: ToolName,
    /// Modelin okuyacağı açıklama (İngilizce).
    pub description: String,
    /// Argümanların JSON şeması.
    pub parameters: serde_json::Value,
    /// Varsayılan risk; girdiye bağlı risk aracın kendisinde hesaplanır.
    pub risk: RiskLevel,
}

/// Bağımsız gözlemcinin sonucu (Mimari §1 kural 7: `rc=0` kanıt değildir).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Verification {
    /// Etki bağımsız olarak gözlendi.
    Verified,
    /// Etki gözlenemedi (ör. iptal, zaman aşımı).
    Unverifiable {
        /// Neden.
        reason: String,
    },
    /// Gözlem beklenenle çelişiyor.
    Failed {
        /// Neden.
        reason: String,
    },
}
