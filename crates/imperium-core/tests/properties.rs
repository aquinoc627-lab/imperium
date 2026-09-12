//! Phase 21 property tests: adversarial inputs must never panic the kernel,
//! and outputs must hold the documented invariants. Deterministic, local,
//! no network. Runs under the gated integration target (`cargo test -p
//! imperium-core`).

use imperium_core::policy::imp::{glob_match, lint_policy, parse_policy_lenient, parse_rules};
use imperium_core::v0::*;
use proptest::prelude::*;

/// Arbitrary strings: short/medium adversarial Unicode (including NUL and
/// controls) plus occasional near-cap inputs (Phase 21 ceiling = 64 KiB).
fn any_nl() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::collection::vec(any::<char>(), 0..256).prop_map(|v| v.into_iter().collect()),
        proptest::collection::vec(any::<char>(), 60_000..70_000)
            .prop_map(|v| v.into_iter().collect()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn compile_rules_never_panics_and_ir_is_sane(nl in any_nl()) {
        let Ok(ir) = compile_rules(&nl) else { return Ok(()) };
        assert!(ir.validate().is_ok(), "compiled IR must validate");
        assert!((0.0..=1.0).contains(&ir.risk_score));
        assert!(!ir.tasks.is_empty());
        for task in &ir.tasks {
            assert!(!task.capabilities.is_empty());
            assert!(task.capabilities.iter().all(|c| !c.is_empty()));
            if let Some(path) = task.target_path.as_deref() {
                assert!(!path.contains('\0'), "target path must never carry NUL");
            }
            for effect in &task.effects {
                // Effect paths come from the same resolver; never NUL.
                let path = match effect {
                    imperium_core::intent::Effect::Echo { .. } => None,
                    imperium_core::intent::Effect::Write { path, .. }
                    | imperium_core::intent::Effect::Read { path }
                    | imperium_core::intent::Effect::Append { path, .. }
                    | imperium_core::intent::Effect::List { path } => Some(path),
                    imperium_core::intent::Effect::Fetch { url } => Some(url),
                };
                if let Some(p) = path {
                    assert!(!p.contains('\0'), "effect path must never carry NUL");
                }
            }
        }
    }

    #[test]
    fn compile_dry_run_and_monte_carlo_never_panic(nl in any_nl()) {
        if let Ok(ir) = compile_rules(&nl) {
            let _ = dry_run(&ir);
            let (sim, events) = dry_run_events(&ir);
            assert!(!sim.effects_preview.is_empty());
            let _ = fold_events(&events);
            let _ = dry_run_monte_carlo(&ir, None, None, 16, 42);
        }
    }

    #[test]
    fn local_propose_never_panics(nl in any_nl()) {
        let _ = local_propose(&nl);
    }

    #[test]
    fn resolve_scratch_path_holds_confinement_invariants(raw in any_nl()) {
        if let Ok(resolved) = resolve_scratch_path(&raw) {
            assert!(resolved.starts_with("scratch/"), "resolved: {resolved}");
            assert!(!resolved.contains('\0'));
            assert!(!resolved.contains('\\'));
            assert!(
                !resolved.split('/').any(|seg| seg == ".." || seg == "."),
                "no dot components in resolved: {resolved}"
            );
        }
    }

    #[test]
    fn fetch_url_host_never_panics_and_hosts_are_sane(url in any_nl()) {
        if let Ok(host) = fetch_url_host(&url) {
            assert!(host.contains('.'));
            assert!(!host.contains('@'));
            assert!(!host.contains(':'));
            let parts: Vec<&str> = host.split('.').collect();
            let ipv4 = parts.len() == 4
                && parts
                    .iter()
                    .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));
            assert!(!ipv4, "host must not be an IPv4 literal: {host}");
        }
    }

    #[test]
    fn policy_parser_and_linter_never_panic(text in any_nl()) {
        let _ = parse_rules(&text);
        let _ = lint_policy(&text);
        let (policy, _) = parse_policy_lenient(&text);
        for verb in ["read", "write", "append", "list", "fetch"] {
            let _ = policy.evaluate(verb, &text, &text);
        }
        let _ = policy.requires_approval(&text);
        let _ = policy.content_denied(&text);
        let _ = glob_match(&text, &text);
        let _ = imperium_core::policy::imp::builtin_content_denied(&text);
    }

    #[test]
    fn diff_preview_never_panics_and_is_capped(
        (old, new) in (proptest::option::of(any_nl()), any_nl())
    ) {
        let lines = imperium_core::diff_preview(old.as_deref(), &new);
        assert!(lines.len() <= 21, "diff preview must be capped: {}", lines.len());
    }

    #[test]
    fn token_roundtrip_tamper_and_net_floor(secret in any_nl()) {
        // Echo grants carry no fs permissions; tokens must match that shape.
        let perms = Permissions::empty();
        let token = issue_token(ECHO_CAP, "cli", "intent-1", perms, 1000, 60_000, &secret);
        let seen = std::collections::HashSet::new();
        let revoked = std::collections::HashSet::new();
        let grant = grant_for_capability(ECHO_CAP);
        let ctx = VerifyContext {
            secret: &secret,
            now_ms: 1500,
            seen_nonces: &seen,
            revoked_ids: &revoked,
            grant: &grant,
            expected_subject: None,
            expected_intent: None,
        };
        assert_eq!(verify_token(&token, &ctx), VerifyReason::Ok);

        // Flip every hex char of the signature: must fail closed.
        let mut bad = token.clone();
        bad.signature = bad
            .signature
            .chars()
            .map(|c| if c == '0' { '1' } else { '0' })
            .collect();
        assert_ne!(bad.signature, token.signature);
        assert_eq!(verify_token(&bad, &ctx), VerifyReason::InvalidSignature);

        // Net permissions on a non-http token are rejected by the net floor.
        let mut nettok = token.clone();
        nettok.permissions.net.push("example.com".into());
        nettok.signature = sign_token(&nettok, &secret);
        assert_eq!(verify_token(&nettok, &ctx), VerifyReason::NetworkDenied);
    }

    #[test]
    fn path_allowed_matches_the_reference_prefix_rule(
        (path, prefix) in (any_nl(), any_nl())
    ) {
        let np = prefix.replace('\\', "/");
        let with_slash = if np.ends_with('/') {
            np.clone()
        } else {
            format!("{np}/")
        };
        let norm = path.replace('\\', "/");
        let expected = norm == np || norm.starts_with(&with_slash);
        assert_eq!(path_allowed(&path, &[prefix]), expected);
    }

    #[test]
    fn mc_gate_matches_integer_exact_reference(
        (successes, trials) in (0u64..1000, 0u64..1000)
    ) {
        let expected = trials > 0 && successes * 10_000 >= 9_000 * trials;
        assert_eq!(mc_gate(successes, trials), expected);
    }

    #[test]
    fn sensitive_path_and_truncation_never_panic(nl in any_nl()) {
        let _ = is_sensitive_path(&nl);
        // The echo-name truncation path (regression: byte-boundary panic).
        if let Ok(ir) = compile_rules(&format!("Echo this message: {nl}")) {
            assert!(!ir.name.is_empty());
        }
    }
}
