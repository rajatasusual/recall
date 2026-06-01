//! Domain layer - Business entities and logic
//!
//! This module contains pure business logic and domain entities,
//! independent of framework and persistence details.

pub mod clipboard;
pub mod event;

pub use event::EventRecord;
