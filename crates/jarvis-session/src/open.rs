//! Dosyayı açma: izin denetimi, şema sürümü, yedek ve ileri yönlü migrasyon (Tasarım 0007).
//!
//! Sıra önemlidir: şema sürümü okunup denetlenene ve gerekiyorsa yedek alınana kadar dosyaya
//! yazılmaz (daha yeni şemalı bir dosya hiç değişmez).

use std::fs::{self, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

use crate::{SCHEMA_VERSION, StoreError};

/// `(sürüm, SQL)`; sürümler 1'den başlar ve ardışıktır. Yayımlanmış bir dosya değiştirilmez.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/0001_init.sql"))];

/// Sahibi dışında hiçbir izin bitine izin verilmez.
const FOREIGN_BITS: u32 = 0o077;

/// Dosyayı hazırlar ve kullanıma hazır bağlantıyı döndürür.
pub fn prepare(path: &Path) -> Result<Connection, StoreError> {
    let created = ensure_private_file(path)?;
    let mut conn = Connection::open(path)?;
    let version = read_version(&conn)?;
    if version > SCHEMA_VERSION {
        return Err(StoreError::NewerSchema {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    if version < SCHEMA_VERSION && !created {
        backup(&conn, path, version)?;
    }
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    migrate(&mut conn, version)?;
    Ok(conn)
}

/// Dosya yoksa üst dizinleriyle birlikte `0600` oluşturur (`true`); varsa izinlerini denetler.
fn ensure_private_file(path: &Path) -> Result<bool, StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.try_exists()? {
        // `create_new`: arada başka biri oluşturduysa açık hata (üzerine yazılmaz).
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        return Ok(true);
    }
    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    if mode & FOREIGN_BITS == 0 {
        Ok(false)
    } else {
        Err(StoreError::Permissions {
            path: path.to_owned(),
            mode,
        })
    }
}

/// `schema_meta` yoksa 0; varsa `version` satırı (negatif değil) olmalıdır.
fn read_version(conn: &Connection) -> Result<i64, StoreError> {
    let has_meta: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_meta')",
        [],
        |row| row.get(0),
    )?;
    if !has_meta {
        return Ok(0);
    }
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let value = value.ok_or_else(|| corrupt("schema_meta'da 'version' satırı yok"))?;
    match value.parse::<i64>() {
        Ok(version) if version >= 0 => Ok(version),
        _ => Err(corrupt(&format!("schema_meta.version geçersiz: '{value}'"))),
    }
}

fn corrupt(detail: &str) -> StoreError {
    StoreError::Corrupt(detail.to_owned())
}

/// `<dosya>.bak-<sürüm>`: önce `0600` boş dosya, sonra `VACUUM INTO`. Var olan yedeğin
/// üzerine yazılmaz.
fn backup(conn: &Connection, path: &Path, version: i64) -> Result<(), StoreError> {
    let target = backup_path(path, version);
    let fail = |detail: String| StoreError::Backup {
        path: target.clone(),
        detail,
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&target)
        .map_err(|e| fail(e.to_string()))?;
    let name = target
        .to_str()
        .ok_or_else(|| fail("yol UTF-8 değil".to_owned()))?;
    if let Err(error) = conn.execute("VACUUM INTO ?1", [name]) {
        // Boş yer tutucu kalırsa sonraki açılış yanlışlıkla "yedek var" der; hata yine döner.
        let cleanup = fs::remove_file(&target)
            .err()
            .map(|e| format!("; temizlenemedi: {e}"));
        return Err(fail(format!("{error}{}", cleanup.unwrap_or_default())));
    }
    Ok(())
}

fn backup_path(path: &Path, version: i64) -> PathBuf {
    let mut name = path.file_name().map(ToOwned::to_owned).unwrap_or_default();
    name.push(format!(".bak-{version}"));
    path.with_file_name(name)
}

/// Eksik her migrasyonu kendi transaction'ında uygular ve sürümü aynı transaction'da yazar.
fn migrate(conn: &mut Connection, from: i64) -> Result<(), StoreError> {
    for (version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > from) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_meta (key, value) VALUES ('version', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [version.to_string()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_contiguous_and_end_at_the_schema_version() {
        let versions: Vec<i64> = MIGRATIONS.iter().map(|(v, _)| *v).collect();
        let expected: Vec<i64> = (1..=SCHEMA_VERSION).collect();
        assert_eq!(versions, expected);
    }

    #[test]
    fn backup_name_appends_the_old_version() {
        let path = Path::new("/veri/jarvis.db");
        assert_eq!(backup_path(path, 3), PathBuf::from("/veri/jarvis.db.bak-3"));
    }
}
