Recall — Intelligent Clipboard

Brief

Recall captures clipboard items, stores them locally, and provides a small Tauri + Preact UI to browse, pin, filter and restore clipboard entries.

Quick start (development)

- Install dependencies and start the web UI/dev server:

```bash
npm install
npm run dev
```

- Start the Tauri dev runtime (from project root):

```bash
npm run tauri dev
```

Backend (Tauri/Rust)

- Database file: stored in the platform app data directory as `events.db` (SQLite, WAL mode).
- Main backend modules:
  - `core` — event model (`Event`), payloads and helpers
  - `sources` — clipboard polling with `arboard` and capture; runs in a fast, non-blocking background task and captures app/window context on macOS
  - `storage` — DB wrapper, schema, and batched EventWriter; emits `events:new` to the frontend on new inserts
  - `commands` — Tauri command handlers invoked by the frontend

Available Tauri commands (invoke)

- `get_events(pinned_only?: boolean, source_app?: string)` — returns events, filters optional
- `get_all_events()` — returns recent events (legacy)
- `get_events_by_timestamp_range(start_ms, end_ms)`
- `get_pinned_events()`
- `pin_event(event_id)` / `unpin_event(event_id)`
- `delete_event(event_id)`
- `get_event_count()`
- `test_insert_clipboard_event(content)` — test helper

Realtime event flow:
- frontend listens for `events:new` and merges newly ingested clipboard events into state without a full refresh

Frontend (src)

- Preact + TypeScript UI lives in `src/` and components are under `src/components/`.
- `EventTimeline` is the main view: supports filtering by pinned and by application, pin/unpin, delete, copy back to clipboard, and shows a small toast on copy.

Design notes

- Deduplication: clipboard text is hashed (xxHash64) and stored as `content_hash` in the DB; duplicate prevention happens both in-memory (last seen hash) and by consulting the DB before inserting.
- Polling: system clipboard polling uses `arboard` inside `tokio::task::spawn_blocking(...)` with a `tokio::time::interval` ticker so clipboard reads stay fast and non-blocking.
- Pinning: `pinned` is a boolean column; pinned items are ordered to the top in queries.
- Context: `source_app` and `window_title` (macOS) are captured per event where available.

Where to look

- Backend: `src-tauri/src/`
- Frontend: `src/components/EventTimeline.tsx`, `src/styles/EventTimeline.css`
- DB schema: `src-tauri/src/storage/schema.rs`

Next steps (ideas)

- Add soft-delete / trash with undo
- Add search and tag extraction
- Add richer content types (images, files)

License

- (Add your license here)
