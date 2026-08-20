//! File-backed v0 loop used by the CLI.

use anyhow::{anyhow, bail, Context, Result};
use imperium_core::v0::*;
use imperium_core::IntentIR;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub token: CapabilityToken,
    pub used: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredIntent {
    pub status: IntentStatus,
    pub ir: IntentIR,
    pub simulation: Option<SimulationResult>,
    pub output: Option<String>,
    pub token: Option<StoredToken>,
    pub events: Vec<V0Event>,
}

pub struct V0Home {
    pub root: PathBuf,
}

impl V0Home {
    pub fn discover() -> Result<Self> {
        let root = std::env::var("IMPERIUM_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".imperium"));
        Ok(Self { root })
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("intents"))?;
        fs::create_dir_all(self.root.join("scratch"))?;
        let secret = self.root.join("token.secret");
        if !secret.exists() {
            fs::write(&secret, uuid::Uuid::new_v4().to_string())?;
        }
        Ok(())
    }

    fn secret(&self) -> Result<String> {
        let path = self.root.join("token.secret");
        if !path.exists() {
            self.init()?;
        }
        Ok(fs::read_to_string(path)?.trim().to_string())
    }

    fn intent_path(&self, id: &str) -> PathBuf {
        self.root.join("intents").join(format!("{id}.json"))
    }

    pub fn load(&self, id: &str) -> Result<StoredIntent> {
        let path = self.intent_path(id);
        let data = fs::read_to_string(&path)
            .with_context(|| format!("intent not found: {}", path.display()))?;
        Ok(serde_json::from_str(&data)?)
    }

    fn save(&self, rec: &StoredIntent) -> Result<()> {
        self.init()?;
        let path = self.intent_path(&rec.ir.id.to_string());
        fs::write(path, serde_json::to_string_pretty(rec)?)?;
        Ok(())
    }

    fn append(rec: &mut StoredIntent, kind: &str, payload: serde_json::Value) {
        rec.events.push(V0Event {
            kind: kind.into(),
            payload,
        });
    }

