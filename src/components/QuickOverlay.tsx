import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "preact/hooks";
import { ClipboardFormat, EventRecord } from "../types";
import { COPY_FORMAT_KEYS, COPY_FORMATS } from "./helpers/copyFormats";
import { formatError, formatTimestamp, getPayloadPreview } from "./helpers/formatting";

export function QuickOverlay() {
  const [events, setEvents] = useState<EventRecord[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const eventsRef = useRef<EventRecord[]>([]);
  const selectedIndexRef = useRef(0);
  const loadingRef = useRef(false);

  const loadRecent = async () => {
    if (loadingRef.current) {
      return;
    }

    loadingRef.current = true;
    setError(null);
    try {
      const recent: EventRecord[] = await invoke("get_recent_events", { limit: 10 });
      eventsRef.current = recent;
      setEvents(recent);
      selectedIndexRef.current = 0;
      setSelectedIndex(0);
    } catch (err) {
      setError(formatError(err));
    } finally {
      loadingRef.current = false;
    }
  };

  const hide = async () => {
    try {
      await invoke("hide_quick_overlay");
    } catch {
      // The backend may already have hidden the overlay.
    }
  };

  const copyEvent = async (event: EventRecord, format: ClipboardFormat) => {
    if (event.payload.type === "clipboard_image" && format !== "original") {
      return;
    }

    try {
      await invoke("copy_event_to_clipboard", {
        event_id: event.id,
        eventId: event.id,
        format,
      });
      await hide();
    } catch (err) {
      setError(formatError(err));
    }
  };

  useEffect(() => {
    loadRecent();

    let stopListening = () => {};
    (async () => {
      const unlisten = await listen("quick-overlay:show", () => {
        loadRecent();
      });
      stopListening = unlisten;
    })();

    return () => stopListening();
  }, []);

  useEffect(() => {
    eventsRef.current = events;
  }, [events]);

  useEffect(() => {
    selectedIndexRef.current = selectedIndex;
  }, [selectedIndex]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const currentEvents = eventsRef.current;
      const selectedEvent = currentEvents[selectedIndexRef.current];

      if (event.key === "Escape") {
        event.preventDefault();
        hide();
        return;
      }

      if (event.key === "ArrowDown") {
        event.preventDefault();
        const nextIndex = Math.min(selectedIndexRef.current + 1, currentEvents.length - 1);
        selectedIndexRef.current = nextIndex;
        setSelectedIndex(nextIndex);
        return;
      }

      if (event.key === "ArrowUp") {
        event.preventDefault();
        const nextIndex = Math.max(selectedIndexRef.current - 1, 0);
        selectedIndexRef.current = nextIndex;
        setSelectedIndex(nextIndex);
        return;
      }

      const numericIndex = numberKeyToIndex(event.key);
      if (numericIndex !== null && currentEvents[numericIndex]) {
        event.preventDefault();
        copyEvent(currentEvents[numericIndex], "original");
        return;
      }

      const format = COPY_FORMAT_KEYS[event.key.length === 1 ? event.key.toLowerCase() : event.key];
      if (format && selectedEvent) {
        event.preventDefault();
        copyEvent(selectedEvent, format);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const selectedEvent = events[selectedIndex];
  const selectedIsText = selectedEvent?.payload.type !== "clipboard_image";

  return (
    <main class="quick-overlay">
      <header class="quick-overlay-header">
        <div>
          <div class="quick-overlay-subtitle">1-0 copy recent clips</div>
        </div>
        <button class="quick-overlay-close" onClick={hide} title="Close">
          ×
        </button>
      </header>

      {error && <div class="quick-overlay-error">{error}</div>}

      <section class="quick-overlay-list" aria-label="Recent clipboard items">
        {events.length === 0 ? (
          <div class="quick-overlay-empty">No recent clips</div>
        ) : (
          events.map((event, index) => (
            <button
              key={event.id}
              class={`quick-overlay-row ${selectedIndex === index ? "selected" : ""}`}
              onMouseEnter={() => setSelectedIndex(index)}
              onClick={() => copyEvent(event, "original")}
              title="Copy"
            >
              <span class="quick-overlay-number">{index === 9 ? 0 : index + 1}</span>
              <span class="quick-overlay-preview">
                {event.payload.type === "clipboard_image" ? (
                  event.payload.preview ? (
                    <img src={event.payload.preview} alt="Clipboard" />
                  ) : (
                    "[image]"
                  )
                ) : (
                  getPayloadPreview(event.payload)
                )}
              </span>
              <span class="quick-overlay-meta">
                {event.source_app ?? event.classification ?? event.payload_type}
                {" · "}
                {formatTimestamp(event.timestamp)}
              </span>
            </button>
          ))
        )}
      </section>

      <footer class="quick-overlay-actions">
        {COPY_FORMATS.map((action) => (
          <button
            key={action.format}
            disabled={!selectedIsText}
            onClick={() => selectedEvent && copyEvent(selectedEvent, action.format)}
            title={action.label}
          >
            <kbd>{action.key}</kbd>
            <span>{action.label}</span>
          </button>
        ))}
      </footer>
    </main>
  );
}

function numberKeyToIndex(key: string): number | null {
  if (!/^[0-9]$/.test(key)) {
    return null;
  }

  return key === "0" ? 9 : Number(key) - 1;
}
