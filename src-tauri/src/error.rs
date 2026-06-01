use serde::Serialize;
use std::fmt;

/// Unified application error type
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}{}",
            self.code,
            self.message,
            self.details
                .as_ref()
                .map(|d| format!(" ({})", d))
                .unwrap_or_default()
        )
    }
}

impl std::error::Error for AppError {}

impl From<crate::persistence::StorageError> for AppError {
    fn from(err: crate::persistence::StorageError) -> Self {
        match err {
            crate::persistence::StorageError::DatabaseError(db_err) => {
                AppError {
                    code: "DB_ERROR".to_string(),
                    message: "Database operation failed".to_string(),
                    details: Some(db_err.to_string()),
                }
            }
            crate::persistence::StorageError::SerializationError(serde_err) => {
                AppError {
                    code: "SERIALIZATION_ERROR".to_string(),
                    message: "Failed to serialize/deserialize data".to_string(),
                    details: Some(serde_err.to_string()),
                }
            }
            crate::persistence::StorageError::IoError(io_err) => {
                AppError {
                    code: "IO_ERROR".to_string(),
                    message: "IO operation failed".to_string(),
                    details: Some(io_err.to_string()),
                }
            }
            crate::persistence::StorageError::PcapError(pcap_err) => {
                AppError {
                    code: "PCAP_ERROR".to_string(),
                    message: "PCAP operation failed".to_string(),
                    details: Some(pcap_err.to_string()),
                }
            }
            crate::persistence::StorageError::SchemaError(msg) => {
                AppError {
                    code: "SCHEMA_ERROR".to_string(),
                    message: "Schema initialization failed".to_string(),
                    details: Some(msg),
                }
            }
            crate::persistence::StorageError::EventInsertionError(msg) => {
                AppError {
                    code: "INSERT_ERROR".to_string(),
                    message: "Failed to insert event".to_string(),
                    details: Some(msg),
                }
            }
        }
    }
}

/// Standard result type for application operations
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_serialization() {
        let err = AppError {
            code: "TEST_ERROR".to_string(),
            message: "Test message".to_string(),
            details: Some("Test details".to_string()),
        };

        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\":\"TEST_ERROR\""));
        assert!(json.contains("\"message\":\"Test message\""));
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError {
            code: "TEST".to_string(),
            message: "msg".to_string(),
            details: None,
        };
        assert_eq!(err.to_string(), "TEST: msg");
    }
}
