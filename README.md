# Recall — Intelligent Clipboard

<img src="image.png" alt="recall" width="64"/>

Recall captures clipboard items (text and images), stores them locally, and provides a Tauri + Preact UI to browse, pin, filter and restore clipboard entries.
Architecture has been refactored into a **layered system**:
- Domain layer (core types)
- Service layer (business logic)
- Persistence layer (SQLite + blobs)
- API layer (Tauri command boundary)
This separation improves testability, modularity, and alignment with Tauri v2 best practices.

<img src="screenshot.png" alt="recall" width="600"/>

---
## Quick start (development)

npm install
cd src-tauri/
npm run tauri dev

⸻

## Backend architecture (Rust)

### Layers

1. API layer (api/)

* Thin Tauri command wrappers
* Delegates all logic to services

2. Service layer

* EventService
* Owns business logic (create, pin, delete, query)
* Replaces direct DB access from commands

3. Domain layer (domain/)

* Pure types:
    * Event
    * EventPayload
    * ClipboardContent
* No I/O or framework dependencies

4. Persistence layer (persistence/)

* SQLite access
* Schema definition
* Batched writer (EventWriter)

5. Config (config.rs)

* Centralized runtime configuration
* Environment-variable override support

⸻

### Clipboard event flow

1. Clipboard polling detects change
2. Domain converts raw input → ClipboardContent
3. Service layer validates + deduplicates
4. Persistence layer writes:
    * event row
    * optional blob row (images)
5. Frontend receives events:new update

⸻

### Available Tauri commands

All commands are service-backed:

* get_events(pinned_only?, source_app?)
* get_all_events()
* get_events_by_timestamp_range(start, end)
* get_pinned_events()
* pin_event(id)
* unpin_event(id)
* delete_event(id)
* get_event_count()

⸻

## Frontend (src)

Unchanged UI structure:

* EventTimeline — root controller
* EventHeader — filters + controls
* EventList — list container
* EventItem — event rendering
* EventActions — pin/copy/delete actions
* ErrorBox — error display

Helpers:

* clipboard.ts
* formatting.ts

⸻

#### Key improvements (post-refactor)

Architecture

* Clear separation: domain → service → persistence → API
* Commands reduced to thin adapters

Maintainability

* Business logic centralized in EventService
* Storage logic isolated in persistence/

Testability

* Domain and service layers are unit-test friendly
* Config injection enables deterministic tests

Reliability

* Improved error handling via structured AppError
* Better type safety in service-return boundaries

⸻

Database

No schema changes:

* events
* blobs
* edges

All remain backward compatible.

⸻

## Next steps

* Complete migration of remaining core logic into domain layer
* Introduce repository traits for persistence abstraction
* Add full-text search pipeline
* Add observability (tracing per service call)
* Optimize clipboard polling efficiency