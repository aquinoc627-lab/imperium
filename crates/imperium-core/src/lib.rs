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
