//! File-backed v0 loop used by the CLI.

use anyhow::{anyhow, bail, Context, Result};
use imperium_core::forms::{build_template, render_template, verify_shadow, FormTemplate};
use imperium_core::policy::imp;
use imperium_core::synth::{synthesize, CapabilityManifest};
use imperium_core::v0::*;
use imperium_core::IntentIR;
use imperium_store::{IntentDocument, Ledger};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
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
#[serde(deny_unknown_fields)]
pub struct GrantFile {
    pub grants: BTreeMap<String, Permissions>,
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
        let grants = self.root.join("grants.json");
        if !grants.exists() {
            fs::write(
                &grants,
                serde_json::to_string_pretty(&GrantFile {
                    grants: initial_grants(),
                })?,
            )?;
        }
        Ok(())
    }

    /// Load and validate `.imperium/grants.json`. Fail-closed: unknown
    /// capabilities or grants wider than the built-in defaults are errors.
    pub fn load_grants(&self) -> Result<BTreeMap<String, Permissions>> {
        let path = self.root.join("grants.json");
        let data = fs::read_to_string(&path)
            .with_context(|| format!("grants file unreadable: {}", path.display()))?;
        let parsed: GrantFile =
            serde_json::from_str(&data).with_context(|| "grants.json is not valid JSON")?;
        let defaults = default_grants();
        let mut out = BTreeMap::new();
        for (cap, declared) in &parsed.grants {
            let Some(default) = defaults.get(cap) else {
                bail!("grants.json declares unknown capability: {cap}");
            };
            if !is_permission_subset(declared, default) {
                bail!("grants.json grant for {cap} exceeds built-in defaults");
            }
            out.insert(cap.clone(), declared.clone());
        }
        Ok(out)
    }

    fn declared_grant(&self, capability: &str) -> Result<Permissions> {
        let grants = self.load_grants()?;
        Ok(grants.get(capability).cloned().unwrap_or_default())
    }

    /// Load `.imperium/policy.imp` if present. Fail-closed: parse issues
    /// refuse to load (the policy is a security boundary, not a suggestion).
    pub fn load_policy(&self) -> Result<Option<imp::Policy>> {
        let path = self.root.join("policy.imp");
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path)
            .with_context(|| format!("policy file unreadable: {}", path.display()))?;
        let (policy, issues) = imp::parse_policy_lenient(&data);
        if !issues.is_empty() {
            let detail = issues
                .iter()
                .map(|i| format!("line {}: {}", i.line, i.message))
                .collect::<Vec<_>>()
                .join("; ");
            bail!("policy.imp failed to parse (fail-closed): {detail}");
        }
        Ok(Some(policy))
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
        // The ledger is a projection of the canonical record: sync on save.
        self.ledger()?.sync(&self.as_document(rec)?)?;
        Ok(())
    }

    pub fn ledger(&self) -> Result<Ledger> {
        Ledger::open(&self.root.join("ledger.db"))
    }

    pub fn as_document(&self, rec: &StoredIntent) -> Result<IntentDocument> {
        Ok(serde_json::from_value(serde_json::to_value(rec)?)?)
    }

    /// Semantic identity of a record via the store's canonical helper.
    pub fn semantic_key_of(&self, rec: &StoredIntent) -> Option<String> {
        self.as_document(rec).ok()?.semantic_key()
    }

    /// World facts (store view) mapped into the core kernel's input type.
    pub fn core_world_stats(&self) -> Result<std::collections::BTreeMap<String, CapabilityStat>> {
        let stats = self.ledger()?.world_stats()?;
        Ok(stats
            .into_iter()
            .map(|(cap, s)| {
                (
                    cap,
                    CapabilityStat {
                        samples: s.samples,
                        successes: s.successes,
                        durations_ms: s.durations_ms,
                    },
                )
            })
            .collect())
    }

    // --- Phase 11: capability synthesis registry ---

    fn capabilities_dir(&self) -> PathBuf {
        self.root.join("capabilities")
    }

    /// Synthesize a manifest from an OpenAPI spec and register it unapproved.
    /// Registration records intent; it grants nothing (policy + grants still
    /// gate every fetch).
    pub fn add_capability(&self, name: &str, spec_text: &str) -> Result<CapabilityManifest> {
        let manifest = synthesize(name, spec_text).map_err(|e| anyhow!(e))?;
        let path = self.capabilities_dir().join(format!("{name}.json"));
        if path.exists() {
            bail!("capability already registered: {name}");
        }
        fs::create_dir_all(self.capabilities_dir())?;
        fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
        Ok(manifest)
    }

    /// Explicit human approval flips the manifest's approved flag.
    pub fn approve_capability(&self, name: &str) -> Result<CapabilityManifest> {
        let path = self.capabilities_dir().join(format!("{name}.json"));
        let data = fs::read_to_string(&path)
            .with_context(|| format!("capability not registered: {name}"))?;
        let mut manifest: CapabilityManifest = serde_json::from_str(&data)?;
        if manifest.approved {
            bail!("capability already approved: {name}");
        }
        manifest.approved = true;
        manifest.approved_at = Some(chrono::Utc::now().to_rfc3339());
        fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
        Ok(manifest)
    }

    pub fn list_capabilities(&self) -> Result<Vec<(String, CapabilityManifest)>> {
        let dir = self.capabilities_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let data = fs::read_to_string(entry.path())?;
            let manifest: CapabilityManifest = serde_json::from_str(&data)?;
            let name = manifest.name.clone();
            out.push((name, manifest));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    // --- Phase 13: the evolution loop (forms + shadow) ---

    fn forms_dir(&self) -> PathBuf {
        self.root.join("forms")
    }

    /// Save a human-approved reusable template from a compiled intent.
    /// The form is a convenience, never a privilege: every run re-enters
    /// the full gauntlet.
    pub fn save_form(&self, intent_id: &str, name: &str) -> Result<FormTemplate> {
        let rec = self.load(intent_id)?;
        let template = build_template(&rec.ir).map_err(|e| anyhow!(e))?;
        let path = self.forms_dir().join(format!("{name}.json"));
        if path.exists() {
            bail!("form already exists: {name}");
        }
        fs::create_dir_all(self.forms_dir())?;
        fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::json!({
                "name": name,
                "template": template.template,
                "slot": template.slot,
                "created_from": intent_id,
                "created_at": chrono::Utc::now().to_rfc3339(),
            }))?,
        )?;
        Ok(template)
    }

    fn load_form(&self, name: &str) -> Result<(String, String)> {
        let path = self.forms_dir().join(format!("{name}.json"));
        let data = fs::read_to_string(&path).with_context(|| format!("form not found: {name}"))?;
        let doc: serde_json::Value = serde_json::from_str(&data)?;
        let template = doc
            .get("template")
            .and_then(|v| v.as_str())
            .context("form has no template")?
            .to_string();
        Ok((
            doc.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string(),
            template,
        ))
    }

    /// Run a form: substitute the slot, compile, simulate — then stop for
    /// the normal human approval. The gauntlet is never skipped.
    pub fn run_form(
        &self,
        name: &str,
        slot_value: &str,
        mc: Option<(u64, Option<u64>)>,
    ) -> Result<StoredIntent> {
        let (form_name, template) = self.load_form(name)?;
        let canonical = render_template(&template, slot_value).map_err(|e| anyhow!(e))?;
        let mut rec = self.compile(&canonical, false)?;
        Self::append(
            &mut rec,
            "FormRun",
            serde_json::json!({"form": form_name, "slot_value": slot_value}),
        );
        self.save(&rec)?;
        let id = rec.ir.id.to_string();
        self.simulate_opts(&id, mc)
    }

    /// Form usage summary, computed as a view over the ledger:
    /// runs = FormRun events, verified = matching ShadowVerified events.
    pub fn form_usage(&self, name: &str) -> Result<(u64, u64)> {
        let mut runs = 0;
        let mut verified = 0;
        for rec in self.list()? {
            for ev in &rec.events {
                match ev.kind.as_str() {
                    "FormRun" => {
                        if ev.payload.get("form").and_then(|v| v.as_str()) == Some(name) {
                            runs += 1;
                        }
                    }
                    "ShadowVerified" => {
                        // Count only matches that belong to a run of this form.
                        let of_this_form = rec.events.iter().any(|e| {
                            matches!(e.kind.as_str(), "FormRun"
                                if e.payload.get("form").and_then(|v| v.as_str()) == Some(name))
                        });
                        if ev.payload.get("match").and_then(|v| v.as_bool()) == Some(true)
                            && of_this_form
                        {
                            verified += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok((runs, verified))
    }

    pub fn list_forms(&self) -> Result<Vec<String>> {
        let dir = self.forms_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            out.push(
                entry
                    .path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
            );
        }
        out.sort();
        Ok(out)
    }

    fn append(rec: &mut StoredIntent, kind: &str, payload: serde_json::Value) {
        rec.events.push(V0Event {
            kind: kind.into(),
            payload,
            at: Some(now_ms()),
        });
    }

    pub fn compile(&self, nl: &str, propose: bool) -> Result<StoredIntent> {
        let (canonical, proposer) = if propose {
            local_propose(nl).map_err(|e| anyhow!(e))?
        } else {
            (nl.to_string(), "rules")
        };
        // User policy content rules extend the built-in proposal ceiling.
        if propose {
            if let Some(policy) = self.load_policy()? {
                if let Some((_, reason)) = policy.content_denied(nl) {
                    bail!("Policy denied proposal: {reason}");
                }
            }
        }
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
        // Friction signal (spec 04 seed): the third+ compile of a semantically
        // identical intent suggests a reusable form. No auto-synthesis.
        let key = self.semantic_key_of(&rec);
        if let Some(key) = key {
            let prior = self
                .list()?
                .iter()
                .filter(|r| self.semantic_key_of(r).as_deref() == Some(key.as_str()))
                .count()
                + 1;
            if prior >= 3 {
                Self::append(
                    &mut rec,
                    "FrictionDetected",
                    serde_json::json!({
                        "count": prior,
                        "key": key,
                        "suggestion": "consider saving this as a reusable form",
                    }),
                );
            }
        }
        self.save(&rec)?;
        Ok(rec)
    }

    /// Simulate, optionally as a seeded Monte Carlo over the world model.
    /// `trials` triggers the probabilistic path; `seed: None` derives the
    /// seed from the intent id (reproducible per intent).
    pub fn simulate_opts(&self, id: &str, mc: Option<(u64, Option<u64>)>) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        let policy = self.load_policy()?;
        let (sim, dry_events) = match mc {
            Some((trials, seed)) => {
                let seed = seed.unwrap_or_else(|| derive_seed(&rec.ir.id.to_string()));
                let core_stats = self.core_world_stats()?;
                let s =
                    dry_run_monte_carlo(&rec.ir, policy.as_ref(), Some(&core_stats), trials, seed);
                // The deterministic preview events remain the audit trail.
                let (_, evs) = dry_run_events_with_policy(&rec.ir, policy.as_ref());
                (s, evs)
            }
            None => dry_run_events_with_policy(&rec.ir, policy.as_ref()),
        };
        debug_assert!(dry_events
            .iter()
            .all(|e| { e.payload.get("dry_run") == Some(&serde_json::value::Value::Bool(true)) }));
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
                "effects_preview": sim.effects_preview,
            }),
        );
        self.save(&rec)?;
        Ok(rec)
    }

    fn issue_token_for(&self, rec: &StoredIntent) -> Result<StoredToken> {
        let cap = rec
            .ir
            .tasks
            .first()
            .and_then(|t| t.capabilities.first())
            .cloned()
            .unwrap_or_else(|| ECHO_CAP.into());
        let declared = self.declared_grant(&cap)?;
        let now = now_ms();
        let token = issue_token(
            &cap,
            "cli",
            &rec.ir.id.to_string(),
            declared,
            now,
            15 * 60 * 1000,
            &self.secret()?,
        );
        Ok(StoredToken {
            token,
            used: false,
            revoked: false,
        })
    }

    pub fn approve(&self, id: &str) -> Result<StoredIntent> {
        self.approve_internal(id, false)
    }

    fn approve_internal(&self, id: &str, auto: bool) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        let sim = rec
            .simulation
            .as_ref()
            .ok_or_else(|| anyhow!("Simulate before approval."))?;
        // Approval gate: static sims need p == 1; probabilistic sims use the
        // integer-exact Phase 12 threshold (p_success >= 0.9).
        let passes = if sim.probabilistic {
            mc_gate(sim.mc_successes, sim.trials)
        } else {
            sim.success_probability >= 1.0
        };
        if !passes {
            bail!(
                "Policy: simulation below approval threshold (p={}).",
                if sim.probabilistic {
                    format!("{:.4}", sim.p_success)
                } else {
                    format!("{}", sim.success_probability)
                }
            );
        }
        let policy_required = self.policy_requires_approval(&rec)?;
        let issued = self.issue_token_for(&rec)?;
        let token_id = issued.token.id.clone();
        let fingerprint = issued.token.signature.chars().take(12).collect::<String>();
        let risk = rec.ir.risk_score;
        rec.status = IntentStatus::Approved;
        rec.token = Some(issued);
        let mut payload = if auto {
            serde_json::json!({"auto": true, "risk": risk})
        } else {
            serde_json::json!({})
        };
        if policy_required {
            payload["policy_approval"] = serde_json::json!(true);
        }
        Self::append(&mut rec, "IntentApproved", payload);
        Self::append(
            &mut rec,
            "TokenIssued",
            serde_json::json!({
                "token_id": token_id,
                "fingerprint": fingerprint,
                "auto": auto,
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
        Self::append(
            &mut rec,
            "TokenRevoked",
            serde_json::json!({"token_id": tid}),
        );
        self.save(&rec)?;
        Ok(rec)
    }

    /// Does the user policy force explicit approval for this intent's verb?
    fn policy_requires_approval(&self, rec: &StoredIntent) -> Result<bool> {
        let policy = self.load_policy()?;
        let verb = rec
            .ir
            .tasks
            .first()
            .and_then(|t| t.capabilities.first())
            .map(|c| host_verb(c))
            .unwrap_or("effect");
        Ok(policy.is_some_and(|p| p.requires_approval(verb)))
    }

    pub fn execute(&self, id: &str) -> Result<StoredIntent> {
        self.execute_with_transport(id, &UreqTransport)
    }

    /// Execute with an injected transport — tests never touch the network.
    pub fn execute_with_transport(
        &self,
        id: &str,
        transport: &dyn HttpTransport,
    ) -> Result<StoredIntent> {
        self.execute_inner(id, transport, false)
    }

    /// Shadow execution (Phase 13): the full gauntlet, but destructive
    /// effects redirect under `scratch/shadow/` and the run folds a
    /// `ShadowVerified` event comparing promise vs reality. The intent's
    /// status is unchanged by a shadow run.
    pub fn execute_shadow(&self, id: &str) -> Result<StoredIntent> {
        self.execute_inner(id, &UreqTransport, true)
    }

    fn execute_inner(
        &self,
        id: &str,
        transport: &dyn HttpTransport,
        shadow: bool,
    ) -> Result<StoredIntent> {
        let mut rec = self.load(id)?;
        if rec.status == IntentStatus::Simulated && !rec.ir.requires_approval {
            // Low-risk policy: auto-approve with full audit trail — unless
            // the user policy forces explicit approval for this verb.
            if self.policy_requires_approval(&rec)? {
                bail!("Execute without approve is rejected.");
            }
            rec = self.approve_internal(id, true)?;
        }
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
        let declared = self.declared_grant(&cap)?;
        let secret = self.secret()?;
        let check = verify_token(
            &stored.token,
            &VerifyContext {
                secret: &secret,
                now_ms: now_ms(),
                seen_nonces: &seen,
                revoked_ids: &revoked,
                grant: &declared,
                expected_subject: Some("cli"),
                expected_intent: Some(&rec.ir.id.to_string()),
            },
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
        Self::append(
            &mut rec,
            "TaskStarted",
            serde_json::json!({"task_id": task_id, "shadow": shadow}),
        );

        let task_desc = rec.ir.tasks[0].description.clone();
        let task_path = rec.ir.tasks[0].target_path.clone();
        let fail = |rec: &mut StoredIntent, reason: &str| -> anyhow::Error {
            rec.status = IntentStatus::Failed;
            Self::append(rec, "TaskFailed", serde_json::json!({"reason": reason}));
            anyhow!(reason.to_string())
        };
        let output = if shadow && cap == HTTP_CAP {
            let e = fail(&mut rec, "fetch cannot run in shadow mode");
            self.save(&rec)?;
            return Err(e);
        } else if cap == ECHO_CAP {
            task_desc.clone()
        } else if cap == HTTP_CAP {
            // --- Fetch gate: URL checks → token net → policy (default-deny).
            let url = task_desc.clone();
            let host = match fetch_url_host(&url) {
                Ok(h) => h,
                Err(reason) => {
                    let e = fail(&mut rec, &format!("host.fetch {reason}"));
                    self.save(&rec)?;
                    return Err(e);
                }
            };
            if !stored
                .token
                .permissions
                .net
                .iter()
                .any(|g| g == "*" || g == &host)
            {
                let e = fail(&mut rec, "host.fetch host denied");
                self.save(&rec)?;
                return Err(e);
            }
            let policy = self.load_policy()?;
            let (deny_reason, decision): (Option<String>, Option<imp::PolicyDecision>) =
                match policy.as_ref() {
                    None => (Some(FETCH_NO_ALLOW_RULE.into()), None),
                    Some(p) => {
                        let d = p.evaluate("fetch", &host, &url);
                        let deny = match &d {
                            imp::PolicyDecision::Deny { reason, .. } => {
                                Some(format!("policy: {reason}"))
                            }
                            imp::PolicyDecision::Allow { rule: Some(_) } => None,
                            _ => Some(FETCH_NO_ALLOW_RULE.into()),
                        };
                        (deny, Some(d))
                    }
                };
            Self::append(
                &mut rec,
                "PolicyEvaluated",
                serde_json::json!({
                    "verb": "fetch",
                    "path": host,
                    "decision": match (&decision, &deny_reason) {
                        (_, Some(_)) => "deny",
                        (_, None) => "allow",
                    },
                    "rule": match &decision {
                        Some(imp::PolicyDecision::Allow { rule }) => (*rule).map(|r| serde_json::json!(r)),
                        Some(imp::PolicyDecision::RequireApproval { rule }) => Some(serde_json::json!(rule)),
                        Some(imp::PolicyDecision::Deny { rule, .. }) => Some(serde_json::json!(rule)),
                        None => None,
                    },
                    "reason": deny_reason,
                }),
            );
            if let Some(reason) = deny_reason {
                let e = fail(&mut rec, &reason);
                self.save(&rec)?;
                return Err(e);
            }
            match transport.get(&url) {
                Ok(body) => body,
                Err(e) => {
                    let reason = e.to_string();
                    let e2 = fail(&mut rec, &reason);
                    self.save(&rec)?;
                    return Err(e2);
                }
            }
        } else {
            let Some(path) = task_path.clone() else {
                let e = fail(&mut rec, "target path missing");
                self.save(&rec)?;
                return Err(e);
            };
            // Host enforcement: granted prefix, then sensitive-path deny.
            if !path_allowed(&path, &stored.token.permissions.fs) {
                let reason = format!("host.{} path denied", host_verb(&cap));
                let e = fail(&mut rec, &reason);
                self.save(&rec)?;
                return Err(e);
            }
            if is_sensitive_path(&path) {
                let reason = format!("host.{} {}", host_verb(&cap), SENSITIVE_DENIED_REASON);
                let e = fail(&mut rec, &reason);
                self.save(&rec)?;
                return Err(e);
            }
            // User policy layer, audited: every fs action records its decision.
            let policy = self.load_policy()?;
            if let Some(p) = policy.as_ref() {
                let action_text = if cap == WRITE_CAP || cap == APPEND_CAP {
                    format!("{path} {task_desc}")
                } else {
                    path.clone()
                };
                let decision = p.evaluate(host_verb(&cap), &path, &action_text);
                let reason = match &decision {
                    imp::PolicyDecision::Deny { reason, .. } => Some(format!("policy: {reason}")),
                    _ => None,
                };
                Self::append(
                    &mut rec,
                    "PolicyEvaluated",
                    serde_json::json!({
                        "verb": host_verb(&cap),
                        "path": path,
                        "decision": decision.kind(),
                        "rule": match &decision {
                            imp::PolicyDecision::Allow { rule } => (*rule).map(|r| serde_json::json!(r)),
                            imp::PolicyDecision::RequireApproval { rule } => Some(serde_json::json!(rule)),
                            imp::PolicyDecision::Deny { rule, .. } => Some(serde_json::json!(rule)),
                        },
                        "reason": reason,
                    }),
                );
                if let Some(reason) = reason {
                    let e = fail(&mut rec, &reason);
                    self.save(&rec)?;
                    return Err(e);
                }
            }
            match cap.as_str() {
                WRITE_CAP => {
                    let contents = task_desc.clone();
                    let eff_path = if shadow {
                        format!(
                            "scratch/shadow/{}",
                            path.strip_prefix("scratch/").unwrap_or(&path)
                        )
                    } else {
                        path.clone()
                    };
                    if let Err(e) = write_scratch(&self.root, &eff_path, &contents) {
                        let reason = format!("host.write failed: {e}");
                        let e2 = fail(&mut rec, &reason);
                        self.save(&rec)?;
                        return Err(e2);
                    }
                    let out = format!("wrote {eff_path} ({} bytes)", contents.len());
                    if shadow {
                        out
                    } else {
                        format!("wrote {path} ({} bytes)", contents.len())
                    }
                }
                APPEND_CAP => {
                    let contents = task_desc.clone();
                    let eff_path = if shadow {
                        format!(
                            "scratch/shadow/{}",
                            path.strip_prefix("scratch/").unwrap_or(&path)
                        )
                    } else {
                        path.clone()
                    };
                    if let Err(e) = append_scratch(&self.root, &eff_path, &contents) {
                        let reason = format!("host.append failed: {e}");
                        let e2 = fail(&mut rec, &reason);
                        self.save(&rec)?;
                        return Err(e2);
                    }
                    if shadow {
                        format!("appended {} bytes to {eff_path}", contents.len())
                    } else {
                        format!("appended {} bytes to {path}", contents.len())
                    }
                }
                READ_CAP => {
                    let dest = self.root.join(&path);
                    match fs::read_to_string(&dest) {
                        Ok(contents) => contents,
                        Err(e) => {
                            let reason = format!("host.read not found: {path} ({e})");
                            let e2 = fail(&mut rec, &reason);
                            self.save(&rec)?;
                            return Err(e2);
                        }
                    }
                }
                LIST_CAP => {
                    let dest = self.root.join(&path);
                    match fs::read_dir(&dest) {
                        Ok(entries) => {
                            let names: Vec<String> = entries
                                .filter_map(|e| e.ok())
                                .map(|e| e.file_name().to_string_lossy().to_string())
                                .collect();
                            if names.is_empty() {
                                "(empty)".to_string()
                            } else {
                                format!("entries: {}", names.join(", "))
                            }
                        }
                        Err(e) => {
                            let reason = format!("host.list not found: {path} ({e})");
                            let e2 = fail(&mut rec, &reason);
                            self.save(&rec)?;
                            return Err(e2);
                        }
                    }
                }
                _ => {
                    let e = fail(&mut rec, "unknown capability");
                    self.save(&rec)?;
                    return Err(e);
                }
            }
        };
        // Shadow: verify the dry-run's promise against the shadow reality;
        // the intent's status does not advance.
        if shadow {
            let predicted = rec
                .simulation
                .as_ref()
                .map(|s| s.effects_preview.clone())
                .unwrap_or_default();
            let actual = actual_shadow_effects(&cap, &task_desc, &task_path);
            let diff = verify_shadow(&predicted, &actual);
            Self::append(
                &mut rec,
                "ShadowVerified",
                serde_json::json!({
                    "shadow": true,
                    "match": diff.r#match,
                    "predicted": serde_json::to_value(&diff.predicted)?,
                    "actual": serde_json::to_value(&diff.actual)?,
                }),
            );
            Self::append(
                &mut rec,
                "TaskSucceeded",
                serde_json::json!({"shadow": true, "output": output}),
            );
            self.save(&rec)?;
            rec.output = Some(output);
            return Ok(rec);
        }
        rec.output = Some(output.clone());
        rec.status = IntentStatus::Executed;
        Self::append(
            &mut rec,
            "TaskSucceeded",
            serde_json::json!({"output": output}),
        );
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn replay(&self, id: &str) -> Result<(StoredIntent, FoldedState, bool)> {
        let rec = self.load(id)?;
        let folded = fold_events(&rec.events);
        let matches = folded.status.as_ref() == Some(&rec.status) && folded.output == rec.output;
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

fn host_verb(capability: &str) -> &'static str {
    match capability {
        WRITE_CAP => "write",
        READ_CAP => "read",
        APPEND_CAP => "append",
        LIST_CAP => "list",
        HTTP_CAP => "fetch",
        _ => "effect",
    }
}

/// The effects a shadow run actually produced (paths as executed, redirect
/// included — `verify_shadow` normalizes both sides).
fn actual_shadow_effects(cap: &str, contents: &str, path: &Option<String>) -> Vec<EffectPreview> {
    match (cap, path) {
        (ECHO_CAP, _) => vec![EffectPreview::Echo {
            text: contents.to_string(),
        }],
        (WRITE_CAP, Some(p)) => vec![EffectPreview::Write {
            path: format!("scratch/shadow/{}", p.strip_prefix("scratch/").unwrap_or(p)),
            bytes: contents.len() as u64,
        }],
        (APPEND_CAP, Some(p)) => vec![EffectPreview::Append {
            path: format!("scratch/shadow/{}", p.strip_prefix("scratch/").unwrap_or(p)),
            bytes: contents.len() as u64,
        }],
        (READ_CAP, Some(p)) => vec![EffectPreview::Read { path: p.clone() }],
        (LIST_CAP, Some(p)) => vec![EffectPreview::List { path: p.clone() }],
        _ => vec![],
    }
}

/// Injectable GET transport so tests never touch the network.
pub trait HttpTransport {
    /// Perform a GET; returns the body. Non-2xx → Err with a stable reason.
    fn get(&self, url: &str) -> Result<String>;
}

pub struct UreqTransport;

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str) -> Result<String> {
        match ureq::get(url).call() {
            Ok(resp) => Ok(resp.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(code, _)) => {
                bail!("http status {code}")
            }
            Err(e) => bail!("http transport error: {e}"),
        }
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

fn append_scratch(root: &Path, rel: &str, contents: &str) -> Result<()> {
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest)?;
    file.write_all(contents.as_bytes())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_home() -> V0Home {
        let root = std::env::temp_dir().join(format!("imperium-v0-{}", uuid::Uuid::new_v4()));
        let home = V0Home { root };
        home.init().unwrap();
        home
    }

    #[test]
    fn write_creates_scratch_file() {
        let home = tmp_home();
        let rec = home
            .compile("Write file notes.txt with contents hello-from-cli", false)
            .unwrap();
        let id = rec.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        let rec = home.execute(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        let path = home.root.join("scratch/notes.txt");
        assert_eq!(fs::read_to_string(path).unwrap(), "hello-from-cli");
    }

    #[test]
    fn write_escape_never_compiles() {
        let home = tmp_home();
        assert!(home
            .compile("Write file ../secret with contents x", false)
            .is_err());
        assert!(!home.root.join("secret").exists());
    }

    #[test]
    fn execute_without_approve_is_rejected() {
        let home = tmp_home();
        let rec = home.compile("Echo this message: ping", false).unwrap();
        let id = rec.ir.id.to_string();
        assert!(home.execute(&id).is_err());
    }

    #[test]
    fn low_risk_echo_auto_approves_on_execute() {
        let home = tmp_home();
        let rec = home.compile("Echo this message: ping", false).unwrap();
        let id = rec.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        let rec = home.execute(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        // Auto-approval is auditable: IntentApproved carries auto + risk.
        let approved = rec
            .events
            .iter()
            .find(|e| e.kind == "IntentApproved")
            .expect("auto approve event");
        assert_eq!(approved.payload["auto"], serde_json::json!(true));
        assert_eq!(approved.payload["risk"], serde_json::json!(0.0));
    }

    #[test]
    fn read_executes_and_returns_contents() {
        let home = tmp_home();
        let w = home
            .compile("Write file notes.txt with contents hello", false)
            .unwrap();
        let id = w.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        home.execute(&id).unwrap();
        let r = home.compile("Read file notes.txt", false).unwrap();
        let id = r.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        assert!(!r.ir.requires_approval);
        let rec = home.execute(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        assert_eq!(rec.output.as_deref(), Some("hello"));
    }

    #[test]
    fn read_sensitive_path_fails_closed() {
        let home = tmp_home();
        home.init().unwrap();
        fs::write(home.root.join("scratch/.env"), "TOP_SECRET=1").unwrap();
        let r = home.compile("Read file .env", false).unwrap();
        let id = r.ir.id.to_string();
        let rec = home.simulate_opts(&id, None).unwrap();
        // Dry-run already denies: success 0, un-approvable.
        assert_eq!(rec.simulation.as_ref().unwrap().success_probability, 0.0);
        assert!(home.approve(&id).is_err());
    }

    #[test]
    fn append_grows_file_without_truncating() {
        let home = tmp_home();
        let a = home
            .compile("Append file log.txt with contents one", false)
            .unwrap();
        let b = home
            .compile("Append file log.txt with contents two", false)
            .unwrap();
        for rec in [&a, &b] {
            let id = rec.ir.id.to_string();
            home.simulate_opts(&id, None).unwrap();
            home.approve(&id).unwrap();
            home.execute(&id).unwrap();
        }
        assert_eq!(
            fs::read_to_string(home.root.join("scratch/log.txt")).unwrap(),
            "onetwo"
        );
    }

    #[test]
    fn list_shows_scratch_entries() {
        let home = tmp_home();
        let w = home
            .compile("Write file notes_dir/alpha.txt with contents x", false)
            .unwrap();
        let id = w.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        home.execute(&id).unwrap();
        let l = home.compile("List files under notes_dir", false).unwrap();
        let id = l.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        let rec = home.execute(&id).unwrap();
        assert!(rec.output.unwrap().contains("alpha.txt"));
    }

    #[test]
    fn narrowed_grants_fail_closed_at_execute() {
        let home = tmp_home();
        home.init().unwrap();
        // Narrow cap.write to scratch/sub only.
        let mut grants: BTreeMap<String, Permissions> = default_grants();
        grants.insert(
            WRITE_CAP.into(),
            Permissions {
                fs: vec!["scratch/sub".into()],
                net: vec![],
                env: vec![],
            },
        );
        fs::write(
            home.root.join("grants.json"),
            serde_json::to_string_pretty(&GrantFile { grants }).unwrap(),
        )
        .unwrap();
        let w = home
            .compile("Write file notes.txt with contents hi", false)
            .unwrap();
        let id = w.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        let err = home.execute(&id).unwrap_err().to_string();
        assert!(err.contains("host.write path denied"), "{err}");
    }

    #[test]
    fn grants_exceeding_defaults_are_rejected() {
        let home = tmp_home();
        home.init().unwrap();
        let mut grants: BTreeMap<String, Permissions> = default_grants();
        grants.insert(
            READ_CAP.into(),
            Permissions {
                fs: vec!["scratch".into(), "outside".into()],
                net: vec![],
                env: vec![],
            },
        );
        fs::write(
            home.root.join("grants.json"),
            serde_json::to_string_pretty(&GrantFile { grants }).unwrap(),
        )
        .unwrap();
        assert!(home.load_grants().is_err());
    }

    #[test]
    fn grants_with_unknown_capability_are_rejected() {
        let home = tmp_home();
        home.init().unwrap();
        let mut grants: BTreeMap<String, Permissions> = default_grants();
        grants.insert(
            "cap.shell".into(),
            Permissions {
                fs: vec![],
                net: vec![],
                env: vec![],
            },
        );
        fs::write(
            home.root.join("grants.json"),
            serde_json::to_string_pretty(&GrantFile { grants }).unwrap(),
        )
        .unwrap();
        assert!(home.load_grants().is_err());
    }

    fn write_policy(home: &V0Home, text: &str) {
        fs::write(home.root.join("policy.imp"), text).unwrap();
    }

    struct FakeTransport(&'static str);
    impl HttpTransport for FakeTransport {
        fn get(&self, _url: &str) -> Result<String> {
            Ok(self.0.into())
        }
    }

    struct FailTransport;
    impl HttpTransport for FailTransport {
        fn get(&self, _url: &str) -> Result<String> {
            bail!("http status 500")
        }
    }

    /// Declare cap.http net hosts in grants.json (within the `*` ceiling).
    fn allow_fetch_host(home: &V0Home, host: &str) {
        let mut grants: BTreeMap<String, Permissions> = initial_grants();
        grants.insert(
            HTTP_CAP.into(),
            Permissions {
                fs: vec![],
                net: vec![host.into()],
                env: vec![],
            },
        );
        fs::write(
            home.root.join("grants.json"),
            serde_json::to_string_pretty(&GrantFile { grants }).unwrap(),
        )
        .unwrap();
    }

    fn compile_fetch(home: &V0Home, url: &str) -> String {
        home.compile(&format!("Fetch {url}"), false)
            .unwrap()
            .ir
            .id
            .to_string()
    }

    #[test]
    fn fetch_simulate_denied_without_policy() {
        let home = tmp_home();
        let id = compile_fetch(&home, "https://api.example.com/x");
        let rec = home.simulate_opts(&id, None).unwrap();
        let sim = rec.simulation.unwrap();
        assert_eq!(sim.success_probability, 0.0);
        assert!(matches!(
            &sim.effects_preview[0],
            EffectPreview::Denied { reason, .. } if reason == FETCH_NO_ALLOW_RULE
        ));
        assert!(home.approve(&id).is_err());
    }

    #[test]
    fn fetch_end_to_end_with_fake_transport() {
        let home = tmp_home();
        allow_fetch_host(&home, "api.example.com");
        write_policy(&home, "allow fetch to api.example.com\n");
        let id = compile_fetch(&home, "https://api.example.com/x");
        home.simulate_opts(&id, None).unwrap();
        // High risk: never auto-approves.
        assert!(home
            .execute_with_transport(&id, &FakeTransport("body"))
            .is_err());
        home.approve(&id).unwrap();
        let rec = home
            .execute_with_transport(&id, &FakeTransport("hello-net"))
            .unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        assert_eq!(rec.output.as_deref(), Some("hello-net"));
        let ev = rec
            .events
            .iter()
            .find(|e| e.kind == "PolicyEvaluated")
            .expect("audited fetch decision");
        assert_eq!(ev.payload["verb"], serde_json::json!("fetch"));
        assert_eq!(ev.payload["decision"], serde_json::json!("allow"));
        assert_eq!(ev.payload["path"], serde_json::json!("api.example.com"));
    }

    #[test]
    fn fetch_denied_at_execute_when_policy_tightens() {
        let home = tmp_home();
        allow_fetch_host(&home, "api.example.com");
        // Simulate + approve while allowed...
        write_policy(&home, "allow fetch to api.example.com\n");
        let id = compile_fetch(&home, "https://api.example.com/x");
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        // ...then deny: execution-time enforcement is independent.
        write_policy(&home, "deny fetch matching *.example.com\n");
        assert!(home
            .execute_with_transport(&id, &FakeTransport("x"))
            .is_err());
        let rec = home.load(&id).unwrap();
        let ev = rec
            .events
            .iter()
            .find(|e| e.kind == "PolicyEvaluated")
            .expect("audited fetch decision");
        assert_eq!(ev.payload["decision"], serde_json::json!("deny"));
    }

    #[test]
    fn fetch_requires_declared_grant_host() {
        let home = tmp_home();
        // Grants declare a different host: token has no net → host denied.
        allow_fetch_host(&home, "other.example.com");
        write_policy(&home, "allow fetch to api.example.com\n");
        let id = compile_fetch(&home, "https://api.example.com/x");
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        let err = home
            .execute_with_transport(&id, &FakeTransport("x"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("host.fetch host denied"), "{err}");
    }

    #[test]
    fn fetch_transport_failure_fails_the_task() {
        let home = tmp_home();
        allow_fetch_host(&home, "api.example.com");
        write_policy(&home, "allow fetch to api.example.com\n");
        let id = compile_fetch(&home, "https://api.example.com/x");
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        let err = home
            .execute_with_transport(&id, &FailTransport)
            .unwrap_err()
            .to_string();
        assert!(err.contains("http status 500"), "{err}");
        let rec = home.load(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Failed);
    }

    #[test]
    fn fetch_rejects_bad_urls_at_compile() {
        let home = tmp_home();
        for url in [
            "http://api.example.com/x",
            "https://localhost/x",
            "https://127.0.0.1/x",
            "https://api.example.com:8443/x",
        ] {
            assert!(
                home.compile(&format!("Fetch {url}"), false).is_err(),
                "should not compile: {url}"
            );
        }
    }

    #[test]
    fn capabilities_register_approve_list() {
        let home = tmp_home();
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "servers": [{"url": "https://api.example.com"}],
            "paths": {"/v1/things": {"get": {"operationId": "list_things"}}}
        })
        .to_string();
        let m = home.add_capability("example_api", &spec).unwrap();
        assert_eq!(m.hosts, vec!["api.example.com"]);
        assert!(!m.approved);
        // Duplicate registration is refused.
        assert!(home.add_capability("example_api", &spec).is_err());
        let listed = home.list_capabilities().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(!listed[0].1.approved);
        let m = home.approve_capability("example_api").unwrap();
        assert!(m.approved);
        assert!(m.approved_at.is_some());
        assert!(home.approve_capability("example_api").is_err());
        assert!(home.approve_capability("nope").is_err());
        let listed = home.list_capabilities().unwrap();
        assert!(listed[0].1.approved);
    }

    // --- Phase 12: Monte Carlo ---

    fn run_write(home: &V0Home, file: &str) {
        let rec = home
            .compile(&format!("Write file {file} with contents x"), false)
            .unwrap();
        let id = rec.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        home.execute(&id).unwrap();
    }

    #[test]
    fn events_carry_timestamps_for_world_model() {
        let home = tmp_home();
        run_write(&home, "a.txt");
        let rec = &home.list().unwrap()[0];
        let started = rec.events.iter().find(|e| e.kind == "TaskStarted").unwrap();
        let succeeded = rec
            .events
            .iter()
            .find(|e| e.kind == "TaskSucceeded")
            .unwrap();
        assert!(started.at.is_some());
        assert!(succeeded.at.unwrap() >= started.at.unwrap());
        let stats = home.ledger().unwrap().world_stats().unwrap();
        assert_eq!(stats[WRITE_CAP].samples, 1);
        assert_eq!(stats[WRITE_CAP].successes, 1);
        assert_eq!(stats[WRITE_CAP].durations_ms.len(), 1);
    }

    #[test]
    fn monte_carlo_simulate_is_deterministic_and_gated() {
        let home = tmp_home();
        // History: two clean executions of cap.write.
        run_write(&home, "a.txt");
        run_write(&home, "b.txt");
        let rec = home
            .compile("Write file c.txt with contents x", false)
            .unwrap();
        let id = rec.ir.id.to_string();
        let a = home.simulate_opts(&id, Some((1000, Some(11)))).unwrap();
        let b = home.simulate_opts(&id, Some((1000, Some(11)))).unwrap();
        let sa = a.simulation.clone().unwrap();
        let sb = b.simulation.clone().unwrap();
        assert!(sa.probabilistic);
        assert_eq!(sa.seed, 11);
        assert_eq!(sa.mc_successes, sb.mc_successes);
        assert_eq!(sa.p50_ms, sb.p50_ms);
        // p_attempt = (2+1)/(2+2) = 0.75; 3 attempts → ≈0.984 > 0.9 → approvable.
        assert!(sa.p_success > 0.95, "p={}", sa.p_success);
        home.approve(&id).unwrap();
        home.execute(&id).unwrap();
        // Fresh capability (no history): p ≈ 0.875 < 0.9 → gate blocks approval.
        let rec = home
            .compile("Write file d.txt with contents x", false)
            .unwrap();
        let fresh = rec.ir.id.to_string();
        // Write to a fresh capability: use cap.read (no history in this home).
        home.simulate_opts(&fresh, Some((1000, Some(3)))).unwrap();
        let rec = home.compile("Read file c.txt", false).unwrap();
        let read_id = rec.ir.id.to_string();
        home.simulate_opts(&read_id, Some((1000, Some(3)))).unwrap();
        assert!(home.approve(&read_id).is_err());
    }

    // --- Phase 13: the evolution loop ---

    #[test]
    fn friction_key_is_slot_aware() {
        let home = tmp_home();
        // Three intents differing only in content — the old (Phase 10) key
        // would never have flagged these.
        home.compile("Echo this message: alpha", false).unwrap();
        home.compile("Echo this message: beta", false).unwrap();
        let rec = home.compile("Echo this message: gamma", false).unwrap();
        let friction = rec
            .events
            .iter()
            .find(|e| e.kind == "FrictionDetected")
            .expect("slot-aware friction on third differing intent");
        assert_eq!(friction.payload["count"], serde_json::json!(3));
    }

    #[test]
    fn form_save_run_and_gauntlet() {
        let home = tmp_home();
        let seed = home.compile("Echo this message: ping", false).unwrap();
        let t = home
            .save_form(&seed.ir.id.to_string(), "ping_form")
            .unwrap();
        assert_eq!(t.template, "Echo this message: {{slot}}");
        assert!(home
            .save_form(&seed.ir.id.to_string(), "ping_form")
            .is_err());
        // Run substitutes, compiles, simulates — and stops for approval.
        let rec = home.run_form("ping_form", "hello world", None).unwrap();
        assert_eq!(rec.status, IntentStatus::Simulated);
        assert_eq!(rec.ir.tasks[0].description, "hello world");
        // The gauntlet: echo is low-risk, so execute auto-approves (audited)
        // — a high-risk verb would demand explicit approval here.
        let id = rec.ir.id.to_string();
        let rec = home.execute(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        let approved = rec
            .events
            .iter()
            .find(|e| e.kind == "IntentApproved")
            .expect("auto-approve event");
        assert_eq!(approved.payload["auto"], serde_json::json!(true));
        // Usage is a view over the ledger.
        let (runs, verified) = home.form_usage("ping_form").unwrap();
        assert_eq!(runs, 1);
        assert_eq!(verified, 0);
        assert_eq!(home.list_forms().unwrap(), vec!["ping_form".to_string()]);
    }

    #[test]
    fn shadow_run_verifies_and_keeps_status() {
        let home = tmp_home();
        let rec = home
            .compile("Write file notes.txt with contents hello", false)
            .unwrap();
        let id = rec.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        let rec = home.execute_shadow(&id).unwrap();
        // Status did not advance; the real file was never touched.
        assert_eq!(rec.status, IntentStatus::Approved);
        assert!(rec.output.unwrap().contains("scratch/shadow/notes.txt"));
        assert!(!home.root.join("scratch/notes.txt").exists());
        assert!(home.root.join("scratch/shadow/notes.txt").exists());
        let sv = rec
            .events
            .iter()
            .find(|e| e.kind == "ShadowVerified")
            .expect("ShadowVerified event");
        assert_eq!(sv.payload["match"], serde_json::json!(true));
        // The fold still agrees with the store (shadow is provenance).
        let (_, folded, matches) = home.replay(&id).unwrap();
        assert_eq!(folded.status, Some(IntentStatus::Approved));
        assert!(matches);
        // Real execution afterwards needs a fresh approval (one-shot token).
        assert!(home.execute(&id).is_err());
        home.approve(&id).unwrap();
        let rec = home.execute(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        assert!(home.root.join("scratch/notes.txt").exists());
    }

    #[test]
    fn shadow_mismatch_and_fetch_refusal() {
        let home = tmp_home();
        // A mismatch: the intent promises one path but the simulation was
        // replaced by hand to point elsewhere.
        let rec = home
            .compile("Write file notes.txt with contents hi", false)
            .unwrap();
        let id = rec.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        // Corrupt the stored preview so the promise no longer matches.
        let mut rec = home.load(&id).unwrap();
        rec.simulation.as_mut().unwrap().effects_preview = vec![EffectPreview::Write {
            path: "scratch/other.txt".into(),
            bytes: 2,
        }];
        home.save(&rec).unwrap();
        home.approve(&id).unwrap();
        let rec = home.execute_shadow(&id).unwrap();
        let sv = rec
            .events
            .iter()
            .find(|e| e.kind == "ShadowVerified")
            .unwrap();
        assert_eq!(sv.payload["match"], serde_json::json!(false));
        // Fetch is refused in shadow mode.
        allow_fetch_host(&home, "api.example.com");
        write_policy(&home, "allow fetch to api.example.com\n");
        let fetch_id = compile_fetch(&home, "https://api.example.com/x");
        home.simulate_opts(&fetch_id, None).unwrap();
        home.approve(&fetch_id).unwrap();
        let err = home.execute_shadow(&fetch_id).unwrap_err().to_string();
        assert!(err.contains("fetch cannot run in shadow mode"), "{err}");
    }

    #[test]
    fn verified_badge_requires_three_matching_shadow_runs() {
        let home = tmp_home();
        let seed = home
            .compile("Write file log.txt with contents x", false)
            .unwrap();
        home.save_form(&seed.ir.id.to_string(), "logform").unwrap();
        for i in 0..3 {
            let rec = home.run_form("logform", &format!("v{i}"), None).unwrap();
            let id = rec.ir.id.to_string();
            home.approve(&id).unwrap();
            home.execute_shadow(&id).unwrap();
            // A form run's shadow leaves status Approved; approve again for
            // the next round (fresh intent each time, so no conflict).
        }
        let (runs, verified) = home.form_usage("logform").unwrap();
        assert_eq!(runs, 3);
        assert_eq!(verified, 3);
    }

    #[test]
    fn policy_deny_blocks_simulation_and_approval() {
        let home = tmp_home();
        write_policy(&home, "deny read matching scratch/notes*\n");
        let r = home.compile("Read file notes.txt", false).unwrap();
        let id = r.ir.id.to_string();
        let rec = home.simulate_opts(&id, None).unwrap();
        let sim = rec.simulation.unwrap();
        assert_eq!(sim.success_probability, 0.0);
        assert!(matches!(
            &sim.effects_preview[0],
            EffectPreview::Denied { reason, .. } if reason.contains("policy: deny read matching scratch/notes*")
        ));
        assert!(home.approve(&id).is_err());
    }

    #[test]
    fn policy_allow_path_executes_with_audited_decision() {
        let home = tmp_home();
        write_policy(&home, "deny read matching scratch/notes*\n");
        fs::create_dir_all(home.root.join("scratch")).unwrap();
        fs::write(home.root.join("scratch/other.txt"), "fine").unwrap();
        let r = home.compile("Read file other.txt", false).unwrap();
        let id = r.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        let rec = home.execute(&id).unwrap();
        assert_eq!(rec.status, IntentStatus::Executed);
        let ev = rec
            .events
            .iter()
            .find(|e| e.kind == "PolicyEvaluated")
            .expect("PolicyEvaluated event");
        assert_eq!(ev.payload["verb"], serde_json::json!("read"));
        assert_eq!(ev.payload["decision"], serde_json::json!("allow"));
        assert_eq!(ev.payload["rule"], serde_json::json!(null));
    }

    #[test]
    fn policy_deny_at_execute_records_deny_decision() {
        let home = tmp_home();
        fs::create_dir_all(home.root.join("scratch")).unwrap();
        fs::write(home.root.join("scratch/ok.txt"), "x").unwrap();
        // Simulate + approve under a permissive policy...
        write_policy(&home, "deny read matching *.zzz\n");
        let r = home.compile("Read file ok.txt", false).unwrap();
        let id = r.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        home.approve(&id).unwrap();
        // ...then tighten it: execution-time enforcement is independent.
        write_policy(&home, "deny read matching *.txt\n");
        assert!(home.execute(&id).is_err());
        let rec = home.load(&id).unwrap();
        let ev = rec
            .events
            .iter()
            .find(|e| e.kind == "PolicyEvaluated")
            .expect("PolicyEvaluated event");
        assert_eq!(ev.payload["decision"], serde_json::json!("deny"));
        assert_eq!(
            ev.payload["reason"],
            serde_json::json!(Some("policy: deny read matching *.txt"))
        );
    }

    #[test]
    fn policy_parse_error_fails_closed() {
        let home = tmp_home();
        write_policy(&home, "allow everything\n");
        let r = home.compile("Echo this message: ping", false).unwrap();
        let id = r.ir.id.to_string();
        assert!(home.simulate_opts(&id, None).is_err());
    }

    #[test]
    fn policy_require_approval_blocks_auto_approve() {
        let home = tmp_home();
        write_policy(&home, "require approval read\n");
        fs::create_dir_all(home.root.join("scratch")).unwrap();
        fs::write(home.root.join("scratch/a.txt"), "x").unwrap();
        let r = home.compile("Read file a.txt", false).unwrap();
        let id = r.ir.id.to_string();
        home.simulate_opts(&id, None).unwrap();
        assert!(home.execute(&id).is_err());
        let rec = home.approve(&id).unwrap();
        let approved = rec
            .events
            .iter()
            .find(|e| e.kind == "IntentApproved")
            .unwrap();
        assert_eq!(approved.payload["policy_approval"], serde_json::json!(true));
        assert!(home.execute(&id).is_ok());
    }

    #[test]
    fn policy_content_deny_blocks_proposal() {
        let home = tmp_home();
        write_policy(&home, "deny containing gotcha\n");
        assert!(home
            .compile("say gotcha hello", true)
            .unwrap_err()
            .to_string()
            .contains("Policy denied proposal: deny containing gotcha"));
        // Non-propose canonical compiles are unaffected (built-ins still are).
        assert!(home.compile("Echo this message: hi", false).is_ok());
    }

    #[test]
    fn ledger_syncs_on_save_and_search_finds() {
        let home = tmp_home();
        let rec = home
            .compile("Write file notes.txt with contents hello", false)
            .unwrap();
        let hits = home.ledger().unwrap().search("hello").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, rec.ir.id.to_string());
        // The fold is still the truth: rebuild reproduces the same result.
        let ledger = home.ledger().unwrap();
        assert!(ledger.rebuild(&home.root.join("intents")).unwrap() >= 1);
        assert_eq!(home.ledger().unwrap().search("hello").unwrap().len(), 1);
    }

    #[test]
    fn ledger_rebuild_restores_after_deletion() {
        let home = tmp_home();
        let rec = home
            .compile("Write file notes.txt with contents hello", false)
            .unwrap();
        assert!(home.ledger().unwrap().search("hello").unwrap().len() == 1);
        // Destroy the projection entirely.
        fs::remove_file(home.root.join("ledger.db")).unwrap();
        assert!(home.ledger().unwrap().search("hello").unwrap().is_empty());
        let ledger = home.ledger().unwrap();
        let n = ledger.rebuild(&home.root.join("intents")).unwrap();
        assert_eq!(n, 1);
        let hits = home.ledger().unwrap().search("hello").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, rec.ir.id.to_string());
    }

    #[test]
    fn friction_detected_on_third_identical_intent() {
        let home = tmp_home();
        home.compile("Echo this message: ping", false).unwrap();
        home.compile("say ping", true).unwrap(); // same semantic shape
        let rec = home.compile("Echo this message: ping", false).unwrap();
        let friction = rec
            .events
            .iter()
            .find(|e| e.kind == "FrictionDetected")
            .expect("friction event on third identical compile");
        assert_eq!(friction.payload["count"], serde_json::json!(3));
        assert!(friction.payload["suggestion"]
            .as_str()
            .unwrap()
            .contains("reusable form"));
        // First and second compiles carry no friction event.
        let first = home
            .list()
            .unwrap()
            .into_iter()
            .find(|r| r.events.len() == 1 && r.events[0].kind == "IntentCompiled")
            .unwrap();
        assert!(!first.events.iter().any(|e| e.kind == "FrictionDetected"));
    }

    #[test]
    fn stats_counts_statuses_and_denials() {
        let home = tmp_home();
        let run = |home: &V0Home, nl: &str| -> String {
            let rec = home.compile(nl, false).unwrap();
            let id = rec.ir.id.to_string();
            home.simulate_opts(&id, None).unwrap();
            home.approve(&id).unwrap();
            id
        };
        // One intent executes cleanly.
        let a = run(&home, "Write file a.txt with contents one");
        home.execute(&a).unwrap();
        // Another is revoked before execution → TaskFailed("revoked").
        let b = run(&home, "Write file b.txt with contents two");
        home.revoke(&b).unwrap();
        assert!(home.execute(&b).is_err());
        let stats = home.ledger().unwrap().stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.executed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.top_denials[0].0, "revoked");
        assert_eq!(stats.top_denials[0].1, 1);
    }
}
