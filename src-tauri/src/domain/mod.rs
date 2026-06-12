//! Domain layer - Business entities and logic
//!
//! This module contains pure business logic and domain entities,
//! independent of framework and persistence details.

pub mod clipboard;
pub mod clipboard_format;
pub mod event;

pub use clipboard_format::ClipboardFormat;
pub use event::EventRecord;
