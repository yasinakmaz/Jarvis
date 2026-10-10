-- Şema v1 (Tasarım 0007). Zaman damgaları Unix milisaniyesidir (INTEGER): sıralanabilir ve
-- biçimden bağımsız. Yalnızca ileri yönlü; bu dosya yayımlandıktan sonra değiştirilmez.
CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE messages (
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL,
  run_id TEXT NOT NULL,
  body_json TEXT NOT NULL,
  PRIMARY KEY (session_id, ordinal)
);

CREATE TABLE runs (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  trace_id TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  outcome TEXT,
  agent_version TEXT NOT NULL
);

CREATE TABLE audit_events (
  seq INTEGER PRIMARY KEY,
  at INTEGER NOT NULL,
  run_id TEXT,
  trace_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL
);

CREATE TRIGGER audit_no_update BEFORE UPDATE ON audit_events
  BEGIN SELECT RAISE(ABORT, 'audit_events yalnızca eklemelidir'); END;

CREATE TRIGGER audit_no_delete BEFORE DELETE ON audit_events
  BEGIN SELECT RAISE(ABORT, 'audit_events yalnızca eklemelidir'); END;
