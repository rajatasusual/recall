# Architecture — Recall (clipboard)

## Overview

Recall is a Tauri desktop app with a Rust backend and a Preact frontend. The backend captures clipboard events (text and images), batches and stores them in SQLite with binary blob support, and exposes query and mutation commands to the UI via Tauri invokes. Image content is deduplicated by content hash, encoded to PNG with preview thumbnails, and stored separately in a `blobs` table for efficient retrieval.

## High-level components

- Frontend (Preact/TypeScript)
  - `src/components/EventTimeline.tsx` — main UI: lists events, supports pin/unpin, delete, copy, and filtering by pinned or source app.
  - `src/styles/EventTimeline.css` — styling for the timeline and items.
  - `src/types.ts` — shared type definitions for the frontend.

- Backend (Rust/Tauri)
  - `core` — `Event` struct and `EventPayload` enum. `EventPayload` supports `ClipboardText` and `ClipboardImage` variants. Key event fields: `id`, `timestamp`, `source`, `payload`, `window_title`, `source_app`, `pinned`.
  - `sources` — clipboard watcher (`sources/clipboard.rs`) polls the system clipboard using `arboard`. Refactored with helper functions (`process_text`, `process_image`, `emit_content`) for code reuse:
    - Text: computes xxHash64 `content_hash`, handles truncation if >50KB.
    - Image: encodes PNG, generates base64 data URL preview (max 512×512px), computes hash of full PNG bytes.
    - Both: deduplicated via `content_exists()` check before emission.
    - Captures context (frontmost app and window title on macOS via `osascript`).
  - `storage` — DB layer (`db.rs`) and schema (`schema.rs`) manage `events`, `blobs`, and `edges` tables. `EventWriter` batches writes for efficiency and creates temporal edges between consecutive events. Blob insertion is best-effort to avoid blocking event writes.
  - `commands` — Tauri-invokable handlers that map to storage operations (get, filter, pin/unpin, delete).
  - `lib.rs` — application bootstrap and tray menu wiring. The tray menu now exposes the last 10 pinned clipboard items and restores clipboard contents using native clipboard copy; image restore reads stored PNG blobs and writes native image data via `arboard`.

## Database schema

**events table**

Columns:
- `id` TEXT PRIMARY KEY (UUID)
- `timestamp` INTEGER (ms since epoch)
- `source` TEXT (e.g., "clipboard")
- `payload_type` TEXT (`clipboard_text` or `clipboard_image`)
- `payload_data` TEXT (JSON serialized payload, includes preview URL for images but not raw binary)
- `window_title` TEXT
- `source_app` TEXT
- `content_hash` TEXT (xxHash64 of content)
- `pinned` INTEGER (0/1)
- `created_at` INTEGER (insert time ms)

Indexes:
- `idx_events_timestamp` — for sorting by time
- `idx_events_content_hash` — for fast duplicate lookup
- `idx_events_pinned` — to filter/order pinned items
- `idx_events_source_app` — to filter by app

**blobs table**

Stores binary image data referenced by content hash (allows events to be lightweight JSON while preserving full-quality images):

Columns:
- `content_hash` TEXT PRIMARY KEY (xxHash64 of image PNG bytes)
- `mime` TEXT (e.g., "image/png")
- `data` BLOB (raw PNG bytes)
- `created_at` INTEGER (insert time ms)

Indexes:
- `idx_blobs_content_hash` — for fast retrieval

**edges table**

Temporal relationships between consecutive events (optional graph layer for future analytics):

Columns:
- `from_id` TEXT (event id)
- `to_id` TEXT (event id)
- `relation_type` TEXT (e.g., "temporal_next")
- PRIMARY KEY (from_id, to_id, relation_type)

## Event flow

1. **Clipboard polling** (default 150ms interval) attempts to read clipboard:
   - If text is found (non-empty after trim):
     - Compute xxHash64 `content_hash` and compare with last-seen hash to avoid rapid duplicates.
     - Query `content_exists()` — if true, skip storing; otherwise proceed to step 3.
   - If text is empty:
     - Attempt to read image data.
     - If image found, encode to PNG, generate preview (max 512×512px), compute hash of full PNG bytes, and proceed to step 3.
     - If no image, sleep and continue polling.

2. **Deduplication & event creation**:
   - Via helper functions `process_text()` and `process_image()`, create a `ClipboardContent` enum (text or image variant).
   - Call `emit_content()` which checks `content_exists()` and, if the content is new or on dedup failure, creates an `Event` from the `ClipboardContent` and queues it to `EventWriter`.

3. **Batched persistence**:
   - `EventWriter` accumulates events in a queue, flushes when batch size is reached or on timer tick.
   - For each event, insert into `events` table; if payload is `ClipboardImage` and has raw PNG bytes, also insert into `blobs` table (best-effort, warnings on failure).
   - Emit `events:new` to the frontend on successful inserts.

4. **Frontend realtime merge**:
   - Frontend listens for `events:new` and merges incoming events into local state, avoiding a full list refresh.

5. **User interactions** (frontend invokes):
   - `get_events()` — list & filter events.
   - `pin_event()` / `unpin_event()` — manage pinning.
   - `delete_event()` — remove an event.
   - (Copy-to-clipboard is handled client-side.)

## Deduplication

- **In-memory (text only)**: `last_hash` prevents rapid duplicate text entries on the same polling loop.
- **Persistent (text & image)**: `content_hash` in the `events` table and `blobs` table prevent inserting duplicates already stored in DB (useful across restarts and multiple clipboard changes).
- **Hashing**: 
  - **Text**: xxHash64 over the trimmed text content.
  - **Image**: xxHash64 over the full PNG-encoded bytes (not the preview).
- **Best-effort dedup**: `emit_content()` checks `content_exists()` and emits regardless of success/failure to avoid losing events on transient DB errors.

## Commands (summary)

- `get_events(pinned_only?, source_app?)`
- `get_all_events()`
- `get_events_by_timestamp_range(start, end)`
- `get_pinned_events()`
- `pin_event(id)`, `unpin_event(id)`
- `delete_event(id)`
- `get_event_count()`

## Notes / Future work

- **Code reuse**: Image and text handling share a unified `ClipboardContent` enum and common `emit_content()` deduplication logic; new payload types can be added by extending the enum.
- **Refactoring**: `process_text()`, `process_image()`, and `emit_content()` helper functions keep the main polling loop lean and testable.
- **Cross-platform**: Improve app/window capture for Linux and Windows (accessibility APIs, platform-specific crates).
- **Search & categorization**: Add full-text search, fuzzy match, and tag extraction.
- **Soft-delete**: Add trash / undo flow and confirm deletion.
- **Blob retrieval**: Add endpoint for fetching full-quality images (currently preview is in event, full PNG is in blobs table).
- **Image-to-clipboard restore**: Tray menu image restore now supports native image copy from stored PNG blobs; frontend timeline image copy remains an area for further improvement.

