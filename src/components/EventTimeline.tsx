import { useEffect, useRef, useState } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EventRecord } from "../types";

const copyTextToClipboard = async (text: string) => {
  // prefer the web clipboard when available
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }
  } catch (e) {
    // fall through to tauri fallback
  }
};

const copyImageToClipboard = async (dataUrl: string) => {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard && navigator.clipboard.write) {
      const res = await fetch(dataUrl);
      const blob = await res.blob();
      const item = new ClipboardItem({ [blob.type]: blob });
      await navigator.clipboard.write([item]);
      return;
    }
  } catch (e) {
    // fall through to tauri fallback
  }
}

interface EventTimelineProps {
  refreshInterval?: number;
}

export function EventTimeline({ refreshInterval = 5000 }: EventTimelineProps) {
  const [events, setEvents] = useState<EventRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const [pinnedOnly, setPinnedOnly] = useState<boolean>(false);
  const [sourceAppFilter, setSourceAppFilter] = useState<string | null>(null);
  const [clickedBtn, setClickedBtn] = useState<string | null>(null);
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const toastTimerRef = useRef<number | null>(null);

  const showToast = (message: string) => {
    setToastMessage(message);
    if (toastTimerRef.current) {
      window.clearTimeout(toastTimerRef.current);
    }
    toastTimerRef.current = window.setTimeout(() => {
      setToastMessage(null);
      toastTimerRef.current = null;
    }, 1500);
  };

  const loadEvents = async () => {
    setError(null);
    try {
      // pass both snake_case and camelCase keys to be compatible with TauriJS argument mapping
      const allEvents: EventRecord[] = await invoke("get_events", {
        pinned_only: pinnedOnly,
        pinnedOnly: pinnedOnly,
        source_app: sourceAppFilter,
        sourceApp: sourceAppFilter,
      });
      setEvents(allEvents);
      setLastRefresh(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const mergeIncomingEvent = (newEvent: EventRecord) => {
    if (pinnedOnly && !newEvent.pinned) {
      return;
    }
    if (sourceAppFilter && newEvent.source_app !== sourceAppFilter) {
      return;
    }

    setEvents((prevEvents) => {
      if (prevEvents.some((event) => event.id === newEvent.id)) {
        return prevEvents;
      }

      const insertAt = newEvent.pinned
        ? 0
        : prevEvents.findIndex((event) => !event.pinned);

      if (insertAt === -1) {
        return [...prevEvents, newEvent];
      }

      const next = [...prevEvents];
      next.splice(insertAt, 0, newEvent);
      return next;
    });
    setLastRefresh(new Date());
  };

  useEffect(() => {
    // initial load and subscribe to backend 'events:new' for near-realtime updates
    let stop: () => void = () => { };
    (async () => {
      await loadEvents();
      try {
        const unlisten = await listen("events:new", (event) => {
          const payload = event.payload as unknown;
          if (payload && typeof payload === "object") {
            mergeIncomingEvent(payload as EventRecord);
          }
        });
        stop = () => {
          try { unlisten(); } catch (e) { /* ignore */ }
        };
      } catch (e) {
        // if listening fails, fall back to periodic polling
        const id = setInterval(() => loadEvents(), refreshInterval);
        stop = () => clearInterval(id);
      }
    })();

    return () => stop();
  }, [refreshInterval, pinnedOnly, sourceAppFilter]);

  const formatTimestamp = (ms: number): string => {
    const date = new Date(ms);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    if (diff < 60000) return "just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    return date.toLocaleString();
  };

  const handleDelete = async (eventId: string) => {
    try {
      // some runtimes map arg names to camelCase; provide both forms
      await invoke("delete_event", { event_id: eventId, eventId });
      loadEvents();
    } catch (err) {
      console.error("Failed to delete event:", err);
    }
  };

  const handleSourceAppChange = (e: Event) => {
    const select = e.currentTarget as HTMLSelectElement;
    setSourceAppFilter(select.value || null);
  };
  const getPayloadPreview = (payload: Record<string, any>): string => {
    if (payload.type === "clipboard_image") {
      // payload.preview should be a data URL (small PNG)
      return payload.preview || "[image]";
    }

    if (payload.content) {
      return payload.content.substring(0, 80);
    }
    return JSON.stringify(payload).substring(0, 80);
  };

  const handlePin = async (eventId: string, isPinned: boolean) => {
    try {
      if (isPinned) {
        await invoke("unpin_event", { event_id: eventId, eventId });
      } else {
        await invoke("pin_event", { event_id: eventId, eventId });
      }
      loadEvents();
    } catch (err) {
      console.error("Failed to toggle pin:", err);
    }
  };

  return (
    <div class="event-timeline">
      <div class="timeline-header">
        <div class="header-top">
          <div style="display:flex;gap:8px;align-items:center">
            <label style="font-size:12px;color:var(--muted)">
              <input type="checkbox" checked={pinnedOnly} onChange={() => setPinnedOnly(!pinnedOnly)} />
              &nbsp;Pinned only
            </label>
            <select onChange={handleSourceAppChange} value={sourceAppFilter ?? ""}>
              <option value="">All apps</option>
              {Array.from(new Set(events.map(ev => ev.source_app).filter(Boolean) as string[])).map((app) => (
                <option value={String(app)}>{String(app)}</option>
              ))}
            </select>
            <span class="event-count">
              {events.length} item{events.length !== 1 ? "s" : ""}
            </span>
            <span class="last-refresh" style="font-size:12px;color:var(--muted);margin-left:8px">Last: {lastRefresh.toLocaleTimeString()}</span>
            {toastMessage && (
              <span class="toast">{toastMessage}</span>
            )}
          </div>
          <div>
            <button class="delete-all-btn" onClick={() => {
              setClickedBtn(`delete_all`);
              setTimeout(() => setClickedBtn(null), 420);
              invoke("delete_all_events").then(() => loadEvents());
              setToastMessage("All unpinned events deleted.");
            }}>
              🧹
            </button>
          </div>
        </div>
      </div>

      {error && (
        <div class="error-box">
          <strong>Error:</strong> {error}
        </div>
      )}

      <div class="events-list">
        {events.length === 0 ? (
          <div class="empty-state">
            Waiting for clipboard events...
          </div>
        ) : (
          <div class="events-container">
            {events.map((event) => (
              <div key={event.id} class={`event-item ${event.pinned ? "pinned" : ""}`}>
                <div class="event-header">
                  <div class="event-meta">
                    <span class="time" title={new Date(event.timestamp).toLocaleString()}>
                      {formatTimestamp(event.timestamp)}
                    </span>
                    {event.source_app && (
                      <span class="app-context">{event.source_app}</span>
                    )}
                    {event.payload.is_truncated && (
                      <span class="truncation-badge">truncated</span>
                    )}
                  </div>
                </div>
                <div class="event-content">
                  {event.payload.type === "clipboard_image" ? (
                    event.payload.preview ? (
                      // render small preview image
                      <img src={event.payload.preview} alt="clipboard" style="max-width:80px;max-height:80px;border-radius:6px" />
                    ) : (
                      <span>[image]</span>
                    )
                  ) : (
                    <>
                      {getPayloadPreview(event.payload)}
                      {event.payload.content && event.payload.content.length > 80 && "..."}
                    </>
                  )}
                </div>
                <div class="event-actions">
                  <button
                    class={`pin-btn ${event.pinned ? "pinned" : ""} ${clickedBtn === `${event.id}:pin` ? 'clicked' : ''}`}
                    onClick={() => {
                      setClickedBtn(`${event.id}:pin`);
                      setTimeout(() => setClickedBtn(null), 420);
                      handlePin(event.id, event.pinned);
                    }}
                    title={event.pinned ? "Unpin" : "Pin"}
                  >
                    {event.pinned ? "📌" : "📍"}
                  </button>
                  <button
                    class={`copy-btn ${clickedBtn === `${event.id}:copy` ? 'clicked' : ''}`}
                    onClick={async () => {
                      try {
                        let content: string;
                        if (event.payload.type === "clipboard_image") {
                          content = event.payload.preview ?? event.content_hash ?? JSON.stringify(event.payload);
                          await copyImageToClipboard(content);

                        } else {
                          content = event.payload?.content ?? JSON.stringify(event.payload);
                          await copyTextToClipboard(String(content));

                        }
                        setClickedBtn(`${event.id}:copy`);
                        setTimeout(() => setClickedBtn(null), 420);
                        showToast("Copied");
                      } catch (err) {
                        console.error("Failed to copy to clipboard:", err);
                      }
                    }}
                    title="Copy to clipboard"
                  >
                    Copy
                  </button>
                  <button
                    class={`delete-btn ${clickedBtn === `${event.id}:delete` ? 'clicked' : ''}`}
                    onClick={() => {
                      setClickedBtn(`${event.id}:delete`);
                      setTimeout(() => setClickedBtn(null), 420);
                      handleDelete(event.id);
                    }}
                    title="Delete"
                  >
                    🗑️
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
