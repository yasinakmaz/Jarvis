//! Sağlayıcı `base_url`'i: `https://` ya da yalnızca loopback için `http://`.
//!
//! Tam bir URL ayrıştırıcısı değil; sağlayıcı istemcisine verilmeden önce güvenlik açısından
//! önemli kuralları denetler: düz metin HTTP yalnızca bu makineye, URL'de kimlik bilgisi yok.

use std::fmt;
use std::net::IpAddr;

/// Doğrulanmış sağlayıcı taban adresi (sondaki `/` korunur).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaseUrl(String);

/// `base_url`'in reddedilme nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BaseUrlError {
    /// Şema `https://` ya da `http://` değil.
    #[error("şema https:// olmalı")]
    Scheme,
    /// `http://` loopback dışı bir sunucuya.
    #[error("düz metin http yalnızca loopback (127.0.0.1, ::1, localhost) için")]
    PlainHttp,
    /// Sunucu adı boş.
    #[error("sunucu adı boş")]
    EmptyHost,
    /// URL'de kullanıcı bilgisi (`kullanıcı:parola@`) var; anahtar dosyaya yazılmaz.
    #[error("URL'de kimlik bilgisi olamaz; anahtar api_key_env ile verilir")]
    Credentials,
    /// Port sayı değil ya da sunucu adından sonra beklenmeyen karakter var.
    #[error("port geçersiz (1-65535 arası sayı olmalı)")]
    Port,
    /// Sorgu (`?`) ya da parça (`#`) var.
    #[error("sorgu (?) veya parça (#) olamaz")]
    QueryOrFragment,
    /// Boşluk ya da kontrol karakteri var.
    #[error("boşluk veya kontrol karakteri olamaz")]
    Whitespace,
}

impl BaseUrl {
    /// Adresi doğrular.
    ///
    /// # Errors
    ///
    /// Kural ihlalinde nedenini döndürür ([`BaseUrlError`]).
    pub fn parse(value: &str) -> Result<Self, BaseUrlError> {
        if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(BaseUrlError::Whitespace);
        }
        let (rest, https) = if let Some(rest) = value.strip_prefix("https://") {
            (rest, true)
        } else if let Some(rest) = value.strip_prefix("http://") {
            (rest, false)
        } else {
            return Err(BaseUrlError::Scheme);
        };
        if rest.contains(['?', '#']) {
            return Err(BaseUrlError::QueryOrFragment);
        }
        let authority = rest.split('/').next().unwrap_or_default();
        if authority.contains('@') {
            return Err(BaseUrlError::Credentials);
        }
        let host = host(authority)?;
        if !https && !is_loopback_host(host) {
            return Err(BaseUrlError::PlainHttp);
        }
        Ok(Self(value.to_owned()))
    }

    /// Adres.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `sunucu[:port]` ya da `[ipv6][:port]` içinden köşeli parantezsiz sunucu adı.
fn host(authority: &str) -> Result<&str, BaseUrlError> {
    let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']').ok_or(BaseUrlError::Port)?;
        let port = match after {
            "" => None,
            _ => Some(after.strip_prefix(':').ok_or(BaseUrlError::Port)?),
        };
        (host, port)
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    if host.is_empty() {
        return Err(BaseUrlError::EmptyHost);
    }
    match port {
        Some(port) if !valid_port(port) => Err(BaseUrlError::Port),
        _ => Ok(host),
    }
}

/// Yalnızca rakam, 1-65535.
fn valid_port(port: &str) -> bool {
    port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u16>().is_ok_and(|p| p != 0)
}

/// `localhost` ya da loopback IP.
fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}
