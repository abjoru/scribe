//! Configuration module for scribe
//!
//! Loads config from `$XDG_CONFIG_HOME/scribe/config.toml` or `~/.config/scribe/config.toml`.
//! Falls back to embedded defaults if file doesn't exist.
//! Partial configs are merged with defaults using serde's default attributes.
//!
//! # Example
//!
//! ```no_run
//! use scribe::config::Config;
//!
//! let config = Config::load().expect("Failed to load config");
//! println!("Sample rate: {}", config.audio.sample_rate);
//! println!("VAD aggressiveness: {}", config.vad.aggressiveness);
//! ```

pub mod schema;

pub use schema::Config;
