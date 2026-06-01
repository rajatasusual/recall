# Recall — Intelligent Clipboard

## Brief

Recall captures clipboard items (text and images), stores them locally, and provides a small Tauri + Preact UI to browse, pin, filter and restore clipboard entries. Images are automatically encoded to PNG, deduplicated by content hash, and displayed with preview thumbnails in the timeline. A system tray menu now also exposes the last 10 pinned clipboard items for fast restore back to the active clipboard.

## Quick start (development)

- Install dependencies and start the web UI/dev server:

```bash
npm install
npm run dev
```

- Start the Tauri dev runtime (from project root):

```bash
npm run tauri dev
```

### Backend (Tauri/Rust)

- Database file: stored in the platform app data directory as `events.db` (SQLite, WAL mode).
- Main backend modules:
  - `core` — event model (`Event`), payloads and helpers
  - `sources` — clipboard polling via `tauri-plugin-clipboard-manager` and capture; runs in a fast, non-blocking background task and captures app/window context on macOS
  - `storage` — DB wrapper, schema, and batched EventWriter; emits `events:new` to the frontend on new inserts
  - `commands` — Tauri command handlers invoked by the frontend

### Available Tauri commands (invoke)

- `get_events(pinned_only?: boolean, source_app?: string)` — returns events, filters optional
- `get_all_events()` — returns recent events (legacy)
- `get_events_by_timestamp_range(start_ms, end_ms)`
- `get_pinned_events()`
- `pin_event(event_id)` / `unpin_event(event_id)`
- `delete_event(event_id)`
- `get_event_count()`
- `test_insert_clipboard_event(content)` — test helper

## Realtime event flow:
- frontend listens for `events:new` and merges newly ingested clipboard events into state without a full refresh

Frontend (src)

- Preact + TypeScript UI lives in `src/` and components are under `src/components/`.
- **Main component**: `EventTimeline` is the composable root view that coordinates state and event handling.
- **Subcomponents** (in `src/components/timeline/`):
  - `EventHeader` — filtering controls, event count, last refresh time, delete-all button, and toast messages
  - `EventList` — container for rendering the event list
  - `EventItem` — individual event display with timestamp, app context, content preview
  - `EventActions` — action buttons (pin, copy, delete) with internal click animation state
  - `ErrorBox` — error display with dismiss button
- **Helpers** (in `src/components/helpers/`):
  - `clipboard.ts` — clipboard operations (`copyTextToClipboard`, `copyImageToClipboard`, `copyEventContent`)
  - `formatting.ts` — utilities for formatting timestamps, payload previews, and error messages
- System tray menu: the last 10 pinned clipboard items are surfaced in the tray menu for quick restore back to the system clipboard.
- **Image preview**: When an event payload type is `clipboard_image`, the UI renders an inline image preview (max 240×240px) sourced from the preview data URL stored in the event.

## Design notes

- **Deduplication**: Clipboard content is hashed (xxHash64) and stored as `content_hash` in the DB. Both text and image payloads are deduplicated; duplicate prevention happens both in-memory (last seen hash) and by consulting the DB before inserting.
- **Image handling**: When clipboard text is empty, the watcher attempts to read image data. Images are encoded to PNG, a full-quality PNG is stored in the `blobs` table, and a resized preview (max 512×512px, encoded as base64 data URL) is included in the event payload for quick UI rendering.
- **Polling**: System clipboard polling uses `arboard` inside a fast, non-blocking background task with a configurable interval (default 150ms), so clipboard reads stay responsive.
- **Pinning**: `pinned` is a boolean column; pinned items are ordered to the top in queries.
- **Context**: `source_app` and `window_title` (macOS) are captured per event where available.

## Where to look

- Backend: `src-tauri/src/`
- Frontend: `src/components/EventTimeline.tsx`, `src/styles/EventTimeline.css`
- DB schema: `src-tauri/src/storage/schema.rs`

## Next steps (ideas)

- Add soft-delete / trash with undo
- Add search and tag extraction
- Add more content types (files, rich text formatting)
- Add blob retrieval endpoint for full-quality image downloads
- Improve timeline image copy and cross-platform native image clipboard restore

## License
MIT
