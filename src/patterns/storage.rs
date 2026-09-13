//! SQLite runs on a dedicated worker. Personal choices and rebuildable command
//! observations have separate files; concurrent viewers use SQLite transactions.
use super::{Aggregate, Learning, Occurrence};
use rusqlite::{Connection, params};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub enum Request {
    Index(Vec<Occurrence>),
    Set(String, Learning),
    Refresh,
}
pub enum Reply {
    Loaded(Vec<Aggregate>, BTreeMap<String, Learning>),
    Indexed(Vec<Aggregate>),
    Saved(Result<(), String>),
    Error(String),
}
pub struct Store {
    index: Connection,
    learning: Connection,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self, String> {
        if !root.exists() {
            std::fs::create_dir_all(root)
                .map_err(|_| "Cannot create Linger data directory.".to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
                    .map_err(|_| "Cannot protect Linger data directory.".to_string())?;
            }
        }
        let index = Connection::open(root.join("patterns.sqlite3"))
            .map_err(|_| "Cannot open command cache.".to_string())?;
        let learning = Connection::open(root.join("learning.sqlite3"))
            .map_err(|_| "Cannot open learning state.".to_string())?;
        Self::initialize(index, learning)
    }
    fn initialize(index: Connection, learning: Connection) -> Result<Self, String> {
        for connection in [&index, &learning] {
            connection
                .busy_timeout(std::time::Duration::from_secs(2))
                .map_err(db_error)?;
            let version: i64 = connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(db_error)?;
            if version > 1 {
                return Err(
                    "A newer Linger database version is present; this build will not modify it."
                        .into(),
                );
            }
            connection
                .execute_batch("PRAGMA journal_mode=WAL;")
                .map_err(db_error)?;
        }
        index.execute_batch("CREATE TABLE IF NOT EXISTS occurrences(session TEXT NOT NULL, agent TEXT NOT NULL, call TEXT NOT NULL, ordering TEXT NOT NULL, pattern TEXT NOT NULL, label TEXT NOT NULL, command TEXT NOT NULL, tool TEXT NOT NULL, PRIMARY KEY(session,agent,call)); CREATE INDEX IF NOT EXISTS occurrence_pattern ON occurrences(pattern,session); PRAGMA user_version=1;").map_err(db_error)?;
        learning.execute_batch("CREATE TABLE IF NOT EXISTS learning(pattern TEXT PRIMARY KEY, state TEXT NOT NULL); PRAGMA user_version=1;").map_err(db_error)?;
        Ok(Self { index, learning })
    }
    pub fn index(&mut self, rows: &[Occurrence]) -> Result<(), String> {
        let tx = self.index.transaction().map_err(db_error)?;
        {
            let mut insert=tx.prepare_cached("INSERT INTO occurrences(session,agent,call,ordering,pattern,label,command,tool) VALUES (?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(session,agent,call) DO UPDATE SET ordering=excluded.ordering,pattern=excluded.pattern,label=excluded.label,command=excluded.command,tool=excluded.tool WHERE excluded.ordering < occurrences.ordering OR (excluded.ordering=occurrences.ordering AND excluded.pattern < occurrences.pattern)").map_err(db_error)?;
            for o in rows {
                insert
                    .execute(params![
                        o.session,
                        o.agent,
                        o.call,
                        o.order,
                        o.pattern.key,
                        o.pattern.label,
                        o.pattern.command,
                        o.pattern.tool
                    ])
                    .map_err(db_error)?;
            }
        }
        tx.commit().map_err(db_error)
    }
    pub fn aggregates(&self) -> Result<Vec<Aggregate>, String> {
        let mut grouped = BTreeMap::<String, Aggregate>::new();
        let mut query=self.index.prepare("SELECT pattern,MIN(label),MIN(command),MIN(tool),session,COUNT(*) FROM occurrences GROUP BY pattern,session ORDER BY pattern,session").map_err(db_error)?;
        let values = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)? as usize,
                ))
            })
            .map_err(db_error)?;
        for v in values {
            let (key, label, example, tool, session, count) = v.map_err(db_error)?;
            let pattern = super::Pattern {
                key,
                label,
                command: example.clone(),
                tool: tool.clone(),
            };
            for p in super::projections(&pattern) {
                *grouped
                    .entry(p.key.clone())
                    .or_insert(Aggregate {
                        key: p.key,
                        label: p.label,
                        example: example.clone(),
                        tool: tool.clone(),
                        sessions: Default::default(),
                        level: p.level,
                        inline: p.inline,
                    })
                    .sessions
                    .entry(session.clone())
                    .or_default() += count;
            }
        }
        Ok(grouped.into_values().collect())
    }
    pub fn set(&self, key: &str, state: Learning) -> Result<(), String> {
        self.learning.execute("INSERT INTO learning(pattern,state) VALUES (?1,?2) ON CONFLICT(pattern) DO UPDATE SET state=excluded.state",params![key,serde_json::to_string(&state).unwrap()]).map_err(db_error)?;
        Ok(())
    }
    pub fn states(&self) -> Result<BTreeMap<String, Learning>, String> {
        let mut result = BTreeMap::new();
        let mut query = self
            .learning
            .prepare("SELECT pattern,state FROM learning")
            .map_err(db_error)?;
        let rows = query
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(db_error)?;
        for row in rows {
            let (key, value) = row.map_err(db_error)?;
            let state = serde_json::from_str(&value)
                .map_err(|_| "Unrecognised learning state; existing data retained.".to_string())?;
            result.insert(key, state);
        }
        Ok(result)
    }
}
fn db_error(_: rusqlite::Error) -> String {
    "Linger database operation failed; recorded data has not been reset. Check disk space and file access.".into()
}
fn directory() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("LINGER_DATA_DIR").filter(|p| !p.is_empty()) {
        return Ok(root.into());
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    base.map(|b| b.join("linger"))
        .ok_or_else(|| "Set LINGER_DATA_DIR to save the library on this machine.".into())
}

pub fn worker() -> (
    tokio::sync::mpsc::UnboundedSender<Request>,
    tokio::sync::mpsc::UnboundedReceiver<Reply>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (reply_tx, reply_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::task::spawn_blocking(move || {
        let mut store = match directory().and_then(|root| Store::open(&root)) {
            Ok(s) => s,
            Err(e) => {
                let _ = reply_tx.send(Reply::Error(e));
                return;
            }
        };
        match store
            .aggregates()
            .and_then(|a| store.states().map(|s| (a, s)))
        {
            Ok((a, s)) => {
                let _ = reply_tx.send(Reply::Loaded(a, s));
            }
            Err(e) => {
                let _ = reply_tx.send(Reply::Error(e));
                return;
            }
        }
        while let Some(request) = rx.blocking_recv() {
            let reply = match request {
                Request::Index(rows) => {
                    match store.index(&rows).and_then(|()| store.aggregates()) {
                        Ok(a) => Reply::Indexed(a),
                        Err(e) => Reply::Error(e),
                    }
                }
                Request::Set(key, state) => Reply::Saved(store.set(&key, state)),
                Request::Refresh => match store.aggregates() {
                    Ok(a) => Reply::Indexed(a),
                    Err(e) => Reply::Error(e),
                },
            };
            if reply_tx.send(reply).is_err() {
                break;
            }
        }
    });
    (tx, reply_rx, handle)
}
