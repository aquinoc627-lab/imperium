//! Forms — Phase 13 "The Evolution Loop".
//!
//! The human-gated response to friction: a saved, parameterized template
//! that re-enters the normal gauntlet (dry-run → policy → grants → token →
//! approval) on every use. Slot inference is mechanical (canonical forms
//! have exactly one free-text field); there is no LLM and no autonomy.
//!
//! Also here: the shadow comparator. `execute --shadow` redirects
//! destructive effects under `scratch/shadow/` and folds a `ShadowVerified`
//! event comparing the dry-run's promises against the shadow run's actual
//! effects. A verified badge authorizes nothing — it is an honest signal.

use crate::intent::IntentIR;
use crate::v0::EffectPreview;
use serde::{Deserialize, Serialize};

pub const SLOT_PLACEHOLDER: &str = "{{slot}}";
pub const SLOT_NAME: &str = "description";

/// Reconstruct the canonical form string from a compiled IR (the inverse of
/// what the rules compiler parses). Fetch intents have no free-text slot.
pub fn canonical_of_intent(ir: &IntentIR) -> Result<String, String> {
    let task = ir.tasks.first().ok_or("intent has no tasks")?;
    let cap = task.capabilities.first().map(String::as_str).unwrap_or("");
    let target = task
        .target_path
        .as_deref()
        .ok_or("intent has no target path")?;
    let rel = target.strip_prefix("scratch/").unwrap_or(target);
    Ok(match cap {
        "cap.echo" => format!("Echo this message: {}", task.description),
        "cap.write" => format!("Write file {rel} with contents {}", task.description),
        "cap.read" => format!("Read file {rel}"),
        "cap.append" => format!("Append file {rel} with contents {}", task.description),
        "cap.list" => format!("List files under {rel}"),
        "cap.http" => format!("Fetch {target}"),
        other => return Err(format!("unknown capability: {other}")),
    })
}

/// Build a form template from a compiled IR. In v0 the slot is always the
/// task description (the only free-text field in canonical forms; for
/// read/list it is the path). Built structurally — never by substring
/// replacement. Fetch has no free text and cannot become a form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormTemplate {
    pub template: String,
    pub slot: String,
}

pub fn build_template(ir: &IntentIR) -> Result<FormTemplate, String> {
    let task = ir.tasks.first().ok_or("intent has no tasks")?;
    let cap = task.capabilities.first().map(String::as_str).unwrap_or("");
    if cap == "cap.http" {
        return Err("fetch intents cannot become forms".into());
    }
    fn rel_for(target: &str) -> &str {
        target.strip_prefix("scratch/").unwrap_or(target)
    }
    let template = match cap {
        "cap.echo" => format!("Echo this message: {SLOT_PLACEHOLDER}"),
        "cap.write" => {
            format!(
                "Write file {} with contents {SLOT_PLACEHOLDER}",
                rel_for(
                    task.target_path
                        .as_deref()
                        .ok_or("intent has no target path")?
                )
            )
        }
        "cap.read" => format!("Read file {SLOT_PLACEHOLDER}"),
        "cap.append" => {
            format!(
                "Append file {} with contents {SLOT_PLACEHOLDER}",
                rel_for(
                    task.target_path
                        .as_deref()
                        .ok_or("intent has no target path")?
                )
            )
        }
        "cap.list" => {
            format!("List files under {SLOT_PLACEHOLDER}")
        }
        other => return Err(format!("unknown capability: {other}")),
    };
    Ok(FormTemplate {
        template,
        slot: SLOT_NAME.into(),
    })
}

/// Substitute the slot and return the canonical input for the compiler.
pub fn render_template(template: &str, value: &str) -> Result<String, String> {
    if !template.contains(SLOT_PLACEHOLDER) {
        return Err("template has no slot".into());
    }
    Ok(template.replace(SLOT_PLACEHOLDER, value))
}

/// Shadow comparison: normalize both sides onto the same relative path
/// (`scratch/shadow/x` and `scratch/x` both → `x`), then require same
/// kind, same path, same bytes.
fn normalized_path(p: &str) -> &str {
    p.strip_prefix("scratch/shadow/")
        .or_else(|| p.strip_prefix("scratch/"))
        .unwrap_or(p)
}

