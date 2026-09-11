//! Contract tests: the Rust canonical kernel must satisfy the shared
//! fixtures in `tests/contract/v0_kernel.json` — the same file the TS
//! reference (`web/v0`) validates against.

use imperium_core::forms::{build_template, verify_shadow};
use imperium_core::policy::imp;
use imperium_core::v0::*;
use imperium_core::IntentIR;
use serde_json::{json, Value};

fn fixtures() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/v0_kernel.json");
    let data = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&data).expect("fixture parses")
}

/// Recursive subset check: every key present in `expected` must match
/// `actual`; `actual` may carry extra keys (e.g. Rust-only serde fields).
/// Numbers compare numerically (1 == 1.0) to stay language-agnostic.
fn subset(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => a
            .as_f64()
            .zip(e.as_f64())
            .map(|(a, e)| (a - e).abs() < 1e-6)
            .unwrap_or(false),
        (Value::Object(a), Value::Object(e)) => e
            .iter()
            .all(|(k, ev)| a.get(k).map(|av| subset(av, ev)).unwrap_or(false)),
        (Value::Array(a), Value::Array(e)) => {
            a.len() == e.len() && a.iter().zip(e).all(|(av, ev)| subset(av, ev))
        }
        _ => actual == expected,
    }
}

fn normalize_ir(v: &mut Value) {
    v["id"] = json!("<uuid>");
    v["compiled_at"] = json!("<ts>");
    for t in v["tasks"].as_array_mut().expect("tasks array") {
        t["id"] = json!("<task-uuid>");
    }
}

