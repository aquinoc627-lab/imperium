//! IMPERIUM ledger — Phase 10.
//!
//! A SQLite **projection** over the canonical intent records
//! (`.imperium/intents/<id>.json`). The JSON records (with their embedded
//! event logs) remain the source of truth; the fold rebuilds state from
//! them. The ledger is a searchable index and *nothing else*: deleting it
//! and running `rebuild` reproduces it byte-for-semantic-byte from the
//! canonical records. There is no in-memory truth.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The canonical record shape, parsed tolerantly. Field-for-field this is
/// what `imperium-cli` persists; the ledger only reads it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDocument {
    pub status: String,
    #[serde(default)]
    pub ir: serde_json::Value,
    #[serde(default)]
    pub simulation: Option<serde_json::Value>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub token: Option<serde_json::Value>,
    #[serde(default)]
    pub events: Vec<EventDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDocument {
    pub kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Epoch ms (absent on legacy events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
}

impl IntentDocument {
    /// The intent id from the embedded IR.
    pub fn id(&self) -> Option<String> {
        self.ir.get("id").and_then(|v| v.as_str()).map(String::from)
    }

    pub fn name(&self) -> Option<String> {
        self.ir
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    pub fn nl_source(&self) -> Option<String> {
        self.ir
            .get("nl_source")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    pub fn compiled_at(&self) -> Option<String> {
        self.ir
            .get("compiled_at")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// Flattened task capabilities, in task order.
    pub fn capabilities(&self) -> Vec<String> {
        self.ir
            .get("tasks")
            .and_then(|t| t.as_array())
            .map(|tasks| {
                tasks
                    .iter()
                    .filter_map(|t| t.get("capabilities"))
                    .filter_map(|c| c.as_array())
                    .flatten()
                    .filter_map(|c| c.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Slot-aware friction identity (Phase 13): `capability|task|target`.
    /// The description is deliberately excluded — intents that differ only
    /// in content are the friction signal, and the varying field becomes
    /// the form's slot.
    pub fn semantic_key(&self) -> Option<String> {
        let task = self.ir.get("tasks")?.as_array()?.first()?.clone();
        let cap = task
            .get("capabilities")?
            .as_array()?
            .first()?
            .as_str()?
            .to_string();
        let task_name = task.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let target = task
            .get("target_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Some(format!("{cap}|{task_name}|{target}"))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub status: String,
    pub nl_source: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub total: u64,
    pub by_status: BTreeMap<String, u64>,
    pub executed: u64,
    pub failed: u64,
    pub top_capabilities: Vec<(String, u64)>,
    pub top_denials: Vec<(String, u64)>,
}

/// Observed per-capability facts for the Phase 12 world model. A pure view
/// over the ledger's events — recomputable, never stored separately.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CapabilityStatDoc {
    pub samples: u64,
    pub successes: u64,
    #[serde(default)]
    pub durations_ms: Vec<u64>,
}

pub type WorldStatsDoc = BTreeMap<String, CapabilityStatDoc>;

pub struct Ledger {
    conn: Connection,
}

impl Ledger {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("cannot open ledger {}", path.display()))?;
        Self::init_schema(&conn)?;
        Ok(Self { conn })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS intents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                nl_source TEXT NOT NULL,
                risk_score REAL NOT NULL DEFAULT 0,
                requires_approval INTEGER NOT NULL DEFAULT 1,
                output TEXT,
                capabilities TEXT NOT NULL DEFAULT '[]',
                compiled_at TEXT,
                record TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS events (
                intent_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL,
                PRIMARY KEY (intent_id, seq)
            );
            CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);",
        )?;
        Ok(())
    }

    /// Upsert one document's projection (intents row + event rows).
    pub fn sync(&self, doc: &IntentDocument) -> Result<()> {
        let id = doc
            .id()
            .context("document has no ir.id; not a canonical record")?;
        let capabilities =
            serde_json::to_string(&doc.capabilities()).expect("capabilities serialize");
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO intents (id, name, status, nl_source, risk_score, requires_approval,
                                  output, capabilities, compiled_at, record)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                nl_source = excluded.nl_source,
                risk_score = excluded.risk_score,
                requires_approval = excluded.requires_approval,
                output = excluded.output,
                capabilities = excluded.capabilities,
                compiled_at = excluded.compiled_at,
                record = excluded.record",
            params![
                id,
                doc.name().unwrap_or_default(),
                doc.status,
                doc.nl_source().unwrap_or_default(),
                doc.ir
                    .get("risk_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                doc.ir
                    .get("requires_approval")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                doc.output,
                capabilities,
                doc.compiled_at(),
                serde_json::to_string(doc)?,
            ],
        )?;
        tx.execute("DELETE FROM events WHERE intent_id = ?1", params![id])?;
        for (seq, ev) in doc.events.iter().enumerate() {
            tx.execute(
                "INSERT INTO events (intent_id, seq, kind, payload) VALUES (?1, ?2, ?3, ?4)",
                params![id, seq as i64, ev.kind, serde_json::to_string(&ev.payload)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete the projection and rebuild it from every canonical record in
    /// `intents_dir`. Returns the number of documents projected.
    pub fn rebuild(&self, intents_dir: &Path) -> Result<usize> {
        let mut docs: Vec<IntentDocument> = vec![];
        let entries = std::fs::read_dir(intents_dir).context("intents directory is missing")?;
        for entry in entries {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let data = std::fs::read_to_string(entry.path())
                .with_context(|| format!("unreadable record {}", entry.path().display()))?;
            docs.push(
                serde_json::from_str(&data)
                    .with_context(|| format!("corrupt record {}", entry.path().display()))?,
            );
        }
        self.conn.execute("DELETE FROM events", params![])?;
        self.conn.execute("DELETE FROM intents", params![])?;
        for doc in &docs {
            self.sync(doc)?;
        }
        Ok(docs.len())
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        // Literal search: escape LIKE metacharacters so `%` and `_` in a
        // query can never act as wildcards.
        let like = format!(
            "%{}%",
            query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, nl_source FROM intents
             WHERE name LIKE ?1 ESCAPE '\\' OR nl_source LIKE ?1 ESCAPE '\\'
             OR output LIKE ?1 ESCAPE '\\'
             ORDER BY compiled_at, id",
        )?;
        let hits = stmt
            .query_map(params![like], |row| {
                Ok(SearchHit {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    status: row.get(2)?,
                    nl_source: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(hits)
    }

    pub fn stats(&self) -> Result<Stats> {
        let mut stats = Stats::default();
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM intents GROUP BY status")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (status, count) in rows {
            match status.as_str() {
                "executed" => stats.executed = count,
                "failed" => stats.failed = count,
                _ => {}
            }
            stats.by_status.insert(status, count);
            stats.total += count;
        }

        let mut caps: BTreeMap<String, u64> = BTreeMap::new();
        let mut stmt = self.conn.prepare("SELECT capabilities FROM intents")?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for row in rows {
            let caps_list: Vec<String> = serde_json::from_str(&row)?;
            for c in caps_list {
                *caps.entry(c).or_insert(0) += 1;
            }
        }
        stats.top_capabilities = caps.into_iter().collect();
        stats
            .top_capabilities
            .sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let mut denials: BTreeMap<String, u64> = BTreeMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT kind, payload FROM events WHERE kind IN ('TaskFailed', 'PolicyEvaluated')",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (kind, payload) in rows {
            let payload: serde_json::Value = serde_json::from_str(&payload)?;
            let reason = match kind.as_str() {
                "TaskFailed" => payload.get("reason").and_then(|v| v.as_str()),
                "PolicyEvaluated" => {
                    if payload.get("decision").and_then(|v| v.as_str()) == Some("deny") {
                        payload
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .map(|s| s.strip_prefix("policy: ").unwrap_or(s))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(reason) = reason {
                *denials.entry(reason.to_string()).or_insert(0) += 1;
            }
        }
        stats.top_denials = denials.into_iter().collect();
        stats
            .top_denials
            .sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        Ok(stats)
    }

    /// Observed per-capability facts from the event log (Phase 12 world
    /// model).
    pub fn world_stats(&self) -> Result<WorldStatsDoc> {
        let mut stmt = self
            .conn
            .prepare("SELECT record FROM intents ORDER BY compiled_at, id")?;
        let records = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out: WorldStatsDoc = BTreeMap::new();
        for record in records {
            let doc: IntentDocument = serde_json::from_str(&record)?;
            let Some(cap) = doc.capabilities().first().cloned() else {
                continue;
            };
            let entry = out.entry(cap).or_default();
            let mut started_at: Option<i64> = None;
            for ev in &doc.events {
                if ev.payload.get("shadow") == Some(&serde_json::Value::Bool(true)) {
                    continue;
                }
                match ev.kind.as_str() {
                    "TaskStarted" => started_at = ev.at,
                    "TaskSucceeded" => {
                        if started_at.is_some() {
                            entry.samples += 1;
                            entry.successes += 1;
                            if let (Some(start), Some(end)) = (started_at, ev.at) {
                                if end >= start {
                                    entry.durations_ms.push((end - start) as u64);
                                }
                            }
                        }
                        started_at = None;
                    }
                    "TaskFailed" => {
                        if started_at.is_some() {
                            entry.samples += 1;
                        }
                        started_at = None;
                    }
                    _ => {}
                }
            }
        }
        Ok(out)
    }

    /// Whether a policy-denied execution exists for testing aggregation.
    pub fn deny_count(&self) -> Result<u64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE kind = 'PolicyEvaluated' AND payload LIKE '%\"decision\":\"deny\"%'",
                [],
                |row| row.get::<_, i64>(0).map(|n| n as u64),
            )
            .map_err(Into::into)
    }
}

/// Unique-enough temp dir name for tests (no uuid dep in this crate).
#[cfg(test)]
fn tmp_tag(name: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("imperium-ledger-{}-{}-{name}", std::process::id(), n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(
        status: &str,
        name: &str,
        output: Option<&str>,
        events: Vec<(&str, serde_json::Value)>,
    ) -> IntentDocument {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(1);
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        IntentDocument {
            status: status.into(),
            ir: serde_json::json!({
                "id": format!("00000000-0000-0000-0000-{n:012}"),
                "name": name,
                "nl_source": format!("Echo this message: {name}"),
                "risk_score": 0.0,
                "requires_approval": false,
                "compiled_at": "2026-01-01T00:00:00Z",
                "tasks": [{"name": "Echo", "description": name, "capabilities": ["cap.echo"]}]
            }),
            simulation: None,
            output: output.map(String::from),
            token: None,
            events: events
                .into_iter()
                .map(|(kind, payload)| EventDocument {
                    kind: kind.into(),
                    payload,
                    at: None,
                })
                .collect(),
        }
    }

    #[test]
    fn sync_search_and_stats() {
        let tmp = std::env::temp_dir().join(tmp_tag("sync"));
        let ledger = Ledger::open(&tmp.join("ledger.db")).unwrap();
        let ping = doc("executed", "ping", Some("ping"), vec![]);
        ledger.sync(&ping).unwrap();
        ledger
            .sync(&doc(
                "failed",
                "boom",
                None,
                vec![("TaskFailed", serde_json::json!({"reason": "nonce reused"}))],
            ))
            .unwrap();
        ledger.sync(&ping).unwrap();

        let hits = ledger.search("ping").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].status, "executed");

        let stats = ledger.stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.executed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.top_capabilities[0], ("cap.echo".into(), 2));
        assert_eq!(stats.top_denials[0], ("nonce reused".into(), 1));
    }

    #[test]
    fn world_stats_aggregates_executions_only() {
        let tmp = std::env::temp_dir().join(tmp_tag("world"));
        let ledger = Ledger::open(&tmp.join("ledger.db")).unwrap();
        let ev = |kind: &str, at: Option<i64>, payload: serde_json::Value| EventDocument {
            kind: kind.into(),
            payload,
            at,
        };
        let mut ok = doc("executed", "ping", Some("ping"), vec![]);
        ok.events = vec![
            ev("IntentCompiled", Some(90), serde_json::json!({})),
            ev(
                "TaskStarted",
                Some(100),
                serde_json::json!({"task_id": "t1"}),
            ),
            ev(
                "TaskSucceeded",
                Some(180),
                serde_json::json!({"output": "ping"}),
            ),
        ];
        ledger.sync(&ok).unwrap();
        let mut fail = doc("failed", "boom", None, vec![]);
        fail.events = vec![
            ev(
                "TaskStarted",
                Some(200),
                serde_json::json!({"task_id": "t1"}),
            ),
            ev(
                "TaskFailed",
                Some(250),
                serde_json::json!({"reason": "denied"}),
            ),
        ];
        ledger.sync(&fail).unwrap();
        // Compile-only: never started — not a sample.
        ledger
            .sync(&doc(
                "compiled",
                "noop",
                None,
                vec![("IntentCompiled", serde_json::json!({}))],
            ))
            .unwrap();

        let world = ledger.world_stats().unwrap();
        let echo = world.get("cap.echo").unwrap();
        assert_eq!(echo.samples, 2);
        assert_eq!(echo.successes, 1);
        assert_eq!(echo.durations_ms, vec![80]);
    }

    #[test]
    fn rebuild_from_intents_dir() {
        let tmp = std::env::temp_dir().join(tmp_tag("rebuild"));
        let intents = tmp.join("intents");
        std::fs::create_dir_all(&intents).unwrap();
        let mut d = doc("executed", "alpha", Some("alpha"), vec![]);
        let id = d.id().unwrap();
        std::fs::write(
            intents.join(format!("{id}.json")),
            serde_json::to_string_pretty(&d).unwrap(),
        )
        .unwrap();
        d.status = "failed".into();
        d.output = None;
        std::fs::write(
            intents.join(format!("{id}.json")),
            serde_json::to_string_pretty(&d).unwrap(),
        )
        .unwrap();

        let ledger = Ledger::open(&tmp.join("ledger.db")).unwrap();
        // Seed a stale row, then rebuild should replace it.
        ledger
            .sync(&doc("executed", "stale", Some("stale"), vec![]))
            .unwrap();
        let n = ledger.rebuild(&intents).unwrap();
        assert_eq!(n, 1);
        let hits = ledger.search("alpha").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].status, "failed");
        assert!(ledger.search("stale").unwrap().is_empty());
    }

    #[test]
    fn semantic_key_ignores_description() {
        let a = doc("compiled", "a", None, vec![]);
        let mut b = doc("compiled", "b", None, vec![]);
        // Same capability/task/target; different description should not change key.
        if let Some(tasks) = b.ir.get_mut("tasks").and_then(|t| t.as_array_mut()) {
            if let Some(task) = tasks.get_mut(0) {
                task.as_object_mut()
                    .unwrap()
                    .insert("description".into(), serde_json::json!("other"));
            }
        }
        assert_eq!(a.semantic_key(), b.semantic_key());
    }

    #[test]
    fn search_treats_like_metacharacters_literally() {
        let tmp = std::env::temp_dir().join(tmp_tag("like"));
        let ledger = Ledger::open(&tmp.join("ledger.db")).unwrap();
        ledger.sync(&doc("compiled", "a_b", None, vec![])).unwrap();
        ledger.sync(&doc("compiled", "aXb", None, vec![])).unwrap();
        // Literal underscore matches only the record that contains it.
        let hits = ledger.search("a_b").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "a_b");
        // Underscore must not act as a single-char wildcard.
        assert!(ledger.search("aXb").unwrap().len() == 1);
        assert!(ledger.search("aYb").unwrap().is_empty());
        // Percent is literal too.
        assert!(ledger.search("%").unwrap().is_empty());
    }

    #[test]
    fn deny_count_sees_policy_denies() {
        let tmp = std::env::temp_dir().join(tmp_tag("deny"));
        let ledger = Ledger::open(&tmp.join("ledger.db")).unwrap();
        let doc = doc(
            "failed",
            "blocked",
            None,
            vec![(
                "PolicyEvaluated",
                serde_json::json!({"decision": "deny", "reason": "policy: blocked"}),
            )],
        );
        ledger.sync(&doc).unwrap();
        assert_eq!(ledger.deny_count().unwrap(), 1);
    }

    #[test]
    fn world_stats_ignores_shadow_runs() {
        let tmp = std::env::temp_dir().join(tmp_tag("shadow-world"));
        let ledger = Ledger::open(&tmp.join("ledger.db")).unwrap();
        let ev = |kind: &str, at: Option<i64>, payload: serde_json::Value| EventDocument {
            kind: kind.into(),
            payload,
            at,
        };
        // A redirected shadow run: real TaskStarted/Succeeded pair, but
        // tagged shadow: provenance, not capability performance data.
        let mut shadow = doc("approved", "shadow-run", None, vec![]);
        shadow.events = vec![
            ev(
                "TaskStarted",
                Some(100),
                serde_json::json!({"task_id": "t1", "shadow": true}),
            ),
            ev(
                "TaskSucceeded",
                Some(120),
                serde_json::json!({"shadow": true, "output": "wrote scratch/shadow/x.txt"}),
            ),
        ];
        ledger.sync(&shadow).unwrap();
        // One real execution of the same capability.
        let mut real = doc("executed", "real-run", Some("ping"), vec![]);
        real.events = vec![
            ev(
                "TaskStarted",
                Some(200),
                serde_json::json!({"task_id": "t1"}),
            ),
            ev(
                "TaskSucceeded",
                Some(230),
                serde_json::json!({"output": "ping"}),
            ),
        ];
        ledger.sync(&real).unwrap();

        let world = ledger.world_stats().unwrap();
        let echo = world.get("cap.echo").expect("real run is a sample");
        assert_eq!(echo.samples, 1, "shadow runs must not count as samples");
        assert_eq!(echo.successes, 1);
        assert_eq!(echo.durations_ms, vec![30]);
    }
}
