// Allow some clippy lints for initial stub implementation
#![allow(clippy::multiple_crate_versions)] // TODO: Resolve dependency conflicts in Phase 1+

pub mod audio;
pub mod config;
pub mod error;
pub mod input;
pub mod ipc;
pub mod notifications;
pub mod transcription;
pub mod tray;

pub use error::{Result, ScribeError};
