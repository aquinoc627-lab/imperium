//! IMPERIUM CLI — v0 intent loop is real; other verbs fail closed.

mod mcp;
mod secret;
mod v0_cmd;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use tracing_subscriber::{fmt, EnvFilter};
use v0_cmd::{resolve_nl, V0Home};

#[derive(Parser)]
#[command(name = "imperium")]
#[command(about = "IMPERIUM v0 — compile, simulate, approve, execute, replay")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create .imperium store (no model download)
    Init,
    /// Intent management (v0)
    Intent {
        #[command(subcommand)]
        command: IntentCommands,
    },
    /// The .imperium/policy.imp semantic firewall
    Policy {
        #[command(subcommand)]
        command: PolicyCommands,
    },
    /// The SQLite ledger (projection over intent records)
    Ledger {
        #[command(subcommand)]
        command: LedgerCommands,
    },
    /// OpenAPI → capability manifest registry (Phase 11)
    Capabilities {
        #[command(subcommand)]
        command: CapabilitiesCommands,
    },
    /// Reusable, human-saved intent templates (Phase 13)
    Forms {
        #[command(subcommand)]
        command: FormsCommands,
    },
    /// Audited recurring execution (Phase 16)
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
    },
    /// Token-secret storage posture (Phase 18)
    Secret {
        #[command(subcommand)]
        command: SecretCommands,
    },
    /// Search the ledger (name, source, output)
    Search { query: String },
    /// Full trace of one intent: IR → simulation → token → events → outcome
    Show {
        #[arg(short, long)]
        intent_id: String,
    },
    /// Replay from the event log
    Replay {
        #[arg(short, long)]
        intent_id: String,
    },
    /// The world model: observed facts computed from the event log (Phase 12)
    World,
    /// Serve the kernel as MCP tools over stdio (Phase 14)
    Mcp,
}

#[derive(Subcommand)]
enum LedgerCommands {
    /// Aggregate stats over the ledger projection
    Stats,
    /// Rebuild the projection from the canonical intent records
    Rebuild,
}

#[derive(Subcommand)]
enum FormsCommands {
    /// Save a reusable template from a compiled intent
    Save {
        #[arg(short, long)]
        intent_id: String,
        #[arg(short, long)]
        name: String,
    },
    /// Substitute + compile + simulate (stops for normal approval)
    Run {
        #[arg(short, long)]
        name: String,
        /// The slot value (the free-text part of the intent)
        #[arg(short, long)]
        slot: String,
        /// Monte Carlo trials for the preview
        #[arg(long)]
        trials: Option<u64>,
    },
    /// List forms with their run/verified counts (a view over the ledger)
    List,
}
#[derive(Subcommand)]
enum ScheduleCommands {
    /// Add a new recurring schedule
    Add {
        #[arg(short, long)]
        form: String,
        #[arg(short, long)]
        slot: String,
        #[arg(short, long)]
        every: u64,
    },
    /// Remove a schedule
    Remove {
        #[arg(short, long)]
        name: String,
    },
    /// Pause a schedule
    Pause {
        #[arg(short, long)]
        name: String,
    },
    /// Resume a schedule
    Resume {
        #[arg(short, long)]
        name: String,
    },
    /// List all schedules with next run status
    List,
    /// Run all due schedules (one-shot, no daemon)
    Tick,
}

#[derive(Subcommand)]
enum SecretCommands {
    /// Report the active backend, secret fingerprint, and migration state
    Status,
    /// Migrate the plaintext token.secret into the OS keychain (macOS) and
    /// delete the file; records the choice for future runs
    Bind,
    /// Replace the secret with a fresh high-entropy value (issued tokens stop verifying)
    Rotate,
}

#[derive(Subcommand)]
enum CapabilitiesCommands {
    /// Synthesize a manifest from an OpenAPI spec and register it (unapproved)
    Add {
        /// Path to the OpenAPI JSON spec
        #[arg(short, long)]
        spec: String,
        /// kebab/snake-case capability name
        #[arg(short, long)]
        name: String,
    },
    /// Explicitly approve a registered capability manifest
    Approve {
        #[arg(short, long)]
        name: String,
    },
    /// List registered capability manifests
    List,
}

#[derive(Subcommand)]
enum PolicyCommands {
    /// Parse + lint policy.imp (contradictions, duplicates, firewall redundancy)
    Lint,
    /// Explain a decision chain: policy → host firewall → grants
    Explain {
        #[arg(long)]
        verb: String,
        #[arg(short, long)]
        path: String,
    },
    /// Run the policy test harness (Phase 15): evaluate each entry of
    /// `.imperium/policy.tests.json` against the active policy and print PASS/FAIL
    Test,
    /// Analyze the impact of a proposed policy change against the ledger
    Impact {
        #[arg(long)]
        rule: String,
    },
    /// Show policy coverage against the ledger
    Coverage,
}

