//! Phase 25 — canonical first-run path (echo only).

use anyhow::{bail, Result};
use crate::v0_cmd::V0Home;

/// Fixed sentence for the stranger path. Do not widen without a new spec.
pub const DEMO_NL: &str = "Echo this message: ping";

/// Compile → simulate → approve → execute with a numbered log.
/// Success ends with a line exactly equal to `demo ok`.
pub fn run_first_run(home: &V0Home) -> Result<()> {
    println!("IMPERIUM first-run demo");
    println!("home: {}", home.root.display());
    println!("sentence: {DEMO_NL}");

    home.init()?;
    println!("1. init      ok");

    let rec = home.compile(DEMO_NL, false)?;
    let id = rec.ir.id.to_string();
    println!("2. compile   {}  {}", id, rec.ir.name);

    let rec = home.simulate_opts(&id, None)?;
    let sim = rec
        .simulation
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("simulate produced no result"))?;
    println!(
        "3. simulate  success={} risk={} duration_ms={}",
        sim.success_probability, sim.risk, sim.duration_ms
    );
    for effect in &sim.effects_preview {
        println!("   effect: {effect:?}");
    }

    let rec = home.approve(&id)?;
    let token = rec
        .token
        .as_ref()
        .map(|t| t.token.id.as_str())
        .unwrap_or("—");
    println!("4. approve   token {token}");

    let rec = home.execute(&id)?;
    let out = rec.output.as_deref().unwrap_or("");
    println!("5. execute   {out}");

    if !out.contains("ping") {
        bail!("demo failed: execute output missing expected 'ping'");
    }

    println!("demo ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_echo_succeeds_in_temp_home() {
        let dir = std::env::temp_dir().join(format!(
            "imperium-demo-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let home = V0Home { root: dir.clone() };
        run_first_run(&home).expect("demo");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
