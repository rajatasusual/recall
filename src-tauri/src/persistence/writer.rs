use crate::config::WriterConfig;
use crate::core::Event;
use crate::domain::EventRecord;
use crate::persistence::{Database, StorageResult};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::{interval, Duration};

/// Event writer with batching and backpressure
#[derive(Clone)]
pub struct EventWriter {
    #[allow(dead_code)]
    db: Arc<Database>,
    sender: mpsc::Sender<Event>,
    #[allow(dead_code)]
    config: WriterConfig,
    receiver: Arc<Mutex<Option<mpsc::Receiver<Event>>>>,
    #[allow(dead_code)]
    emitter: Option<AppHandle>,
}

impl EventWriter {
    /// Create a new event writer (without spawning the task yet)
    pub fn new(db: Arc<Database>, config: WriterConfig, emitter: Option<AppHandle>) -> Self {
        let (sender, receiver) = mpsc::channel(config.max_queue_size);

        Self {
            db,
            sender,
            config,
            receiver: Arc::new(Mutex::new(Some(receiver))),
            emitter,
        }
    }

    /// Spawn the writer task (must be called from within a Tokio runtime context)
    pub fn spawn_task(&self) -> StorageResult<()> {
        let mut receiver_lock = self.receiver.lock().unwrap();

        // Only spawn if receiver is still available
        if let Some(receiver) = receiver_lock.take() {
            let db_clone = Arc::clone(&self.db);
            let config_clone = self.config.clone();
            let emitter_clone = self.emitter.clone();

            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    Self::writer_task(db_clone, receiver, config_clone, emitter_clone).await
                {
                    tracing::error!("Event writer task failed: {}", e);
                }
            });

            tracing::info!("Event writer task spawned");
            Ok(())
        } else {
            tracing::info!("Event writer task already spawned");
            Ok(())
        }
    }

    /// Submit an event for writing (non-blocking)
    pub fn write_event(&self, event: Event) -> StorageResult<()> {
        if self.sender.is_closed() {
            return Err(crate::persistence::StorageError::EventInsertionError(
                "Event writer channel closed".to_string(),
            ));
        }

        match self.sender.try_send(event.clone()) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full(event)) => {
                tracing::warn!(
                    "event_dropped: event_id={}, source={}, reason=queue_full",
                    event.id,
                    event.source.as_str()
                );
                Err(crate::persistence::StorageError::EventInsertionError(
                    format!("Event queue full, dropped event {}", event.id),
                ))
            }
            Err(TrySendError::Closed(event)) => {
                tracing::warn!("Failed to queue event {}: channel closed", event.id);
                Err(crate::persistence::StorageError::EventInsertionError(
                    format!("Failed to queue event {}: channel closed", event.id),
                ))
            }
        }
    }

    /// Check whether content hash already exists in DB
    pub fn content_exists(&self, content_hash: &str) -> StorageResult<bool> {
        self.db.content_exists(content_hash)
    }

    /// The batch writer task - runs indefinitely
    async fn writer_task(
        db: Arc<Database>,
        mut receiver: mpsc::Receiver<Event>,
        config: WriterConfig,
        emitter: Option<AppHandle>,
    ) -> StorageResult<()> {
        let mut batch = Vec::with_capacity(config.batch_size);
        let mut flush_interval = interval(Duration::from_millis(config.flush_interval_ms));

        loop {
            tokio::select! {
                // Receive new events
                Some(event) = receiver.recv() => {
                    batch.push(event);

                    if batch.len() >= config.batch_size {
                        Self::flush_batch(&db, &mut batch, emitter.clone()).await?;
                    }
                }

                // Time-based flush
                _ = flush_interval.tick() => {
                        if !batch.is_empty() {
                        Self::flush_batch(&db, &mut batch, emitter.clone()).await?;
                    }
                }

                // Channel closed
                else => {
                    // Flush any remaining events
                    if !batch.is_empty() {
                        Self::flush_batch(&db, &mut batch, emitter.clone()).await?;
                    }
                    tracing::info!("Event writer task shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Flush a batch of events to database
    async fn flush_batch(
        db: &Arc<Database>,
        batch: &mut Vec<Event>,
        emitter: Option<AppHandle>,
    ) -> StorageResult<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let batch_size = batch.len();
        let start = std::time::Instant::now();

        // Insert events
        for event in batch.iter() {
            Self::insert_event_record(db, event)?;
            // emit to frontend listeners if available
            if let Some(ref app) = emitter {
                // best-effort emit; ignore errors
                let _ = app.emit(
                    "events:new",
                    serde_json::to_value(event).unwrap_or(serde_json::json!({"id": event.id})),
                );
            }
        }

        // Create temporal edges
        for i in 1..batch.len() {
            let from_id = &batch[i - 1].id;
            let to_id = &batch[i].id;
            db.insert_edge(from_id, to_id, "temporal_next")?;
        }

        let elapsed = start.elapsed().as_millis();
        tracing::info!(
            "db_write_batch_complete: batch_size={}, elapsed_ms={}",
            batch_size,
            elapsed
        );

        batch.clear();
        Ok(())
    }

    /// Insert a single event record into database
    fn insert_event_record(db: &Arc<Database>, event: &Event) -> StorageResult<()> {
        let payload_data = serde_json::to_string(&event.payload)?;

        // Extract content_hash from payload and persist any binary blobs
        let content_hash = match &event.payload {
            crate::core::EventPayload::ClipboardText { content_hash, .. } => {
                Some(content_hash.as_str())
            }
            crate::core::EventPayload::ClipboardImage {
                content_hash,
                mime,
                data,
                ..
            } => {
                if let Some(bytes) = data {
                    // persist blob (best-effort)
                    if let Err(e) = db.insert_blob(content_hash, mime, bytes) {
                        tracing::warn!("failed_to_insert_blob: {}", e);
                    }
                }
                Some(content_hash.as_str())
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let event_record = EventRecord {
            id: event.id.clone(),
            timestamp: event.timestamp,
            source: event.source.as_str().to_string(),
            payload_type: event.payload.payload_type().to_string(),
            payload_data,
            window_title: event.window_title.clone(),
            source_app: event.source_app.clone(),
            content_hash: content_hash.map(str::to_string),
            pinned: false,
            created_at: now,
        };

        db.insert_event(&event_record)?;

        tracing::debug!(
            "event_ingested: event_id={}, source={}",
            event.id,
            event.source.as_str()
        );
        Ok(())
    }
}