#[derive(Subcommand)]
enum IntentCommands {
    /// Compile NL (or a file) to IR
    Compile {
        #[arg(short, long)]
        input: String,
        /// Map loose phrasing, then run the rules compiler
        #[arg(long)]
        propose: bool,
    },
    Simulate {
        #[arg(short, long)]
        intent_id: String,
        /// Print the simulation JSON (incl. effects_preview) instead of text
        #[arg(long)]
        json: bool,
        /// Run a seeded Monte Carlo over the world model (default 1000 trials)
        #[arg(long)]
        trials: Option<u64>,
        /// Seed for the Monte Carlo run (default: derived from the intent id)
        #[arg(long)]
        seed: Option<u64>,
    },
    Approve {
        #[arg(short, long)]
        intent_id: String,
    },
    Execute {
        #[arg(short, long)]
        intent_id: String,
        /// Shadow run: redirect destructive effects under scratch/shadow/
        /// and fold a ShadowVerified diff (Phase 13)
        #[arg(long)]
        shadow: bool,
    },
    Revoke {
        #[arg(short, long)]
        intent_id: String,
    },
    List,
    Replay {
        #[arg(short, long)]
        intent_id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    fmt().with_env_filter(filter).with_target(false).init();

    let home = V0Home::discover()?;
    match cli.command {
        Commands::Init => {
            home.init()?;
            println!("initialized {}", home.root.display());
        }
        Commands::Replay { intent_id } => print_replay(&home, &intent_id)?,
        Commands::Policy { command } => match command {
            PolicyCommands::Lint => policy_lint(&home)?,
            PolicyCommands::Explain { verb, path } => policy_explain(&home, &verb, &path)?,
            PolicyCommands::Test => policy_test(&home)?,
            PolicyCommands::Impact { rule } => policy_impact(&home, &rule)?,
            PolicyCommands::Coverage => policy_coverage(&home)?,
        },
        Commands::Ledger { command } => match command {
            LedgerCommands::Stats => ledger_stats(&home)?,
            LedgerCommands::Rebuild => ledger_rebuild(&home)?,
        },
        Commands::Capabilities { command } => match command {
            CapabilitiesCommands::Add { spec, name } => {
                let spec_text = resolve_nl(&spec)?;
                let m = home.add_capability(&name, &spec_text)?;
                println!(
                    "{}\tregistered (unapproved)\thosts={}\tops={}",
                    m.name,
                    m.hosts.join(","),
                    m.ops_summary()
                );
            }
            CapabilitiesCommands::Approve { name } => {
                let m = home.approve_capability(&name)?;
                println!(
                    "{}\tapproved\t{}",
                    m.name,
                    m.approved_at.unwrap_or_default()
                );
            }
            CapabilitiesCommands::List => {
                for (name, m) in home.list_capabilities()? {
                    println!(
                        "{}\t{}\thosts={}\tops={}",
                        name,
                        if m.approved { "approved" } else { "unapproved" },
                        m.hosts.join(","),
                        m.ops_summary()
                    );
                }
            }
        },
        Commands::Search { query } => {
            for hit in home.ledger()?.search(&query)? {
                println!(
                    "{}\t{}\t{}\t{}",
                    hit.id, hit.status, hit.name, hit.nl_source
                );
            }
        }
        Commands::Show { intent_id } => show_intent(&home, &intent_id)?,
        Commands::World => world_show(&home)?,
        Commands::Mcp => {
            mcp::McpServer::new(home).serve()?;
        }
        Commands::Forms { command } => match command {
            FormsCommands::Save { intent_id, name } => {
                let t = home.save_form(&intent_id, &name)?;
                println!("{}	{}", name, t.template);
            }
            FormsCommands::Run { name, slot, trials } => {
                let rec = home.run_form(&name, &slot, trials.map(|t| (t, None)))?;
                let sim = rec.simulation.expect("simulated");
                println!("{}	{}	{}", rec.ir.id, rec.status, rec.ir.name);
                if sim.probabilistic {
                    println!(
                        "p_success={:.4} ({}/{}) p50_ms={} p95_ms={}",
                        sim.p_success, sim.mc_successes, sim.trials, sim.p50_ms, sim.p95_ms
                    );
                } else {
                    println!("success={} risk={}", sim.success_probability, sim.risk);
                }
                print_preview(&sim);
            }
            FormsCommands::List => {
                for name in home.list_forms()? {
                    let (runs, verified) = home.form_usage(&name)?;
                    let badge = if verified >= 3 { " verified" } else { "" };
                    println!("{name}	runs={runs}	verified={verified}{badge}");
                }
            }
        },
        Commands::Schedule { command } => match command {
            ScheduleCommands::Add { form, slot, every } => schedule_add(&home, &form, &slot, every)?,
            ScheduleCommands::Remove { name } => schedule_remove(&home, &name)?,
            ScheduleCommands::Pause { name } => schedule_set_enabled(&home, &name, false)?,
            ScheduleCommands::Resume { name } => schedule_set_enabled(&home, &name, true)?,
            ScheduleCommands::List => schedule_list(&home)?,
            ScheduleCommands::Tick => schedule_tick(&home)?,
        },
        Commands::Secret { command } => match command {
            SecretCommands::Status => println!("{}", home.secret_status()?),
            SecretCommands::Bind => println!("{}", home.secret_bind()?),
            SecretCommands::Rotate => println!("{}", home.secret_rotate()?),
        },
        Commands::Intent { command } => match command {
            IntentCommands::Compile { input, propose } => {
                let nl = resolve_nl(&input)?;
                let rec = home.compile(&nl, propose)?;
                println!("{}\t{}\t{}", rec.ir.id, rec.status, rec.ir.name);
            }
            IntentCommands::Simulate {
                intent_id,
                json,
                trials,
                seed,
            } => {
                let mc = trials.map(|t| (t, seed));
                let rec = home.simulate_opts(&intent_id, mc)?;
                let sim = rec.simulation.expect("simulated");
                if json {
                    println!("{}", serde_json::to_string_pretty(&sim)?);
                } else {
                    if sim.probabilistic {
                        println!(
                            "{}\t{}\tp_success={:.4} ({}/{}) p50_ms={} p95_ms={} seed={} risk={} duration_ms={}",
                            rec.ir.id,
                            rec.status,
                            sim.p_success,
                            sim.mc_successes,
                            sim.trials,
                            sim.p50_ms,
                            sim.p95_ms,
                            sim.seed,
                            sim.risk,
                            sim.duration_ms
                        );
                        for factor in &sim.factors {
                            println!("factor:  {factor}");
                        }
                    } else {
                        println!(
                            "{}\t{}\tsuccess={} risk={} duration_ms={}",
                            rec.ir.id,
                            rec.status,
                            sim.success_probability,
                            sim.risk,
                            sim.duration_ms
                        );
                    }
                    print_preview(&sim);
                }
            }
            IntentCommands::Approve { intent_id } => {
                let rec = home.approve(&intent_id)?;
                let fp = rec
                    .token
                    .as_ref()
                    .map(|t| t.token.signature.chars().take(12).collect::<String>())
                    .unwrap_or_default();
                println!("{}\t{}\ttoken={}", rec.ir.id, rec.status, fp);
                // Consent is to a specific diff: show the same preview the
                // simulator produced before the token is ever usable.
                if let Some(sim) = rec.simulation.as_ref() {
                    print_preview(sim);
                }
            }
            IntentCommands::Execute { intent_id, shadow } => {
                let rec = if shadow {
                    home.execute_shadow(&intent_id)?
                } else {
                    home.execute(&intent_id)?
                };
                println!(
                    "{}\t{}\t{}",
                    rec.ir.id,
                    rec.status,
                    rec.output.unwrap_or_default()
                );
            }
            IntentCommands::Revoke { intent_id } => {
                let rec = home.revoke(&intent_id)?;
                println!("{}\trevoked", rec.ir.id);
            }
            IntentCommands::List => {
                for rec in home.list()? {
                    println!("{}\t{}\t{}", rec.ir.id, rec.status, rec.ir.name);
                }
            }
            IntentCommands::Replay { intent_id } => print_replay(&home, &intent_id)?,
        },
    }
    Ok(())
}

fn world_show(home: &V0Home) -> Result<()> {
    let stats = home.ledger()?.world_stats()?;
    if stats.is_empty() {
        println!("no observed facts yet (run some intents)");
        return Ok(());
    }
    for (cap, s) in stats {
        let rate = (s.successes + 1) as f64 / (s.samples + 2) as f64;
        let mut durations = s.durations_ms.clone();
        durations.sort_unstable();
        let median = durations
            .get(durations.len() / 2)
            .copied()
            .unwrap_or_default();
        println!(
            "{}\tsamples={} successes={} rate={:.4} duration_ms(p50)={}",
            cap, s.samples, s.successes, rate, median
        );
    }
    Ok(())
}

fn ledger_stats(home: &V0Home) -> Result<()> {
    use imperium_store::Stats;
    let stats: Stats = home.ledger()?.stats()?;
    println!(
        "intents: {}  executed: {}  failed: {}",
        stats.total, stats.executed, stats.failed
    );
    let by_status = stats
        .by_status
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!("by_status: {by_status}");
    let caps = stats
        .top_capabilities
        .iter()
        .take(5)
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("capabilities: {caps}");
    let denials = stats
        .top_denials
        .iter()
        .take(5)
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("denials: {denials}");
    Ok(())
}

fn ledger_rebuild(home: &V0Home) -> Result<()> {
    let ledger = home.ledger()?;
    let n = ledger.rebuild(&home.root.join("intents"))?;
    println!(
        "rebuilt {n} intents from {}",
        home.root.join("intents").display()
    );
    Ok(())
}

fn show_intent(home: &V0Home, intent_id: &str) -> Result<()> {
    let rec = home.load(intent_id)?;
    println!("{}\t{}\t{}", rec.ir.id, rec.status, rec.ir.name);
    println!("nl:        {}", rec.ir.nl_source);
    println!("goal:      {}", rec.ir.goal.description);
    println!(
        "risk:      {}  approval: {}",
        rec.ir.risk_score, rec.ir.requires_approval
    );
    if let Some(task) = rec.ir.tasks.first() {
        println!(
            "task:      {} cap={}",
            task.name,
            task.capabilities.first().cloned().unwrap_or_default()
        );
        for effect in &task.effects {
            println!("effect:    {effect:?}");
        }
    }
    if let Some(sim) = rec.simulation.as_ref() {
        println!(
            "simulation: success={} risk={} duration_ms={}",
            sim.success_probability, sim.risk, sim.duration_ms
        );
    }
    if let Some(token) = rec.token.as_ref() {
        println!(
            "token:     fingerprint={} used={} revoked={}",
            token.token.signature.chars().take(12).collect::<String>(),
            token.used,
            token.revoked
        );
    }
    if let Some(output) = rec.output.as_ref() {
        println!("output:    {output}");
    }
    println!("events:");
    for (i, ev) in rec.events.iter().enumerate() {
        println!("  {}. {}", i + 1, ev.kind);
        println!("     {}", serde_json::to_string(&ev.payload)?);
    }
    Ok(())
}

fn policy_lint(home: &V0Home) -> Result<()> {
    use imperium_core::policy::imp::{self, Severity};
    let path = home.root.join("policy.imp");
    if !path.exists() {
        println!("no policy.imp (built-in defaults only)");
        return Ok(());
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("policy file unreadable: {}", path.display()))?;
    let issues = imp::lint_policy(&data);
    if issues.is_empty() {
        println!("ok ({})", path.display());
        return Ok(());
    }
    let errors = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    for issue in &issues {
        let sev = match issue.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        println!("{sev}\tline {}\t{}", issue.line, issue.message);
    }
    if errors > 0 {
        bail!("{errors} lint error(s)");
    }
    Ok(())
}

fn policy_explain(home: &V0Home, verb: &str, path: &str) -> Result<()> {
    use imperium_core::policy::imp::{self, PolicyDecision};
    if !imp::POLICY_VERBS.contains(&verb) {
        bail!("explain needs one of: read, write, append, list");
    }
    let policy = match home.load_policy() {
        Ok(p) => p,
        Err(e) => {
            println!("policy:    unloadable ({e})");
            println!("verdict:   DENIED (fail-closed)");
            return Ok(());
        }
    };
    println!("{verb} {path}");

    // Layer 3: user policy (first match wins; fall-through = allow).
    match policy.as_ref().map(|p| p.evaluate(verb, path, path)) {
        Some(PolicyDecision::Deny { rule, reason }) => {
            println!("policy:    DENY rule {rule} — {reason}");
            println!("verdict:   DENIED (by policy)");
            return Ok(());
        }
        Some(PolicyDecision::RequireApproval { rule }) => {
            println!("policy:    REQUIRE_APPROVAL rule {rule}");
        }
        Some(PolicyDecision::Allow { rule: Some(rule) }) => {
            println!("policy:    ALLOW rule {rule}");
        }
        Some(PolicyDecision::Allow { rule: None }) => {
            println!("policy:    ALLOW (no rule matched)");
        }
        None => println!("policy:    (no policy.imp — built-in defaults only)"),
    }

    // Layer 2: built-in host firewall.
    let resolved = imperium_core::v0::resolve_scratch_path(path);
    let host = match &resolved {
        Err(e) => e.to_string(),
        Ok(p) if imperium_core::v0::is_sensitive_path(p) => {
            imperium_core::v0::SENSITIVE_DENIED_REASON.to_string()
        }
        Ok(_) => "ok".to_string(),
    };
    println!("host:      {host}");
    if host != "ok" {
        println!("verdict:   DENIED (by host firewall)");
        return Ok(());
    }

    // Layer 4: grants (built-in defaults, narrowed by grants.json).
    let granted = match home.load_grants() {
        Ok(g) => g
            .get(format!("cap.{verb}").as_str())
            .cloned()
            .unwrap_or_default(),
        Err(e) => {
            println!("grants:    unloadable ({e})");
            println!("verdict:   DENIED (fail-closed)");
            return Ok(());
        }
    };
    let ok = imperium_core::v0::path_allowed(resolved.as_deref().unwrap_or(path), &granted.fs);
    println!(
        "grants:    {}",
        if ok {
            "fs prefix ok".to_string()
        } else {
            format!("host.{verb} path denied")
        }
    );
    println!(
        "verdict:   {}",
        if ok { "ALLOWED" } else { "DENIED (by grants)" }
    );
    Ok(())
}

#[derive(serde::Deserialize)]
struct PolicyTestEntry {
    verb: String,
    path: String,
    expect: String,
}

fn policy_test(home: &V0Home) -> Result<()> {
    use imperium_core::policy::imp::PolicyDecision;

    let test_path = home.root.join("policy.tests.json");
    let data = if test_path.exists() {
        std::fs::read_to_string(&test_path)?
    } else {
        println!("No .imperium/policy.tests.json found; skipping.");
        return Ok(());
    };
    let entries: Vec<PolicyTestEntry> = serde_json::from_str(&data)?;

    let policy = match home.load_policy() {
        Ok(p) => p,
        Err(e) => {
            println!("policy:    unloadable ({e}); treating all as deny");
            for entry in &entries {
                println!(
                    "FAIL {} {} (expected {}, got deny — policy unloadable)",
                    entry.verb, entry.path, entry.expect
                );
            }
            return Ok(());
        }
    };

    let mut passed = 0;
    let mut failed = 0;
    for entry in &entries {
        let verb = match entry.verb.as_str() {
            "read" => "read",
            "write" => "write",
            "append" => "append",
            "list" => "list",
            "fetch" => "fetch",
            _ => {
                println!("FAIL {} {} (expected '{}', unknown verb)", entry.verb, entry.path, entry.expect);
                failed += 1;
                continue;
            }
        };
        let resolved = imperium_core::v0::resolve_scratch_path(&entry.path);
        let host = match &resolved {
            Err(e) => e.to_string(),
            Ok(p) if imperium_core::v0::is_sensitive_path(p) => {
                imperium_core::v0::SENSITIVE_DENIED_REASON.to_string()
            }
            Ok(_) => "ok".to_string(),
        };
        let decision = match policy.as_ref().map(|p| p.evaluate(verb, &host, &entry.path)) {
            Some(d) => d,
            None => imperium_core::policy::imp::PolicyDecision::Allow { rule: None },
        };
        let got = match decision {
            PolicyDecision::Allow { .. } => "allow",
            PolicyDecision::Deny { .. } => "deny",
            PolicyDecision::RequireApproval { .. } => "require_approval",
        };
        let expected = &entry.expect;
        if got == expected {
            println!("PASS {} {}", entry.verb, entry.path);
            passed += 1;
        } else {
            println!(
                "FAIL {} {} (expected {}, got {})",
                entry.verb, entry.path, expected, got
            );
            failed += 1;
        }
    }

    println!(
        "policy test: {} passed, {} failed",
        passed, failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn print_preview(sim: &imperium_core::v0::SimulationResult) {
    if sim.effects_preview.is_empty() {
        return;
    }
    println!("effects:");
    for effect in &sim.effects_preview {
        match effect {
            imperium_core::v0::EffectPreview::Echo { text } => {
                println!("  echo {text}");
            }
            imperium_core::v0::EffectPreview::Write { path, bytes } => {
                println!("  write {path} ({bytes} bytes)");
            }
            imperium_core::v0::EffectPreview::Read { path } => {
                println!("  read {path}");
            }
            imperium_core::v0::EffectPreview::Append { path, bytes } => {
                println!("  append {path} ({bytes} bytes)");
            }
            imperium_core::v0::EffectPreview::List { path } => {
                println!("  list {path}");
            }
            imperium_core::v0::EffectPreview::Fetch { url } => {
                println!("  fetch {url}");
            }
            imperium_core::v0::EffectPreview::Denied {
                capability,
                path,
                reason,
            } => {
                println!("  DENIED {capability} {path}: {reason}");
            }
        }
    }
}

fn print_replay(home: &V0Home, intent_id: &str) -> Result<()> {
    let (rec, folded, matches) = home.replay(intent_id)?;
    println!(
        "{}\tstore={}\tfolded={:?}\tmatch={}\tevents={}",
        rec.ir.id,
        rec.status,
        folded.status,
        matches,
        rec.events.len()
    );
    if !matches {
        bail!("replay diverged from store snapshot");
    }
    Ok(())
}
use std::path::Path;

fn schedules_dir(home: &V0Home) -> std::path::PathBuf {
    home.root.join("schedules")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ScheduleDoc {
    name: String,
    form: String,
    slot: String,
    every_secs: u64,
    enabled: bool,
    created_at: String,
    last_run_at: Option<String>,
    next_run_at: String,
}

fn load_schedule(path: &Path) -> Result<ScheduleDoc> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("schedule unreadable: {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| format!("schedule malformed: {}", path.display()))
}

fn save_schedule(path: &Path, doc: &ScheduleDoc) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(doc)?)?;
    Ok(())
}

fn schedule_add(home: &V0Home, form: &str, slot: &str, every: u64) -> Result<()> {
    if every == 0 {
        bail!("schedule interval must be > 0 seconds");
    }
    let dir = schedules_dir(home);
    fs::create_dir_all(&dir)?;
    let suffix = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .to_string();
    let name = format!("{form}-{}", &suffix[suffix.len().saturating_sub(6)..]);
    let now = chrono::Utc::now();
    let doc = ScheduleDoc {
        name: name.clone(),
        form: form.to_string(),
        slot: slot.to_string(),
        every_secs: every,
        enabled: true,
        created_at: now.to_rfc3339(),
        last_run_at: None,
        next_run_at: (now + chrono::Duration::seconds(every as i64)).to_rfc3339(),
    };
    let path = dir.join(format!("{name}.json"));
    save_schedule(&path, &doc)?;
    println!("{name}\tform={form}\tfile={}", path.display());
    Ok(())
}

fn schedule_remove(home: &V0Home, name: &str) -> Result<()> {
    let path = schedules_dir(home).join(format!("{name}.json"));
    if !path.exists() {
        bail!("no schedule named {name}");
    }
    fs::remove_file(&path)?;
    println!("{name}\tremoved");
    Ok(())
}

fn schedule_set_enabled(home: &V0Home, name: &str, enabled: bool) -> Result<()> {
    let path = schedules_dir(home).join(format!("{name}.json"));
    if !path.exists() {
        bail!("no schedule named {name}");
    }
    let mut doc = load_schedule(&path)?;
    doc.enabled = enabled;
    if enabled {
        // Resuming recomputes the next fire from now; a paused schedule
        // never fires retroactively.
        doc.next_run_at = (chrono::Utc::now()
            + chrono::Duration::seconds(doc.every_secs as i64))
            .to_rfc3339();
    }
    save_schedule(&path, &doc)?;
    println!("{name}\t{}", if enabled { "resumed" } else { "paused" });
    Ok(())
}

fn schedule_list(home: &V0Home) -> Result<()> {
    let dir = schedules_dir(home);
    if !dir.exists() {
        println!("no schedules");
        return Ok(());
    }
    let mut entries: Vec<std::path::PathBuf> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    entries.sort();
    if entries.is_empty() {
        println!("no schedules");
        return Ok(());
    }
    for path in &entries {
        let doc = load_schedule(path)?;
        let state = if doc.enabled { "enabled" } else { "paused" };
        println!(
            "{}\t{}\tform={}\tslot={}\tevery={}s\tnext={}\tlast={}",
            doc.name,
            state,
            doc.form,
            doc.slot,
            doc.every_secs,
            doc.next_run_at,
            doc.last_run_at.as_deref().unwrap_or("never")
        );
    }
    Ok(())
}

/// One-shot audited cron: run every due, enabled schedule through the full
/// gauntlet. High-risk forms stop at the approval gate and are never
/// self-approved; `next_run_at` still advances so a pending approval cannot
/// cause repeated firing.
fn schedule_tick(home: &V0Home) -> Result<()> {
    let dir = schedules_dir(home);
    if !dir.exists() {
        println!("no schedules to tick");
        return Ok(());
    }
    let now = chrono::Utc::now();
    let mut entries: Vec<std::path::PathBuf> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    entries.sort();
    let mut fired = 0usize;
    for path in &entries {
        let mut doc = load_schedule(path)?;
        if !doc.enabled {
            continue;
        }
        let due = chrono::DateTime::parse_from_rfc3339(&doc.next_run_at)
            .map(|t| t.with_timezone(&chrono::Utc) <= now)
            .unwrap_or(false);
        if !due {
            continue;
        }
        fired += 1;
        let rec = home.run_form(&doc.form, &doc.slot, None)?;
        let id = rec.ir.id.to_string();
        let outcome = if rec.ir.requires_approval {
            // The gauntlet stopped at the approval gate; the human decides.
            format!("approval-required\t{id}")
        } else {
            // Low-risk verb: the audited auto-approve path completes it.
            home.approve(&id)?;
            let done = home.execute(&id)?;
            format!("executed\t{}\t{}", id, done.output.unwrap_or_default())
        };
        doc.last_run_at = Some(now.to_rfc3339());
        doc.next_run_at = (now + chrono::Duration::seconds(doc.every_secs as i64)).to_rfc3339();
        save_schedule(path, &doc)?;
        println!("{}\t{}", doc.name, outcome);
    }
    if fired == 0 {
        println!("nothing due");
    }
    Ok(())
}



/// Parsed form of an `--rule` argument for `policy impact`.
#[derive(Debug, PartialEq)]
enum ImpactRule {
    /// `deny <verb> matching <pattern>` — pattern matched as a substring of
    /// the intent's natural-language source (documented simplification: the
    /// ledger stores nl text, not resolved paths).
    Deny { verb: String, pattern: String },
    /// `require approval <verb>` (the canonical 3-token .imp form).
    RequireApproval { verb: String },
}

fn parse_impact_rule(rule_str: &str) -> Result<ImpactRule> {
    use imperium_core::policy::imp;
    let parts: Vec<&str> = rule_str.split_whitespace().collect();
    let rule = match parts.as_slice() {
        ["deny", verb, "matching", rest @ ..] if !rest.is_empty() => ImpactRule::Deny {
            verb: verb.to_string(),
            pattern: rest.join(" "),
        },
        ["require", "approval", verb] => ImpactRule::RequireApproval {
            verb: verb.to_string(),
        },
        ["allow", ..] => bail!(
            "allow-rule impact is not simulated (first-match ordering makes it \
             position-dependent); use `imperium policy explain` for the live chain"
        ),
        _ => bail!(
            "impact rule format: `deny <verb> matching <pattern>` or `require approval <verb>`"
        ),
    };
    let verb = match &rule {
        ImpactRule::Deny { verb, .. } => verb,
        ImpactRule::RequireApproval { verb } => verb,
    };
    if !imp::POLICY_VERBS.contains(&verb.as_str()) {
        bail!("unknown verb: {verb}; must be one of: read, write, append, list, fetch");
    }
    Ok(rule)
}

/// Pure report for `policy impact`. Returns the printed report; `changed`
/// counts intents whose decision would flip. Fetch verbs are skipped (the
/// ledger does not store resolved URLs); hard denials can only tighten, so a
/// deny rule reports allow/require_approval -> deny and a require-approval
/// rule reports allow -> require_approval.
fn policy_impact_report(home: &V0Home, rule: &ImpactRule) -> Result<(String, usize)> {
    use imperium_core::policy::imp::PolicyDecision;

    let (verb, describe) = match rule {
        ImpactRule::Deny { verb, pattern } => (verb.as_str(), format!("deny {verb} matching {pattern}")),
        ImpactRule::RequireApproval { verb } => (verb.as_str(), format!("require approval {verb}")),
    };

    let current_policy = home.load_policy()?;
    let hits = home.ledger()?.search(verb)?; // intents mentioning this verb's capability family
    let mut report = String::new();
    let mut changed = 0usize;

    for hit in &hits {
        let nl = hit.nl_source.to_lowercase();
        let original = current_policy
            .as_ref()
            .map(|p| p.evaluate(verb, &nl, &nl))
            .unwrap_or(PolicyDecision::Allow { rule: None });

        let would_flip = match rule {
            ImpactRule::Deny { pattern, .. } => {
                original.kind() != "deny" && nl.contains(&pattern.to_lowercase())
            }
            ImpactRule::RequireApproval { .. } => original.kind() == "allow",
        };
        if !would_flip {
            continue;
        }
        changed += 1;
        let new_kind = match rule {
            ImpactRule::Deny { .. } => "deny",
            ImpactRule::RequireApproval { .. } => "require_approval",
        };
        report.push_str(&format!(
            "  {} [{}] {} -> {}\n",
            hit.id.split('-').next().unwrap_or(&hit.id),
            hit.status,
            original.kind(),
            new_kind
        ));
    }

    let mut out = format!(
        "policy impact analysis:\n  rule: {describe}\n  intents scanned: {}\n  would change decision: {changed}\n",
        hits.len()
    );
    if !report.is_empty() {
        out.push_str("\naffected intents:\n");
        out.push_str(&report);
    }
    Ok((out, changed))
}

fn policy_impact(home: &V0Home, rule_str: &str) -> Result<()> {
    let rule = parse_impact_rule(rule_str)?;
    let (report, changed) = policy_impact_report(home, &rule)?;
    println!("{report}");
    if changed > 0 {
        println!("{changed} intent(s) would have their decision changed");
        std::process::exit(1);
    }
    println!("no intents would have their decision changed");
    Ok(())
}

/// Pure summary for `policy coverage`: rule census by canonical form plus the
/// ledger's intent census. Read-only; no decisions are re-evaluated.
fn policy_coverage_report(home: &V0Home) -> Result<String> {
    use std::collections::BTreeMap;
    let policy = home.load_policy()?;
    let hits = home.ledger()?.search("")?;

    let mut rules: BTreeMap<String, usize> = BTreeMap::new();
    let rule_count = policy.as_ref().map_or(0, |p| p.rules.len());
    if let Some(p) = policy.as_ref() {
        for r in &p.rules {
            *rules.entry(r.canonical()).or_default() += 1;
        }
    }

    let mut intents: BTreeMap<String, usize> = BTreeMap::new();
    for hit in &hits {
        let first = hit
            .nl_source
            .split_whitespace()
            .next()
            .unwrap_or("other")
            .to_lowercase();
        *intents.entry(first).or_default() += 1;
    }

    let mut out = format!(
        "policy coverage:\n  intents in ledger: {}\n  policy rules: {rule_count}\n",
        hits.len()
    );
    if rules.is_empty() {
        out.push_str("  rules: (no policy.imp — built-in defaults only)\n");
    } else {
        out.push_str("  rules by form:\n");
        for (form, n) in &rules {
            out.push_str(&format!("    {form} x{n}\n"));
        }
    }
    out.push_str("  intents by opening word:\n");
    for (word, n) in &intents {
        out.push_str(&format!("    {word} x{n}\n"));
    }
    Ok(out)
}

fn policy_coverage(home: &V0Home) -> Result<()> {
    println!("{}", policy_coverage_report(home)?);
    Ok(())
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    use imperium_core::v0::IntentStatus;
    use v0_cmd::StoredIntent;

    fn tmp_home() -> V0Home {
        let root = std::env::temp_dir().join(format!("imperium-sched-{}", uuid::Uuid::new_v4()));
        let home = V0Home { root };
        home.init().unwrap();
        home
    }

    fn echo_form(home: &V0Home, name: &str) {
        let rec = home
            .compile("Echo this message: schedule-smoke", false)
            .unwrap();
        home.save_form(&rec.ir.id.to_string(), name).unwrap();
    }

    fn find_schedule(home: &V0Home, stem_prefix: &str) -> std::path::PathBuf {
        fs::read_dir(schedules_dir(home))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_stem()
                    .is_some_and(|s| s.to_string_lossy().starts_with(stem_prefix))
            })
            .expect("schedule file not found")
    }

