//! Phase 18 — secret binding. The HMAC token secret is stored behind an
//! injectable [`SecretStore`]: the default is the legacy file backend; the
//! opt-in keychain backend binds it to the macOS Keychain via the `security`
//! CLI (no SDK). TPM sealing is out of scope (see specs/18-secret-binding.md).

use anyhow::{anyhow, bail, Result};
use std::path::{Path, PathBuf};

/// The active storage backend, resolved from `IMPERIUM_SECRET_BACKEND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    File,
    Keychain,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Keychain => "keychain",
        }
    }
}

/// Fail-closed resolution from an explicit choice string: `file` selects the
/// file backend, the literal `keychain` selects the keychain, anything else
/// is an error.
pub fn resolve_backend_from(choice: Option<&str>) -> Result<Backend> {
    match choice {
        None => Ok(Backend::File),
        Some("file") => Ok(Backend::File),
        Some("keychain") => Ok(Backend::Keychain),
        Some(other) => Err(anyhow!(
            "secret backend must be `file` or `keychain`, got `{other}` (fail-closed)"
        )),
    }
}

/// Full resolution: `IMPERIUM_SECRET_BACKEND` env var wins; else the marker
/// file written by `imperium secret bind`; else the file default.
pub fn resolve_backend(home: &Path) -> Result<Backend> {
    if let Ok(v) = std::env::var("IMPERIUM_SECRET_BACKEND") {
        return resolve_backend_from(Some(&v));
    }
    let marker = home.join("secret.backend");
    let choice = std::fs::read_to_string(&marker)
        .ok()
        .map(|s| s.trim().to_string());
    resolve_backend_from(choice.as_deref())
}

/// Read/erase a secret value. Implementations must never log the value.
pub trait SecretStore {
    fn get(&self) -> Result<Option<String>>;
    fn put(&self, value: &str) -> Result<()>;
    fn delete(&self) -> Result<()>;
    fn label(&self) -> &'static str;
}

// ---------------------------------------------------------------- file

pub struct FileStore {
    pub path: PathBuf,
}

impl FileStore {
    pub fn new(home: &Path) -> Self {
        Self {
            path: home.join("token.secret"),
        }
    }
}

impl SecretStore for FileStore {
    fn get(&self) -> Result<Option<String>> {
        if !self.path.exists() {
            return Ok(None);
        }
        Ok(Some(
            std::fs::read_to_string(&self.path)?.trim().to_string(),
        ))
    }

    fn put(&self, value: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(std::fs::write(&self.path, value)?)
    }

    fn delete(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn label(&self) -> &'static str {
        "file"
    }
}

// ------------------------------------------------------------- keychain

/// Pure construction of the `security` argv — unit-tested without executing.
/// Service is fixed; the account is the canonicalized home path so multiple
/// homes never share a secret.
pub fn keychain_args(op: &str, account: &str, value: Option<&str>) -> Vec<String> {
    let mut argv = vec![
        "security".to_string(),
        op.to_string(),
        "-s".to_string(),
        "imperium".to_string(),
        "-a".to_string(),
        account.to_string(),
    ];
    match op {
        "find-generic-password" => argv.push("-w".to_string()),
        "add-generic-password" => {
            argv.push("-w".to_string());
            argv.push(value.expect("add requires a value").to_string());
            argv.push("-U".to_string());
        }
        "delete-generic-password" => {}
        other => unreachable!("unsupported security op: {other}"),
    }
    argv
}

fn run_security(argv: &[String]) -> Result<Option<String>> {
    use std::process::{Command, Stdio};
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    if out.status.success() {
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return Ok(if text.is_empty() { None } else { Some(text) });
    }
    // 44 = item not found (secItemNotFound); anything else is a real failure.
    if let Some(code) = out.status.code() {
        if code == 44 {
            return Ok(None);
        }
    }
    bail!("security exited with {:?}", out.status.code())
}

pub struct KeychainStore {
    pub account: String,
}

impl KeychainStore {
    pub fn new(home: &Path) -> Self {
        let account = home
            .canonicalize()
            .unwrap_or_else(|_| home.to_path_buf())
            .display()
            .to_string();
        Self { account }
    }

    fn run(&self, op: &str, value: Option<&str>) -> Result<Option<String>> {
        run_security(&keychain_args(op, &self.account, value))
    }
}

impl SecretStore for KeychainStore {
    fn get(&self) -> Result<Option<String>> {
        self.run("find-generic-password", None)
    }

