//! Aktör iş parçacığında çalışan sorgular. Her biri gerçek `SQLite`'a karşı L5a testleriyle
//! sınanır (ADR 0024: derleme zamanı sorgu denetimi yok).

use jarvis_types::{EventSeq, Message, RunId, SessionId, Timestamp};
use rusqlite::{Connection, Row, params};

use crate::{AuditRow, Page, RunOutcome, RunRecord, SessionSummary, StoreError};

const SUMMARY: &str = "SELECT s.id, s.created_at, s.updated_at,
       (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id)
  FROM sessions s";

fn corrupt(what: &str, detail: impl std::fmt::Display) -> StoreError {
    StoreError::Corrupt(format!("{what}: {detail}"))
}

fn timestamp(millis: i64) -> Result<Timestamp, StoreError> {
    Timestamp::from_unix_millis(millis).map_err(|e| corrupt("zaman damgası", e))
}

/// `SUMMARY` satırı. Dönüşüm hataları `rusqlite` hatasına değil `Corrupt`'a gider.
fn summary(row: &Row<'_>) -> rusqlite::Result<Result<SessionSummary, StoreError>> {
    let (id, created, updated, count): (String, i64, i64, i64) =
        (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
    Ok((|| {
        Ok(SessionSummary {
            id: SessionId::parse(&id).map_err(|e| corrupt("oturum kimliği", e))?,
            created_at: timestamp(created)?,
            updated_at: timestamp(updated)?,
            message_count: u64::try_from(count).map_err(|e| corrupt("mesaj sayısı", e))?,
        })
    })())
}

pub fn create_session(conn: &Connection, id: SessionId, now: Timestamp) -> Result<(), StoreError> {
    let now = now.unix_millis();
    conn.execute(
        "INSERT INTO sessions VALUES (?1, ?2, ?2)",
        params![id.to_string(), now],
    )?;
    Ok(())
}

pub fn session(conn: &Connection, id: SessionId) -> Result<Option<SessionSummary>, StoreError> {
    let sql = format!("{SUMMARY} WHERE s.id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map([id.to_string()], summary)?;
    rows.next().transpose()?.transpose()
}

pub fn list_sessions(conn: &Connection, page: Page) -> Result<Vec<SessionSummary>, StoreError> {
    let sql = format!("{SUMMARY} ORDER BY s.updated_at DESC, s.id DESC LIMIT ?1 OFFSET ?2");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([page.limit(), page.offset()], summary)?;
    rows.map(|row| row?).collect()
}

pub fn delete_session(conn: &Connection, id: SessionId) -> Result<bool, StoreError> {
    Ok(conn.execute("DELETE FROM sessions WHERE id = ?1", [id.to_string()])? == 1)
}

fn exists(conn: &Connection, id: SessionId) -> Result<bool, StoreError> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sessions WHERE id = ?1)",
        [id.to_string()],
        |row| row.get(0),
    )?)
}

pub fn messages(conn: &Connection, id: SessionId) -> Result<Vec<Message>, StoreError> {
    if !exists(conn, id)? {
        return Err(StoreError::UnknownSession(id));
    }
    let mut stmt =
        conn.prepare("SELECT body_json FROM messages WHERE session_id = ?1 ORDER BY ordinal")?;
    let bodies = stmt.query_map([id.to_string()], |row| row.get::<_, String>(0))?;
    bodies
        .map(|body| serde_json::from_str(&body?).map_err(|e| corrupt("mesaj gövdesi", e)))
        .collect()
}

pub fn append_messages(
    conn: &mut Connection,
    id: SessionId,
    run: RunId,
    messages: &[Message],
    now: Timestamp,
) -> Result<(), StoreError> {
    let tx = conn.transaction()?;
    let touched = tx.execute(
        "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
        params![id.to_string(), now.unix_millis()],
    )?;
    if touched == 0 {
        return Err(StoreError::UnknownSession(id));
    }
    let mut ordinal: i64 = tx.query_row(
        "SELECT COALESCE(MAX(ordinal) + 1, 0) FROM messages WHERE session_id = ?1",
        [id.to_string()],
        |row| row.get(0),
    )?;
    {
        let mut insert = tx.prepare("INSERT INTO messages VALUES (?1, ?2, ?3, ?4)")?;
        for message in messages {
            let body = serde_json::to_string(message).map_err(|e| corrupt("mesaj", e))?;
            insert.execute(params![id.to_string(), ordinal, run.to_string(), body])?;
            ordinal = ordinal
                .checked_add(1)
                .ok_or(StoreError::OutOfRange("message ordinal"))?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn record_run(conn: &Connection, run: &RunRecord) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO runs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (id) DO UPDATE SET finished_at = excluded.finished_at,
                                        outcome = excluded.outcome",
        params![
            run.id.to_string(),
            run.session_id.map(|s| s.to_string()),
            run.trace_id.to_string(),
            run.started_at.unix_millis(),
            run.finished_at.map(Timestamp::unix_millis),
            run.outcome.map(RunOutcome::as_str),
            run.agent_version,
        ],
    )?;
    Ok(())
}

pub fn append_audit(conn: &Connection, row: &AuditRow) -> Result<(), StoreError> {
    let seq = i64::try_from(row.seq.get()).map_err(|_| StoreError::OutOfRange("audit seq"))?;
    conn.execute(
        "INSERT INTO audit_events VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            seq,
            row.at.unix_millis(),
            row.run_id.map(|r| r.to_string()),
            row.trace_id.to_string(),
            row.kind,
            row.payload_json,
        ],
    )?;
    Ok(())
}

pub fn next_audit_seq(conn: &Connection) -> Result<EventSeq, StoreError> {
    let last: Option<i64> =
        conn.query_row("SELECT MAX(seq) FROM audit_events", [], |row| row.get(0))?;
    let Some(last) = last else {
        return Ok(EventSeq::FIRST);
    };
    let last = u64::try_from(last).map_err(|e| corrupt("denetim sıra numarası", e))?;
    EventSeq::new(last)
        .next()
        .ok_or(StoreError::OutOfRange("audit seq"))
}