fn preview_matches(a: &EffectPreview, b: &EffectPreview) -> bool {
    match (a, b) {
        (EffectPreview::Echo { text: a }, EffectPreview::Echo { text: b }) => a == b,
        (
            EffectPreview::Write { path: a, bytes: x },
            EffectPreview::Write { path: b, bytes: y },
        ) => normalized_path(a) == normalized_path(b) && x == y,
        (EffectPreview::Read { path: a }, EffectPreview::Read { path: b }) => {
            normalized_path(a) == normalized_path(b)
        }
        (
            EffectPreview::Append { path: a, bytes: x },
            EffectPreview::Append { path: b, bytes: y },
        ) => normalized_path(a) == normalized_path(b) && x == y,
        (EffectPreview::List { path: a }, EffectPreview::List { path: b }) => {
            normalized_path(a) == normalized_path(b)
        }
        (EffectPreview::Fetch { url: a }, EffectPreview::Fetch { url: b }) => a == b,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowDiff {
    pub predicted: Vec<EffectPreview>,
    pub actual: Vec<EffectPreview>,
    pub r#match: bool,
}

/// Compare the dry-run's promises against the shadow run's actual effects.
pub fn verify_shadow(predicted: &[EffectPreview], actual: &[EffectPreview]) -> ShadowDiff {
    let r#match = predicted.len() == actual.len()
        && predicted
            .iter()
            .zip(actual)
            .all(|(p, a)| preview_matches(p, a));
    ShadowDiff {
        predicted: predicted.to_vec(),
        actual: actual.to_vec(),
        r#match,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v0::compile_rules;

    #[test]
    fn templates_for_every_form_verb() {
        let echo = compile_rules("Echo this message: ping").unwrap();
        assert_eq!(
            build_template(&echo).unwrap(),
            FormTemplate {
                template: "Echo this message: {{slot}}".into(),
                slot: "description".into(),
            }
        );
        let write = compile_rules("Write file notes.txt with contents hi").unwrap();
        assert_eq!(
            build_template(&write).unwrap().template,
            "Write file notes.txt with contents {{slot}}"
        );
        let read = compile_rules("Read file notes.txt").unwrap();
        assert_eq!(
            build_template(&read).unwrap().template,
            "Read file {{slot}}"
        );
        let list = compile_rules("List files under notes_dir").unwrap();
        assert_eq!(
            build_template(&list).unwrap().template,
            "List files under {{slot}}"
        );
    }

    #[test]
    fn fetch_cannot_become_a_form() {
        let fetch = compile_rules("Fetch https://api.example.com/x").unwrap();
        assert!(build_template(&fetch).is_err());
    }

    #[test]
    fn render_substitutes_and_compiles_back() {
        let write = compile_rules("Write file notes.txt with contents hi").unwrap();
        let t = build_template(&write).unwrap();
        let canonical = render_template(&t.template, "new contents").unwrap();
        assert_eq!(canonical, "Write file notes.txt with contents new contents");
        let recompiled = compile_rules(&canonical).unwrap();
        assert_eq!(recompiled.tasks[0].description, "new contents");
        assert_eq!(
            recompiled.tasks[0].target_path.as_deref(),
            Some("scratch/notes.txt")
        );
    }

    #[test]
    fn shadow_diff_normalizes_the_redirect() {
        let predicted = vec![EffectPreview::Write {
            path: "scratch/notes.txt".into(),
            bytes: 5,
        }];
        let actual = vec![EffectPreview::Write {
            path: "scratch/shadow/notes.txt".into(),
            bytes: 5,
        }];
        let diff = verify_shadow(&predicted, &actual);
        assert!(diff.r#match);
        // Different bytes → no match.
        let actual = vec![EffectPreview::Write {
            path: "scratch/shadow/notes.txt".into(),
            bytes: 9,
        }];
        assert!(!verify_shadow(&predicted, &actual).r#match);
        // Kind mismatch → no match.
        let actual = vec![EffectPreview::Read {
            path: "scratch/shadow/notes.txt".into(),
        }];
        assert!(!verify_shadow(&predicted, &actual).r#match);
    }
}
