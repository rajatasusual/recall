# Architecture — Recall (clipboard)
## Overview
Recall is a Tauri desktop app with a Rust backend and a Preact frontend. The backend captures clipboard events (text and images), batches and stores them in SQLite with binary blob support, and exposes query and mutation commands to the UI via Tauri invokes.
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
- `src/components/EventTimeline.tsx` — root component managing state, filtering, and orchestration
- `src/components/timeline/`
  - `EventHeader.tsx`
  - `EventList.tsx`
  - `EventItem.tsx`
  - `EventActions.tsx`
  - `ErrorBox.tsx`
- `src/components/helpers/`
  - `clipboard.ts`
  - `formatting.ts`
- `src/styles/EventTimeline.css`
- `src/types.ts`
---
### Backend (Rust/Tauri)
#### API layer (`api/`)
- `api/services.rs` — service façade used by Tauri commands
- `commands/events.rs` — thin command handlers delegating to services
Commands are now **thin wrappers** over services (no business logic inside handlers).
---
#### Service layer (`services`)
- `EventService`
  - Encapsulates event creation, querying, pinning, deletion
  - Replaces direct DB access from commands
  - Centralizes business rules (validation, mapping, transformations)
---
#### Domain layer (`domain`)
- `domain/event`
  - `Event`
  - `EventPayload`
  - `EventRecord`
- `domain/clipboard`
  - Clipboard content abstraction (`ClipboardContent`)
  - Processing logic for text/image normalization
Domain layer is **framework-agnostic** and contains no persistence or Tauri dependencies.
---
#### Persistence layer (`persistence`)
- `persistence/database.rs` — SQLite connection + queries
- `persistence/schema.rs` — schema definition (events, blobs, edges)
- `persistence/writer.rs` — batched event writer
- `storage/mod.rs` — backward-compatible re-exports
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
Same structure as before, with no semantic changes.
### blobs table
Same structure as before.
### edges table
Same structure as before.
---
## Event flow (post-refactor)
1. Clipboard polling (`sources/clipboard.rs`)
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
- `get_events`
- `get_all_events`
- `get_events_by_timestamp_range`
- `get_pinned_events`
- `pin_event`, `unpin_event`
- `delete_event`
- `get_event_count`
---
## Notes / Future work
- Core → Domain migration completion
- Move remaining legacy modules into `domain/` and `persistence/`
- Introduce event pipeline observability (tracing spans per layer)
- Add repository traits to decouple persistence backend
- Add full-text search service layer abstraction

⸻