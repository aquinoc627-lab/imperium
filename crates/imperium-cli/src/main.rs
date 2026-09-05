//! IMPERIUM CLI — v0 intent loop is real; other verbs fail closed.

mod mcp;
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
    #[serde(default)]
    text: String,
}

fn policy_test(home: &V0Home) -> Result<()> {
    use imperium_core::policy::imp::{self, PolicyDecision};

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
