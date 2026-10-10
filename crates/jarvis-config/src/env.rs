//! Ortam erişimi enjekte edilir; testler süreç ortamına dokunmaz.

use std::collections::BTreeMap;
use std::ffi::OsString;

/// Ortam değişkeni okuyucu.
pub trait EnvLookup {
    /// Değişkenin değeri; tanımlı değilse `None`.
    fn get(&self, name: &str) -> Option<OsString>;
}

/// Sürecin gerçek ortamı.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEnv;

impl EnvLookup for SystemEnv {
    fn get(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }
}

/// Testler ve gömülü kullanım için sabit ortam.
impl EnvLookup for BTreeMap<String, String> {
    fn get(&self, name: &str) -> Option<OsString> {
        Self::get(self, name).map(OsString::from)
    }
}