#[test]
fn compile_cases_match_reference() {
    let fx = fixtures();
    for case in fx["compile_cases"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let ir = compile_rules(input)
            .unwrap_or_else(|e| panic!("case {}: compile failed: {e}", case["name"]));
        let mut v = serde_json::to_value(&ir).unwrap();
        normalize_ir(&mut v);
        assert!(
            subset(&v, &case["expected_ir"]),
            "case {}: IR mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn compile_errors_match_reference() {
    let fx = fixtures();
    for case in fx["compile_errors"].as_array().unwrap() {
        let err = compile_rules(case["input"].as_str().unwrap()).expect_err("compile should fail");
        assert_eq!(
            err,
            case["error"].as_str().unwrap(),
            "case {}: error message mismatch",
            case["name"]
        );
    }
}

#[test]
fn propose_cases_match_reference() {
    let fx = fixtures();
    for case in fx["propose_cases"].as_array().unwrap() {
        let (canonical, proposer) = local_propose(case["input"].as_str().unwrap())
            .unwrap_or_else(|e| panic!("case {}: propose failed: {e}", case["name"]));
        assert_eq!(canonical, case["canonical"].as_str().unwrap());
        assert_eq!(proposer, case["proposer"].as_str().unwrap());
    }
    for case in fx["propose_errors"].as_array().unwrap() {
        let err = local_propose(case["input"].as_str().unwrap()).expect_err("propose should fail");
        assert_eq!(err, case["error"].as_str().unwrap());
    }
}

#[test]
fn token_signatures_match_reference() {
    let fx = fixtures();
    for case in fx["token_cases"].as_array().unwrap() {
        let claims = &case["claims"];
        let token = CapabilityToken {
            id: claims["id"].as_str().unwrap().into(),
            capability: claims["capability"].as_str().unwrap().into(),
            subject: claims["subject"].as_str().unwrap().into(),
            intent_id: claims["intent_id"].as_str().unwrap().into(),
            permissions: Permissions {
                fs: claims["permissions"]["fs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().into())
                    .collect(),
                net: claims["permissions"]["net"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().into())
                    .collect(),
                env: claims["permissions"]["env"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().into())
                    .collect(),
            },
            nonce: claims["nonce"].as_str().unwrap().into(),
            issued_at: claims["issued_at"].as_i64().unwrap(),
            expires_at: claims["expires_at"].as_i64().unwrap(),
            signature: String::new(),
        };
        let sig = sign_token(&token, case["secret"].as_str().unwrap());
        assert_eq!(
            sig,
            case["signature_hex"].as_str().unwrap(),
            "canonical payload signature diverged from TS reference"
        );
    }
}

fn folded_to_json(f: &FoldedState) -> Value {
    json!({
        "status": f.status.as_ref().map(|s| s.to_string()),
        "output": f.output,
        "fail_reason": f.fail_reason,
        "proposer": f.proposer,
        "canonical": f.canonical,
        "token": f.token.as_ref().map(|t| json!({
            "token_id": t.token_id,
            "fingerprint": t.fingerprint,
            "revoked": t.revoked,
        })),
        "simulation": f.simulation.as_ref().map(|s| serde_json::to_value(s).unwrap()),
    })
}

#[test]
fn fold_cases_match_reference() {
    let fx = fixtures();
    for case in fx["fold_cases"].as_array().unwrap() {
        let events: Vec<V0Event> =
            serde_json::from_value(case["events"].clone()).expect("events parse");
        let folded = fold_events(&events);
        let v = folded_to_json(&folded);
        assert!(
            subset(&v, &case["expected"]),
            "case {}: fold mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn simulate_cases_match_reference() {
    let fx = fixtures();
    for case in fx["simulate_cases"].as_array().unwrap() {
        let ir: IntentIR = serde_json::from_value(case["ir"].clone()).expect("fixture IR parses");
        let policy = case.get("policy").and_then(|p| p.as_str()).map(|src| {
            let (policy, issues) = imp::parse_policy_lenient(src);
            assert!(issues.is_empty(), "fixture policy must parse");
            policy
        });
        let sim = dry_run_with_policy(&ir, policy.as_ref());
        let v = serde_json::to_value(&sim).unwrap();
        assert!(
            subset(&v, &case["expected"]),
            "case {}: simulation mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn policy_cases_match_reference() {
    let fx = fixtures();
    for case in fx["policy_cases"].as_array().unwrap() {
        let (policy, issues) = imp::parse_policy_lenient(case["policy"].as_str().unwrap());
        assert!(
            issues.is_empty(),
            "case {}: policy must parse",
            case["name"]
        );
        let decision = policy.evaluate(
            case["verb"].as_str().unwrap(),
            case["path"].as_str().unwrap(),
            case["text"].as_str().unwrap(),
        );
        let v = serde_json::to_value(&decision).unwrap();
        assert!(
            subset(&v, &case["expected"]),
            "case {}: decision mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn policy_parse_errors_match_reference() {
    let fx = fixtures();
    for case in fx["policy_parse_errors"].as_array().unwrap() {
        let (_, issues) = imp::parse_policy_lenient(case["policy"].as_str().unwrap());
        let v = serde_json::to_value(&issues).unwrap();
        assert!(
            subset(&v, &case["issues"]),
            "case {}: parse issues mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn policy_lint_cases_match_reference() {
    let fx = fixtures();
    for case in fx["policy_lint_cases"].as_array().unwrap() {
        let issues = imp::lint_policy(case["policy"].as_str().unwrap());
        let v = serde_json::to_value(&issues).unwrap();
        assert!(
            subset(&v, &case["issues"]),
            "case {}: lint issues mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn builtin_content_cases_match_reference() {
    let fx = fixtures();
    for case in fx["builtin_content_cases"].as_array().unwrap() {
        let denied = imp::builtin_content_denied(case["text"].as_str().unwrap());
        assert_eq!(
            denied,
            case["expected"].as_bool().unwrap(),
            "case {}: builtin content mismatch",
            case["name"]
        );
    }
}
#[test]
fn monte_carlo_cases_match_reference() {
    let fx = fixtures();
    let mut pins: Vec<String> = vec![];
    for case in fx["monte_carlo_cases"].as_array().unwrap() {
        let ir: IntentIR = serde_json::from_value(case["ir"].clone()).expect("fixture IR parses");
        let stats: std::collections::BTreeMap<String, CapabilityStat> =
            serde_json::from_value(case["stats"].clone()).expect("fixture stats parse");
        let sim = dry_run_monte_carlo(
            &ir,
            None,
            Some(&stats),
            case["trials"].as_u64().unwrap(),
            case["seed"].as_u64().unwrap(),
        );
        let v = serde_json::to_value(&sim).unwrap();
        if case["expected"].get("PLACEHOLDER").is_some() {
            pins.push(format!("PIN-ME {}: {v}", case["name"]));
            continue;
        }
        assert!(
            subset(&v, &case["expected"]),
            "case {}: monte carlo mismatch\nactual: {v}",
            case["name"]
        );
    }
    assert!(pins.is_empty(), "\n{}", pins.join("\n"));
}

#[test]
fn form_template_cases_match_reference() {
    let fx = fixtures();
    for case in fx["form_template_cases"].as_array().unwrap() {
        let ir: IntentIR = serde_json::from_value(case["ir"].clone()).expect("fixture IR parses");
        let v = match build_template(&ir) {
            Ok(t) => serde_json::to_value(&t).unwrap(),
            Err(e) => json!({"error": e}),
        };
        assert!(
            subset(&v, &case["expected"]),
            "case {}: form template mismatch\nactual: {v}",
            case["name"]
        );
    }
}

#[test]
fn shadow_diff_cases_match_reference() {
    let fx = fixtures();
    for case in fx["shadow_diff_cases"].as_array().unwrap() {
        let predicted: Vec<EffectPreview> =
            serde_json::from_value(case["predicted"].clone()).expect("predicted parses");
        let actual: Vec<EffectPreview> =
            serde_json::from_value(case["actual"].clone()).expect("actual parses");
        let diff = verify_shadow(&predicted, &actual);
        let v = serde_json::to_value(&diff).unwrap();
        assert!(
            subset(&v, &case["expected"]),
            "case {}: shadow diff mismatch\nactual: {v}",
            case["name"]
        );
    }
}
