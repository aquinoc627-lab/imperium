//! Event Store
//! 
//! SQLite-backed append-only event store with projections and snapshots.

pub mod event_store;
pub mod projections;
pub mod snapshots;
pub mod migrations;

pub use event_store::*;
pub use projections::*;
pub use snapshots::*;