# Architecture — Recall (clipboard)
## Overview
Recall is a Tauri desktop app with a Rust backend and a Preact frontend. The backend captures clipboard events (text and images), batches and stores them in SQLite with binary blob support, and exposes search, query, mutation, quick-copy, and copy-format commands to the UI via Tauri invokes.
The backend follows a **domain-driven + layered architecture**:
- Domain layer (pure types and business entities)
- Service layer (business logic orchestration)
- Persistence layer (database + schema + writers)
- API layer (Tauri commands + services boundary)
- Config layer (centralized application configuration)
Image content is deduplicated by content hash, encoded to PNG with preview thumbnails, and stored separately in a `blobs` table for efficient retrieval.
---
## Architecture layers (post-refactor)

Frontend (Preact)
↓
API Layer (Tauri commands / handlers)
↓
Service Layer (EventService, orchestration logic)
↓
Domain Layer (Event, ClipboardContent, etc.)
↓
Persistence Layer (Database, Writer, Schema)
↓
SQLite + Blob Store

---
## High-level components
### Frontend (Preact/TypeScript)
- `src/components/EventTimeline.tsx` — root component managing state, debounced search, filtering, live event merging, and orchestration
- `src/components/QuickOverlay.tsx` — compact last-10 clipboard overlay opened by `Cmd+Shift+V`
- `src/components/timeline/`
  - `EventHeader.tsx` — search-first input, filters, count, refresh state, and bulk delete control
  - `EventList.tsx` — list container with search-aware empty state
  - `EventItem.tsx` — event rendering with matched-term highlighting
  - `EventActions.tsx` — pin, copy, copy-format transformations, and delete controls
  - `ErrorBox.tsx`
- `src/components/helpers/`
  - `formatting.ts`
- `src/App.css`
- `src/types.ts`
---
### Backend (Rust/Tauri)
#### API layer (`api/`)
- `api/services.rs` — service façade used by Tauri commands
- `commands/events.rs` — thin command handlers delegating to services
- `commands/events.rs` — event retrieval/mutation commands plus clipboard-only copy formatting
Commands are now **thin wrappers** over services (no business logic inside handlers).
---
#### Service layer (`services`)
- `EventService`
  - Encapsulates event querying, recent-history retrieval, search filtering, pinning, deletion, and single-event lookup
  - Replaces direct DB access from commands
  - Centralizes response mapping for frontend command boundaries
---
#### Domain layer (`domain`)
- `domain/event`
  - `Event`
  - `EventPayload`
  - `EventRecord`
- `domain/clipboard`
  - Clipboard content abstraction (`ClipboardContent`)
  - Processing logic for text/image normalization
Event domain types are framework-light. Clipboard monitoring still integrates with Tauri clipboard APIs directly while the migration continues.
---
#### Persistence layer (`persistence`)
- `persistence/database.rs` — SQLite connection + queries
- `persistence/schema.rs` — schema definition (events, blobs, edges)
- `persistence/writer.rs` — batched event writer
---
#### Core / legacy compatibility
- `core/` retained for compatibility and incremental migration
- Gradual migration target: domain + service fully replaces core logic
---
#### Config layer (`config.rs`)
- `AppConfig`
- `ClipboardConfig`
- `StorageConfig`
- `WriterConfig`
Supports environment overrides and test-time configuration injection.
---
## Database schema
### events table
Same structure as before, with no semantic changes. Search-first retrieval uses existing columns and does not require a migration.
### blobs table
Same structure as before.
### edges table
Same structure as before.
---
## Event flow (post-refactor)
1. Clipboard polling (`domain/clipboard/mod.rs`)
   - Reads text or image
   - Converts into `ClipboardContent` (domain abstraction)
2. Domain processing
   - `process_text()` / `process_image()`
   - Computes hash + normalization
   - Produces unified content type
3. Service layer (`EventService`)
   - Applies business rules
   - Calls persistence layer via writer/database
4. Persistence layer
   - Inserts into `events`
   - Inserts into `blobs` (if image)
   - Emits durability guarantees (best-effort blob writes preserved)
5. API layer
   - Tauri commands call `EventService`
6. Frontend
   - Listens to `events:new` events
   - Updates UI incrementally
---
## Retrieval flow
1. User types in the search-first input in `EventHeader`
   - `EventTimeline` trims and debounces the query before invoking the backend
2. API layer
   - `get_events(pinned_only?, source_app?, classification?, query?)` forwards all filters to `EventService`
3. Service layer
   - Delegates retrieval to `Database::get_events`
   - Formats `EventRecord` values for the frontend
4. Persistence layer
   - Applies pinned, classification, and legacy source app filters
   - Applies the optional search query with parameterized SQLite `LIKE` predicates
   - Searchable fields include payload data, classification, source, common local date formats, and legacy source app/window title values if present
   - Multi-word queries require every term to match somewhere across the searchable fields
5. Frontend results
   - Active search terms are highlighted in visible metadata and content previews
   - New `events:new` payloads are merged only when they match the active filters and search query
---
## Quick copy and transformations
### Quick overlay flow
1. User presses `Cmd+Shift+V`
2. Main Recall window is minimized
3. `quick_overlay` is centered, shown, focused, and sent `quick-overlay:show`
4. `QuickOverlay` invokes `get_recent_events(limit: 10)`
5. User chooses an item with `1` through `0`, `Enter`, click, or a format key
6. Frontend invokes `copy_event_to_clipboard(event_id, format?)`
7. Backend writes the selected representation to the system clipboard
8. The quick overlay hides after a successful copy

### Main-window copy flow
1. User chooses original copy or a format button in `EventActions`
2. Frontend invokes `copy_event_to_clipboard(event_id, format?)`
3. Backend writes the selected representation to the system clipboard

### Supported transforms
- `original`
- `plain_text`
- `uppercase`
- `lowercase`
- `remove_formatting`
- `convert_quotes`
- `strip_tracking_params`

Images support original copy only. Text transforms operate on the stored clipboard text before writing to the system clipboard.

Recall does not capture the frontmost app, request Apple Events automation, restore focus to another app, or send paste keystrokes.
---
## Window model
- `main`
  - transparent Tauri window
  - minimum inner size: `640x800`
  - full history, search, filters, and item actions
- `quick_overlay`
  - transparent, frameless, always-on-top Tauri window
  - default inner size: `560x620`
  - minimum inner size: `480x520`
  - hidden immediately after creation

The window-state plugin deliberately excludes the `VISIBLE` flag so a previously visible overlay is not restored at startup. Focus and shortcut handlers keep `main` and `quick_overlay` mutually exclusive: showing one hides the other.
---
## Deduplication
Unchanged behavior, but now logically split:
- Domain layer: hash computation
- Service layer: deduplication decision
- Persistence layer: enforcement via DB lookup
Mechanisms:
- In-memory last-hash (fast-path)
- DB-level `content_hash` uniqueness
- Best-effort dedup in `emit_content`
---
## Commands (summary)
All commands now route through `EventService`:
- `get_events(pinned_only?, source_app?, classification?, query?)`
- `get_recent_events(limit?)`
- `pin_event`, `unpin_event`
- `delete_event`
- `delete_all_events`
- `copy_event_to_clipboard(event_id, format?)`
- `hide_quick_overlay`
---
## Notes / Future work
- Core → Domain migration completion
- Move remaining legacy modules into `domain/` and `persistence/`
- Introduce event pipeline observability (tracing spans per layer)
- Add repository traits to decouple persistence backend
- Add a full-text search index/service abstraction if query volume or history size outgrows the current SQLite `LIKE` retrieval

⸻