    pub fn compile(&self, nl: &str, propose: bool) -> Result<StoredIntent> {
        let (canonical, proposer) = if propose {
            local_propose(nl).map_err(|e| anyhow!(e))?
        } else {
            (nl.to_string(), "rules")
        };
        let mut ir = compile_rules(&canonical).map_err(|e| anyhow!(e))?;
        if propose {
            ir.nl_source = nl.trim().to_string();
            ir.compiler_version = Some(format!(
                "{}+{proposer}",
                ir.compiler_version.clone().unwrap_or_default()
            ));
        }
        ir.validate().map_err(|e| anyhow!(e))?;
        let mut rec = StoredIntent {
            status: IntentStatus::Compiled,
            ir,
            simulation: None,
            output: None,
            token: None,
            events: vec![],
        };
        if propose {
            Self::append(
                &mut rec,
                "IntentProposed",
                serde_json::json!({"proposer": proposer, "canonical": canonical}),
            );
        }
        let version = rec.ir.compiler_version.clone();
        Self::append(
            &mut rec,
            "IntentCompiled",
            serde_json::json!({"compiler_version": version}),
        );
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn simulate(&self, id: &str) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        let sim = simulate_static(&rec.ir);
        rec.simulation = Some(sim.clone());
        rec.status = IntentStatus::Simulated;
        Self::append(
            &mut rec,
            "IntentSimulated",
            serde_json::json!({
                "success_probability": sim.success_probability,
                "risk": sim.risk,
                "duration_ms": sim.duration_ms,
                "notes": sim.notes,
            }),
        );
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn approve(&self, id: &str) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        let sim = rec
            .simulation
            .as_ref()
            .ok_or_else(|| anyhow!("Simulate before approval."))?;
        if sim.risk > 0.0 || sim.success_probability < 1.0 {
            bail!("Policy: high-risk simulation cannot be approved.");
        }
        let cap = rec
            .ir
            .tasks
            .first()
            .and_then(|t| t.capabilities.first())
            .cloned()
            .unwrap_or_else(|| ECHO_CAP.into());
        let now = now_ms();
        let token = issue_token(
            &cap,
            "cli",
            &rec.ir.id.to_string(),
            grant_for_capability(&cap),
            now,
            15 * 60 * 1000,
            &self.secret()?,
        );
        rec.status = IntentStatus::Approved;
        rec.token = Some(StoredToken {
            token: token.clone(),
            used: false,
            revoked: false,
        });
        Self::append(&mut rec, "IntentApproved", serde_json::json!({}));
        Self::append(
            &mut rec,
            "TokenIssued",
            serde_json::json!({
                "token_id": token.id,
                "fingerprint": token.signature.chars().take(12).collect::<String>(),
            }),
        );
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn revoke(&self, id: &str) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        let token = rec
            .token
            .as_mut()
            .ok_or_else(|| anyhow!("No token to revoke."))?;
        token.revoked = true;
        let tid = token.token.id.clone();
        Self::append(&mut rec, "TokenRevoked", serde_json::json!({"token_id": tid}));
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn execute(&self, id: &str) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        if rec.status != IntentStatus::Approved {
            bail!("Execute without approve is rejected.");
        }
        let stored = rec
            .token
            .clone()
            .ok_or_else(|| anyhow!("No capability token. Approve first."))?;
        let mut seen = HashSet::new();
        if stored.used {
            seen.insert(stored.token.nonce.clone());
        }
        let mut revoked = HashSet::new();
        if stored.revoked {
            revoked.insert(stored.token.id.clone());
        }
        let cap = rec
            .ir
            .tasks
            .first()
            .and_then(|t| t.capabilities.first())
            .cloned()
            .unwrap_or_else(|| ECHO_CAP.into());
        let check = verify_token(
            &stored.token,
            &self.secret()?,
            now_ms(),
            &seen,
            &revoked,
            &grant_for_capability(&cap),
            Some("cli"),
            Some(&rec.ir.id.to_string()),
        );
        if check != VerifyReason::Ok {
            rec.status = IntentStatus::Failed;
            Self::append(
                &mut rec,
                "TaskFailed",
                serde_json::json!({"reason": check.as_str()}),
            );
            self.save(&rec)?;
            bail!("Token verify failed: {}", check.as_str());
        }
        if stored.token.capability != cap {
            rec.status = IntentStatus::Failed;
            Self::append(
                &mut rec,
                "TaskFailed",
                serde_json::json!({"reason": "capability mismatch"}),
            );
            self.save(&rec)?;
            bail!("Token capability does not match task.");
        }
        if let Some(t) = rec.token.as_mut() {
            t.used = true;
        }
        let task_id = rec.ir.tasks[0].id.to_string();
        Self::append(&mut rec, "TaskStarted", serde_json::json!({"task_id": task_id}));

        let output = if cap == WRITE_CAP {
            let path = rec.ir.tasks[0]
                .target_path
                .clone()
                .ok_or_else(|| anyhow!("Write target missing."))?;
            let contents = rec.ir.tasks[0].description.clone();
            if !path_allowed(&path, &stored.token.permissions.fs) {
                rec.status = IntentStatus::Failed;
                Self::append(
                    &mut rec,
                    "TaskFailed",
                    serde_json::json!({"reason": "host.write path denied"}),
                );
                self.save(&rec)?;
                bail!("host.write path denied");
            }
            write_scratch(&self.root, &path, &contents)?;
            format!("wrote {path} ({} bytes)", contents.len())
        } else {
            rec.ir.tasks[0].description.clone()
        };
        rec.output = Some(output.clone());
        rec.status = IntentStatus::Executed;
        Self::append(&mut rec, "TaskSucceeded", serde_json::json!({"output": output}));
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn replay(&self, id: &str) -> Result<(StoredIntent, FoldedState, bool)> {
        let rec = self.load(id)?;
        let folded = fold_events(&rec.events);
        let matches = folded.status.as_ref() == Some(&rec.status)
            && folded.output == rec.output;
        Ok((rec, folded, matches))
    }

    pub fn list(&self) -> Result<Vec<StoredIntent>> {
        let dir = self.root.join("intents");
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out: Vec<StoredIntent> = vec![];
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let data = fs::read_to_string(entry.path())?;
            out.push(serde_json::from_str(&data)?);
        }
        out.sort_by(|a, b| a.ir.name.cmp(&b.ir.name));
        Ok(out)
    }
}

fn write_scratch(root: &Path, rel: &str, contents: &str) -> Result<()> {
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dest, contents)?;
    Ok(())
}

pub fn resolve_nl(input: &str) -> Result<String> {
    let path = Path::new(input);
    if path.is_file() {
        Ok(fs::read_to_string(path)?)
    } else {
        Ok(input.to_string())
    }
}