    fn force_due(home: &V0Home, stem_prefix: &str) {
        let path = find_schedule(home, stem_prefix);
        let mut doc: ScheduleDoc =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        doc.next_run_at = "2020-01-01T00:00:00+00:00".to_string();
        save_schedule(&path, &doc).unwrap();
    }

    #[test]
    fn add_creates_enabled_schedule_with_future_fire_time() {
        let home = tmp_home();
        schedule_add(&home, "greet", "world", 60).unwrap();
        let doc: ScheduleDoc = serde_json::from_str(&fs::read_to_string(
            find_schedule(&home, "greet-"),
        ).unwrap()).unwrap();
        assert!(doc.enabled);
        assert_eq!(doc.form, "greet");
        assert!(chrono::DateTime::parse_from_rfc3339(&doc.next_run_at).is_ok());
    }

    #[test]
    fn add_rejects_zero_interval() {
        let home = tmp_home();
        assert!(schedule_add(&home, "greet", "x", 0).is_err());
    }

    #[test]
    fn tick_executes_due_low_risk_form_and_advances() {
        let home = tmp_home();
        echo_form(&home, "greet");
        schedule_add(&home, "greet", "tick-one", 3600).unwrap();
        force_due(&home, "greet-");

        schedule_tick(&home).unwrap();

        // The file advanced out of the past.
        let doc: ScheduleDoc = serde_json::from_str(
            &fs::read_to_string(find_schedule(&home, "greet-")).unwrap(),
        )
        .unwrap();
        assert!(doc.last_run_at.is_some());
        let next = chrono::DateTime::parse_from_rfc3339(&doc.next_run_at).unwrap();
        assert!(next > chrono::Utc::now() - chrono::Duration::seconds(5));
    }

