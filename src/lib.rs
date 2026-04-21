//! Terminal Pomodoro core library.
//!
//! This crate exposes the reusable core of `termodoro`:
//! configuration handling, phase/domain logic, timer engine utilities,
//! and persistence helpers.
//!
//! The CLI binary uses the same modules, and `docs.rs` renders this library API.

/// User configuration model and validation utilities.
pub mod config;
/// Pomodoro phase types and phase transition rules.
pub mod domain;
/// Timer engine helpers and interactive runner.
pub mod engine;
/// Configuration/state/history persistence and process helpers.
pub mod persistence;
