//! Cryptographic primitives used by core types.
//!
//! Full key management lives in `imperium-crypto`. This module is the
//! small surface `imperium-core` needs so the IR crate compiles.

use serde::{Deserialize, Serialize};
use std::fmt;

/// BLAKE3 hash output (32 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub fn blake3(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self([0u8; 32])
    }
}
