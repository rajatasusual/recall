Architecture — Recall (clipboard)

Overview

Recall is a Tauri desktop app with a Rust backend and a Preact frontend. The backend captures clipboard events, batches and stores them in SQLite, and exposes query and mutation commands to the UI via Tauri invokes.

High-level components

- Frontend (Preact/TypeScript)
  - `src/components/EventTimeline.tsx` — main UI: lists events, supports pin/unpin, delete, copy, and filtering by pinned or source app.
  - `src/styles/EventTimeline.css` — styling for the timeline and items.
  - `src/types.ts` — shared type definitions for the frontend.

- Backend (Rust/Tauri)
  - `core` — `Event` struct and `EventPayload` enum. Key fields: `id`, `timestamp`, `source`, `payload`, `window_title`, `source_app`, `pinned`.
  - `sources` — clipboard watcher (`sources/clipboard.rs`) polls the system clipboard, computes an MD5 `content_hash`, captures context (frontmost app and window title on macOS via `osascript`), and submits events to `EventWriter`.
  - `storage` — DB layer (`db.rs`) and schema (`schema.rs`) manage `events` and `edges` tables. `EventWriter` batches writes for efficiency and creates temporal edges between consecutive events.
  - `commands` — Tauri-invokable handlers that map to storage operations (get, filter, pin/unpin, delete).

Database schema (events table)

Columns added/used:
- `id` TEXT PRIMARY KEY (UUID)
- `timestamp` INTEGER (ms since epoch)
- `source` TEXT (e.g., "clipboard")
- `payload_type` TEXT
- `payload_data` TEXT (JSON serialized payload)
- `window_title` TEXT
- `source_app` TEXT
- `content_hash` TEXT (MD5 of text content)
- `pinned` INTEGER (0/1)
- `created_at` INTEGER (insert time ms)

Indexes:
- `idx_events_timestamp` — for sorting by time
- `idx_events_content_hash` — for fast duplicate lookup
- `idx_events_pinned` — to filter/order pinned items
- `idx_events_source_app` — to filter by app

Event flow

1. Clipboard watcher polls clipboard (default 400ms).
2. If text found, compute MD5 `content_hash` and perform a quick compare with the last-seen hash to avoid rapid duplicates.
3. Query DB via `content_exists(content_hash)` — if exists, skip storing; otherwise create an `Event` with captured context and queue it to `EventWriter`.
4. `EventWriter` batches writes and flushes them into SQLite; it also inserts simple `temporal_next` edges linking consecutive events.
5. Frontend invokes Tauri commands to list, filter, pin/unpin, delete, and restore clipboard entries.

Deduplication rules

- In-memory: `last_hash` prevents immediate duplicates on the same runtime loop.
- Persistent: `content_hash` prevents inserting duplicates already stored in DB (useful across restarts).
- Hashing is MD5 over the trimmed text content. (You can swap to SHA256 if desired.)

Pinning & Ordering

- `pinned` is stored per event; queries order by `pinned DESC, timestamp DESC` so pinned items appear at the top.
- UI supports filtering to show only pinned items.

Commands (summary)

- `get_events(pinned_only?, source_app?)`
- `get_all_events()`
- `get_events_by_timestamp_range(start, end)`
- `get_pinned_events()`
- `pin_event(id)`, `unpin_event(id)`
- `delete_event(id)`
- `get_event_count()`

Notes / Future work

- Improve app/window capture for cross-platform behavior (accessibility APIs, platform-specific crates).
- Add search, fuzzy match, and categorisation of clipboard content.
- Add soft-delete / trash and confirm deletion flow.
- Consider richer payload serialization (images/binary) and thumbnailing.

