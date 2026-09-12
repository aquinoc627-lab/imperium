pub mod capability;
pub mod crypto;
pub mod error;
pub mod event;
pub mod forms;
pub mod intent;
pub mod policy;
pub mod synth;
pub mod v0;

pub use capability::*;
pub use crypto::*;
pub use error::*;
pub use event::*;
pub use intent::*;
pub use policy::*;

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 2;

/// Protocol versions accepted by validation (v1 is legacy, read-only).
pub const ACCEPTED_PROTOCOL_VERSIONS: &[u32] = &[1, 2];

/// Magic bytes for IMPERIUM data files
pub const MAGIC_BYTES: &[u8; 8] = b"IMPERIUM";

// ----- Diff preview (Phase 15) -----

/// Produce a concise diff between old and new content.
///
/// Returns `+` lines for new content and `-` lines for old content that is
/// absent from the new version. Truncated to 20 lines with `... N more lines`
/// marker when the diff exceeds that length.
pub fn diff_preview(old: Option<&str>, new: &str) -> Vec<String> {
    let old_lines: Vec<&str> = old.map_or_else(Vec::new, |s| s.lines().collect());
    let new_lines: Vec<&str> = new.lines().collect::<Vec<&str>>();

    let old_set: std::collections::HashSet<&str> = old_lines.iter().copied().collect();
    let new_set: std::collections::HashSet<&str> = new_lines.iter().copied().collect();

    let only_old: Vec<&str> = old_lines
        .iter()
        .filter(|l| !new_set.contains(*l))
        .copied()
        .collect();
    let only_new: Vec<&str> = new_lines
        .iter()
        .filter(|l| !old_set.contains(*l))
        .copied()
        .collect();

    let old_len = only_old.len();
    let new_len = only_new.len();

    let mut lines: Vec<String> = only_old
        .into_iter()
        .map(|l| format!("- {}", l))
        .chain(only_new.into_iter().map(|l| format!("+ {}", l)))
        .collect();

    // Sort so - lines come before + lines for stable output
    lines.sort();

    let extra = if lines.len() > 20 {
        Some((old_len + new_len).saturating_sub(20))
    } else {
        None
    };
    let marker = extra.filter(|e| *e > 0).map(|e| {
        lines.truncate(20);
        format!("... {e} more lines")
    });

    if let Some(m) = marker {
        lines.push(m);
    }

    lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn diff_preview_never_underflows() {
        // Fewer than 20 total diff lines used to underflow (n - 20).
        assert_eq!(
            crate::diff_preview(Some("a\nb"), "b\nc"),
            vec!["+ c".to_string(), "- a".to_string()]
        );
        // Over 20 lines: truncated with a "N more lines" marker.
        let old: String = (0..30).map(|i| format!("old{i}\n")).collect();
        let new: String = (0..30).map(|i| format!("new{i}\n")).collect();
        let out = crate::diff_preview(Some(&old), &new);
        assert_eq!(out.len(), 21);
        assert!(out.last().unwrap().starts_with("... "));
        assert!(out.last().unwrap().ends_with("more lines"));
    }
}