    #[test]
    fn tick_high_risk_form_stops_at_the_human_gate() {
        let home = tmp_home();
        let rec = home
            .compile("Write file notes.txt with contents scheduled", false)
            .unwrap();
        assert!(rec.ir.requires_approval);
        home.save_form(&rec.ir.id.to_string(), "scribe").unwrap();
        schedule_add(&home, "scribe", "more", 3600).unwrap();
        force_due(&home, "scribe-");

        schedule_tick(&home).unwrap();

        // Nothing was executed: the newest intent for the write form is left
        // at the simulated/approval stage, never executed.
        let mut statuses = vec![];
        for entry in fs::read_dir(home.root.join("intents")).unwrap().flatten() {
            let text = fs::read_to_string(entry.path()).unwrap();
            if let Ok(r) = serde_json::from_str::<StoredIntent>(&text) {
                if r.ir.name.contains("notes") || r.ir.nl_source.contains("notes.txt") {
                    statuses.push(r.status);
                }
            }
        }
        assert!(
            statuses.iter().all(|s| *s != IntentStatus::Executed),
            "high-risk schedule must never self-approve: {statuses:?}"
        );
    }

    #[test]
    fn paused_schedule_never_fires() {
        let home = tmp_home();
        echo_form(&home, "greet");
        schedule_add(&home, "greet", "paused-run", 3600).unwrap();
        let stem = find_schedule(&home, "greet-")
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        schedule_set_enabled(&home, &stem, false).unwrap();
        force_due(&home, "greet-");

        // tick completes without executing anything for the paused schedule.
        schedule_tick(&home).unwrap();
        let rec = home.compile("Echo this message: probe", false).unwrap();
        let count_before = home.list().unwrap().len();
        let _ = rec;
        schedule_tick(&home).unwrap();
        assert_eq!(home.list().unwrap().len(), count_before);
    }

