pub mod database;
pub mod schema;
pub mod writer;

use thiserror::Error;

pub use database::Database;
pub use writer::EventWriter;

/// Storage-related errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("PCAP error: {0}")]
    PcapError(#[from] pcap::Error),

    #[error("Schema initialization failed: {0}")]
    SchemaError(String),

    #[error("Event insertion failed: {0}")]
    EventInsertionError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
