//! Açma, izinler, yedek ve migrasyon (Tasarım 0007 test planı, L5a). Oracle: gerçek dosya
//! sistemi ve aynı dosyaya açılan bağımsız bir `rusqlite` bağlantısı.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use jarvis_session::{AuditRow, SCHEMA_VERSION, Store, StoreError};
use jarvis_types::{BoxFuture, Clock, EventSeq, Timestamp, TraceId};
use rusqlite::Connection;

struct ZeroClock;

impl Clock for ZeroClock {
    #[expect(clippy::unwrap_used, reason = "sabit; geçersizse test düşmeli")]
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(0).unwrap()
    }

    fn sleep(&self, _duration: Duration) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

fn open(path: &Path) -> Result<Store, StoreError> {
    Store::open(path, Arc::new(ZeroClock))
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or_default()
}

fn backups(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|name| name.contains(".bak-"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn version(path: &Path) -> rusqlite::Result<String> {
    Connection::open(path)?.query_row(
        "SELECT value FROM schema_meta WHERE key = 'version'",
        [],
        |row| row.get(0),
    )
}

fn tables(path: &Path) -> rusqlite::Result<Vec<String>> {
    let conn = Connection::open(path)?;
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY 1")?;
    stmt.query_map([], |row| row.get(0))?.collect()
}

/// Kendi bağlantısıyla, `0600` izinli bir dosya hazırlar.
fn prepare(path: &Path, sql: &str) -> Result<(), Box<dyn std::error::Error>> {
    Connection::open(path)?.execute_batch(sql)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn db(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("jarvis.db")
}

#[test]
fn fresh_path_gets_v1_schema_private_permissions_and_no_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("eksik/alt/jarvis.db");
    drop(open(&path).unwrap());
    assert_eq!(version(&path).unwrap(), SCHEMA_VERSION.to_string());
    assert_eq!(
        tables(&path).unwrap(),
        [
            "audit_events",
            "messages",
            "runs",
            "schema_meta",
            "sessions"
        ]
    );
    assert_eq!(mode(&path), 0o600);
    assert!(backups(path.parent().unwrap()).is_empty());
}

#[test]
fn reopening_a_current_file_neither_migrates_nor_backs_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    drop(open(&path).unwrap());
    drop(open(&path).unwrap());
    assert_eq!(version(&path).unwrap(), "1");
    assert!(backups(dir.path()).is_empty());
}

#[test]
fn legacy_v0_file_is_backed_up_then_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    prepare(
        &path,
        "CREATE TABLE legacy (x INTEGER); INSERT INTO legacy VALUES (1), (2);",
    )
    .unwrap();
    drop(open(&path).unwrap());

    assert_eq!(backups(dir.path()), ["jarvis.db.bak-0"]);
    let backup = dir.path().join("jarvis.db.bak-0");
    assert_eq!(mode(&backup), 0o600);
    assert_eq!(tables(&backup).unwrap(), ["legacy"], "yedek eski şemadır");
    let rows: Vec<i64> = Connection::open(&backup)
        .unwrap()
        .prepare("SELECT x FROM legacy ORDER BY x")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows, [1, 2]);
    assert_eq!(version(&path).unwrap(), "1");
    assert!(
        tables(&path).unwrap().contains(&"legacy".to_owned()),
        "veri korunur"
    );
}

#[test]
fn newer_schema_is_refused_and_the_file_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    prepare(
        &path,
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO schema_meta VALUES ('version', '99');",
    )
    .unwrap();
    let before = fs::read(&path).unwrap();
    let error = open(&path).unwrap_err();
    assert_eq!(
        error,
        StoreError::NewerSchema {
            found: 99,
            supported: SCHEMA_VERSION
        }
    );
    assert_eq!(
        error.to_string(),
        "veritabanı şeması 99, bu sürüm en çok 1 destekliyor; Jarvis'i güncelleyin"
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(backups(dir.path()).is_empty());
}

#[test]
fn unreadable_schema_version_is_corruption() {
    for sql in [
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO schema_meta VALUES ('version', 'bir');",
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO schema_meta VALUES ('version', '-1');",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = db(&dir);
        prepare(&path, sql).unwrap();
        assert!(matches!(open(&path), Err(StoreError::Corrupt(_))), "{sql}");
        assert!(backups(dir.path()).is_empty());
    }
}

#[test]
fn group_or_world_readable_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    prepare(&path, "CREATE TABLE legacy (x INTEGER);").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let error = open(&path).unwrap_err();
    assert_eq!(
        error,
        StoreError::Permissions {
            path: path.clone(),
            mode: 0o644
        }
    );
    assert!(
        error
            .to_string()
            .ends_with("izinleri 644; 0600 olmalı (chmod 600)")
    );
    assert!(backups(dir.path()).is_empty());
    assert_eq!(tables(&path).unwrap(), ["legacy"]);
}

#[test]
fn existing_backup_blocks_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    prepare(&path, "CREATE TABLE legacy (x INTEGER);").unwrap();
    let backup = dir.path().join("jarvis.db.bak-0");
    fs::write(&backup, b"eski yedek").unwrap();
    let error = open(&path).unwrap_err();
    assert!(
        matches!(&error, StoreError::Backup { path, .. } if *path == backup),
        "{error}"
    );
    assert_eq!(
        fs::read(&backup).unwrap(),
        b"eski yedek",
        "eski yedeğin üzerine yazılmaz"
    );
    assert_eq!(tables(&path).unwrap(), ["legacy"], "migrasyon yapılmadı");
}

#[test]
fn file_that_is_not_sqlite_is_a_database_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    fs::write(&path, vec![b'x'; 4096]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(open(&path), Err(StoreError::Database(_))));
}

#[tokio::test]
async fn audit_rows_cannot_be_updated_or_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let path = db(&dir);
    let store = open(&path).unwrap();
    let row = AuditRow {
        seq: EventSeq::new(3),
        at: Timestamp::from_unix_millis(1_500).unwrap(),
        run_id: None,
        trace_id: TraceId::new(),
        kind: "halt".to_owned(),
        payload_json: "{}".to_owned(),
    };
    store.append_audit(row.clone()).await.unwrap();
    let conn = Connection::open(&path).unwrap();
    for sql in [
        "UPDATE audit_events SET kind = 'x'",
        "DELETE FROM audit_events",
    ] {
        let error = conn.execute(sql, []).unwrap_err();
        assert!(
            error.to_string().contains("yalnızca eklemelidir"),
            "{sql}: {error}"
        );
    }
    let stored: (i64, i64, Option<String>, String, String, String) = conn
        .query_row("SELECT * FROM audit_events", [], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap();
    assert_eq!(
        stored,
        (
            3,
            1_500,
            None,
            row.trace_id.to_string(),
            "halt".to_owned(),
            "{}".to_owned()
        )
    );
}