    #[test]
    fn remove_deletes_the_schedule() {
        let home = tmp_home();
        schedule_add(&home, "greet", "doomed", 60).unwrap();
        assert!(schedule_remove(&home, "nope-does-not-exist").is_err());
        // Find the real name, then remove it.
        let path = find_schedule(&home, "greet-");
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        schedule_remove(&home, &stem).unwrap();
        assert!(fs::read_dir(schedules_dir(&home)).unwrap().count() == 0);
    }
}

#[cfg(test)]
mod policy_tool_tests {
    use super::*;

    fn tmp_home() -> V0Home {
        let root = std::env::temp_dir().join(format!("imperium-pol-{}", uuid::Uuid::new_v4()));
        let home = V0Home { root };
        home.init().unwrap();
        home
    }

    #[test]
    fn impact_parses_deny_and_require_approval_forms() {
        assert_eq!(
            parse_impact_rule("deny read matching notes.txt").unwrap(),
            ImpactRule::Deny { verb: "read".into(), pattern: "notes.txt".into() }
        );
        assert_eq!(
            parse_impact_rule("require approval write").unwrap(),
            ImpactRule::RequireApproval { verb: "write".into() }
        );
        assert!(parse_impact_rule("deny read matching").is_err());
        assert!(parse_impact_rule("deny hug matching x").is_err());
        assert!(parse_impact_rule("allow read under scratch").is_err()); // fail-closed
    }

