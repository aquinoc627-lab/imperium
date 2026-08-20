//! IMPERIUM CLI — v0 intent loop is real; other verbs fail closed.

mod v0_cmd;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
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
    /// Replay from the event log
    Replay {
        #[arg(short, long)]
        intent_id: String,
    },
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
    },
    Approve {
        #[arg(short, long)]
        intent_id: String,
    },
    Execute {
        #[arg(short, long)]
        intent_id: String,
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
        Commands::Intent { command } => match command {
            IntentCommands::Compile { input, propose } => {
                let nl = resolve_nl(&input)?;
                let rec = home.compile(&nl, propose)?;
                println!("{}\t{}\t{}", rec.ir.id, rec.status, rec.ir.name);
            }
            IntentCommands::Simulate { intent_id } => {
                let rec = home.simulate(&intent_id)?;
                let sim = rec.simulation.expect("simulated");
                println!(
                    "{}\t{}\tsuccess={} risk={} duration_ms={}",
                    rec.ir.id,
                    rec.status,
                    sim.success_probability,
                    sim.risk,
                    sim.duration_ms
                );
            }
            IntentCommands::Approve { intent_id } => {
                let rec = home.approve(&intent_id)?;
                let fp = rec
                    .token
                    .as_ref()
                    .map(|t| t.token.signature.chars().take(12).collect::<String>())
                    .unwrap_or_default();
                println!("{}\t{}\ttoken={}", rec.ir.id, rec.status, fp);
            }
            IntentCommands::Execute { intent_id } => {
                let rec = home.execute(&intent_id)?;
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
