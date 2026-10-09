//! Çalışma alanı yolları.

use std::path::{Path, PathBuf};

/// Çalışma alanı kökü (xtask'ın bir üst dizini).
pub fn root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

/// xtask'ın geçici çıktılarını yazdığı dizin.
pub fn scratch(root: &Path) -> PathBuf {
    root.join("target").join("xtask")
}

/// Dosya yolunun uzantısı `ext` mi (büyük/küçük harf duyarsız)?
pub fn has_extension(path: &str, ext: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// `path`'i `root`'a göre göreli, `/` ayraçlı bir dizgeye çevirir.
pub fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