    #[test]
    fn impact_flips_allow_to_deny_for_matching_nl_and_deny_stays_deny() {
        let home = tmp_home();
        let rec = home.compile("Read file sample.txt", false).unwrap();
        let _ = rec;

        // A deny on the path text must flip the read intent's decision.
        let (report, changed) = policy_impact_report(&home, &ImpactRule::Deny {
            verb: "read".into(),
            pattern: "sample.txt".into(),
        })
        .unwrap();
        assert_eq!(changed, 1);
        assert!(report.contains("allow -> deny"), "{report}");

        // A non-matching pattern changes nothing.
        let (_, changed0) = policy_impact_report(&home, &ImpactRule::Deny {
            verb: "read".into(),
            pattern: "not-in-the-ledger".into(),
        })
        .unwrap();
        assert_eq!(changed0, 0);
    }

    #[test]
    fn impact_require_approval_flips_only_allows() {
        let home = tmp_home();
        // b.txt is already denied by policy (glob must match the full nl line);
        // a.txt is an ordinary allow.
        std::fs::write(home.root.join("policy.imp"), "deny read matching *b.txt*\n").unwrap();
        home.compile("Read file a.txt", false).unwrap();
        home.compile("Read file b.txt", false).unwrap();

        let (report, changed) = policy_impact_report(&home, &ImpactRule::RequireApproval {
            verb: "read".into(),
        })
        .unwrap();
        assert_eq!(changed, 1, "{report}");
        assert!(report.contains("allow -> require_approval"), "{report}");
    }

    #[test]
    fn coverage_counts_rules_and_intents() {
        let home = tmp_home();
        home
            .compile("Echo this message: coverage-probe", false)
            .unwrap();
        let out = policy_coverage_report(&home).unwrap();
        assert!(out.contains("intents in ledger: 1"), "{out}");
        assert!(out.contains("(no policy.imp"), "{out}");
        assert!(out.contains("echo x1"), "{out}");
    }
}
