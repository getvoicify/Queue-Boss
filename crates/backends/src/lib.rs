//! Queue Boss backend adapters.
//!
//! `SandboxBackend` is the in-memory synthetic queue (C4); `PgBossBackend`
//! arrives in E2. Both satisfy `qb_core::conformance::assert_backend_conforms`.

pub mod sandbox;
mod simulator;

pub use sandbox::SandboxBackend;