    fn put(&self, value: &str) -> Result<()> {
        if !cfg!(target_os = "macos") {
            bail!("keychain backend requires macOS (`security` CLI)");
        }
        self.run("add-generic-password", Some(value))?;
        Ok(())
    }

    fn delete(&self) -> Result<()> {
        // A missing item is already the desired state.
        let _ = self.run("delete-generic-password", None)?;
        Ok(())
    }

    fn label(&self) -> &'static str {
        "keychain"
    }
}

// -------------------------------------------------------------- memory

/// Test-only in-memory store (never the default resolution target).
#[cfg(test)]
pub struct MemoryStore {
    value: std::sync::Mutex<Option<String>>,
}

#[cfg(test)]
impl MemoryStore {
    pub fn new() -> Self {
        Self {
            value: std::sync::Mutex::new(None),
        }
    }
}

#[cfg(test)]
impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl SecretStore for MemoryStore {
    fn get(&self) -> Result<Option<String>> {
        Ok(self.value.lock().expect("lock").clone())
    }
    fn put(&self, value: &str) -> Result<()> {
        *self.value.lock().expect("lock") = Some(value.to_string());
        Ok(())
    }
    fn delete(&self) -> Result<()> {
        *self.value.lock().expect("lock") = None;
        Ok(())
    }
    fn label(&self) -> &'static str {
        "memory"
    }
}

// ---------------------------------------------------------- generation

/// Fresh secret: two UUIDv4 simple values concatenated — 64 hex chars,
/// ≈256 bits, dependency-free. Any string is a valid HMAC key.
pub fn generate_secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}


/// Short, display-safe identifier for `secret status`.
pub fn fingerprint(secret: &str) -> String {
    secret.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_resolution_is_fail_closed() {
        assert_eq!(resolve_backend_from(None).unwrap(), Backend::File);
        assert_eq!(resolve_backend_from(Some("file")).unwrap(), Backend::File);
        assert_eq!(
            resolve_backend_from(Some("keychain")).unwrap(),
            Backend::Keychain
        );
        assert!(resolve_backend_from(Some("garbage")).is_err());
        assert!(resolve_backend_from(Some("")).is_err());
    }

    #[test]
    fn marker_file_is_honored() {
        let dir = std::env::temp_dir().join(format!("imp-res-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        // No marker → file default.
        assert_eq!(resolve_backend(&dir).unwrap(), Backend::File);

        // Marker → honored.
        std::fs::write(dir.join("secret.backend"), "keychain\n").unwrap();
        assert_eq!(resolve_backend(&dir).unwrap(), Backend::Keychain);

        // Garbage marker → fail closed.
        std::fs::write(dir.join("secret.backend"), "garbage").unwrap();
        assert!(resolve_backend(&dir).is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keychain_argv_matches_spec_exactly() {
        let acct = "/tmp/h1";
        assert_eq!(
            keychain_args("find-generic-password", acct, None),
            vec![
                "security",
                "find-generic-password",
                "-s",
                "imperium",
                "-a",
                acct,
                "-w"
            ]
        );
        assert_eq!(
            keychain_args("add-generic-password", acct, Some("abc")),
            vec![
                "security",
                "add-generic-password",
                "-s",
                "imperium",
                "-a",
                acct,
                "-w",
                "abc",
                "-U"
            ]
        );
        assert_eq!(
            keychain_args("delete-generic-password", acct, None),
            vec!["security", "delete-generic-password", "-s", "imperium", "-a", acct]
        );
    }

    #[test]
    fn fresh_secrets_are_64_hex_chars() {
        let s = generate_secret();
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(s, generate_secret());
    }

    #[test]
    fn file_store_roundtrip_and_delete_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("imp-secret-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = FileStore::new(&dir);
        assert_eq!(store.get().unwrap(), None);
        store.put("value-1").unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some("value-1"));
        store.delete().unwrap();
        store.delete().unwrap(); // idempotent
        assert_eq!(store.get().unwrap(), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn memory_store_supports_the_migration_flow() {
        let store = MemoryStore::new();
        assert_eq!(store.get().unwrap(), None);
        store.put("legacy").unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some("legacy"));
        store.delete().unwrap();
        assert_eq!(store.get().unwrap(), None);
    }

    #[test]
    fn fingerprint_is_stable_prefix() {
        let s = generate_secret();
        assert_eq!(fingerprint(&s), s[..8].to_string());
    }
}

