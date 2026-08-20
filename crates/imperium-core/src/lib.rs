pub mod intent;
pub mod event;
pub mod capability;
pub mod policy;
pub mod crypto;
pub mod error;
pub mod v0;

pub use intent::*;
pub use event::*;
pub use capability::*;
pub use policy::*;
pub use crypto::*;
pub use error::*;

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Magic bytes for IMPERIUM data files
pub const MAGIC_BYTES: &[u8; 8] = b"IMPERIUM";
