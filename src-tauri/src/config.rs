//! Centralized configuration module
//! 
//! Consolidates all hardcoded values into a single configuration structure
//! that can be overridden via environment variables or configuration files.

use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub clipboard: ClipboardConfig,
    pub storage: StorageConfig,
    pub writer: WriterConfig,
}

/// Application-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Enable debug logging
    pub debug: bool,
}

/// Clipboard monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    /// Poll interval for clipboard changes (milliseconds)
    pub poll_interval_ms: u64,
    /// Maximum clipboard payload size (bytes)
    pub max_payload_bytes: usize,
    /// Number of characters to keep for text preview
    pub preview_chars: usize,
    /// Maximum dimension for image previews (pixels)
    pub preview_max_dim: u32,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: env::var("CLIPBOARD_POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(150),
            max_payload_bytes: env::var("CLIPBOARD_MAX_PAYLOAD_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50 * 1024),
            preview_chars: env::var("CLIPBOARD_PREVIEW_CHARS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2048),
            preview_max_dim: env::var("CLIPBOARD_PREVIEW_MAX_DIM")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(512),
        }
    }
}

/// Storage (database) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// SQLite database path (relative or absolute)
    pub database_path: String,
    /// Enable WAL mode (crash-safe)
    pub enable_wal: bool,
    /// SQLite cache size in pages (negative = KB)
    pub cache_size: i32,
    /// Query result limit (max events per query)
    pub query_limit: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "recall.db".to_string()),
            enable_wal: true,
            cache_size: env::var("DB_CACHE_SIZE_KB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(-64000), // 64MB
            query_limit: env::var("DB_QUERY_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
        }
    }
}

/// Event writer (batching) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriterConfig {
    /// Events to batch before writing
    pub batch_size: usize,
    /// Flush interval (milliseconds)
    pub flush_interval_ms: u64,
    /// Maximum queue size before backpressure
    pub max_queue_size: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            batch_size: env::var("WRITER_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            flush_interval_ms: env::var("WRITER_FLUSH_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(200),
            max_queue_size: env::var("WRITER_MAX_QUEUE_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig {
                name: "Recall".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                debug: env::var("DEBUG")
                    .ok()
                    .map(|v| v.to_lowercase() == "true")
                    .unwrap_or(false),
            },
            clipboard: ClipboardConfig::default(),
            storage: StorageConfig::default(),
            writer: WriterConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from environment and defaults
    pub fn from_env() -> Self {
        Self::default()
    }

    /// Create configuration with custom values
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

/// Configuration builder for testing
pub struct ConfigBuilder {
    config: Config,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            config: Config::default(),
        }
    }
}

impl ConfigBuilder {
    pub fn clipboard_poll_interval(mut self, interval_ms: u64) -> Self {
        self.config.clipboard.poll_interval_ms = interval_ms;
        self
    }

    pub fn database_path(mut self, path: String) -> Self {
        self.config.storage.database_path = path;
        self
    }

    pub fn writer_batch_size(mut self, size: usize) -> Self {
        self.config.writer.batch_size = size;
        self
    }

    pub fn writer_flush_interval(mut self, ms: u64) -> Self {
        self.config.writer.flush_interval_ms = ms;
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

