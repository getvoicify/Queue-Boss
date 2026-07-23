//! Queue Boss backend adapters.
//!
//! `SandboxBackend` is the in-memory synthetic queue (C4); `PgBossBackend` is
//! the read-only pg-boss v10 adapter (E2). Both satisfy the shared
//! `qb_core::conformance` suites.

pub mod pgboss;
pub mod sandbox;
mod simulator;

pub use pgboss::PgBossBackend;
pub use sandbox::SandboxBackend;
