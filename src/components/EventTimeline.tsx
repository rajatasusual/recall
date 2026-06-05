import { useEffect, useRef, useState } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EventRecord } from "../types";
import { EventHeader } from "./timeline/EventHeader";
import { EventList } from "./timeline/EventList";
import { ErrorBox } from "./timeline/ErrorBox";
import { copyEventContent } from "./helpers/clipboard";
import { formatError } from "./helpers/formatting";

interface EventTimelineProps {
  refreshInterval?: number;
}

export function EventTimeline({ refreshInterval = 5000 }: EventTimelineProps) {
  const [events, setEvents] = useState<EventRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const [pinnedOnly, setPinnedOnly] = useState<boolean>(false);
  const [sourceAppFilter, setSourceAppFilter] = useState<string | null>(null);
  const [classificationFilter, setClassificationFilter] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState<string>("");
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const toastTimerRef = useRef<number | null>(null);
  const deleteAllClickedRef = useRef<boolean>(false);
  const loadRequestRef = useRef<number>(0);

  useEffect(() => {
    const id = window.setTimeout(() => {
      setDebouncedSearchQuery(searchQuery.trim());
    }, 180);

    return () => window.clearTimeout(id);
  }, [searchQuery]);

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
    const requestId = loadRequestRef.current + 1;
    loadRequestRef.current = requestId;
    setError(null);
    try {
      const allEvents: EventRecord[] = await invoke("get_events", {
        pinned_only: pinnedOnly,
        pinnedOnly: pinnedOnly,
        source_app: sourceAppFilter,
        classification: classificationFilter,
        query: debouncedSearchQuery || null,
      });
      if (requestId !== loadRequestRef.current) {
        return;
      }
      setEvents(allEvents);
      setLastRefresh(new Date());
    } catch (err) {
      if (requestId !== loadRequestRef.current) {
        return;
      }
      setError(formatError(err));
    }
  };

  const mergeIncomingEvent = (newEvent: EventRecord) => {
    if (pinnedOnly && !newEvent.pinned) {
      return;
    }
    if (sourceAppFilter && newEvent.source_app !== sourceAppFilter) {
      return;
    }
    if (classificationFilter && newEvent.classification !== classificationFilter) {
      return;
    }
    if (!matchesEventSearch(newEvent, debouncedSearchQuery)) {
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
      return next.slice(0, 1000);
    });
    setLastRefresh(new Date());
  };

  useEffect(() => {
    let stop: () => void = () => {};
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
          try {
            unlisten();
          } catch {
            // ignore
          }
        };
      } catch {
        const id = setInterval(() => loadEvents(), refreshInterval);
        stop = () => clearInterval(id);
      }
    })();

    return () => stop();
  }, [refreshInterval, pinnedOnly, sourceAppFilter, classificationFilter, debouncedSearchQuery]);

  const handlePin = async (eventId: string, isPinned: boolean) => {
    try {
      const action = isPinned ? "unpin_event" : "pin_event";
      await invoke(action, { event_id: eventId, eventId });
      await loadEvents();
    } catch (err) {
      setError(formatError(err));
    }
  };

  const handleCopy = async (event: EventRecord) => {
    try {
      await copyEventContent(event);
      showToast("Copied");
    } catch (err) {
      setError(formatError(err));
    }
  };

  const handleDelete = async (eventId: string) => {
    try {
      await invoke("delete_event", { event_id: eventId, eventId });
      await loadEvents();
    } catch (err) {
      setError(formatError(err));
    }
  };

  const handleDeleteAll = async () => {
    deleteAllClickedRef.current = true;
    setTimeout(() => {
      deleteAllClickedRef.current = false;
    }, 420);
    try {
      await invoke("delete_all_events");
      await loadEvents();
      showToast("All unpinned events deleted.");
    } catch (err) {
      setError(formatError(err));
    }
  };

  const handlePinnedOnlyChange = (value: boolean) => {
    setPinnedOnly(value);
  };

  const handleSourceAppChange = (app: string | null) => {
    setSourceAppFilter(app);
  };

  const handleClassificationChange = (classification: string | null) => {
    setClassificationFilter(classification);
  };

  const handleSearchChange = (query: string) => {
    setSearchQuery(query);
  };

  return (
    <div class="event-timeline">
      <EventHeader
        events={events}
        pinnedOnly={pinnedOnly}
        sourceAppFilter={sourceAppFilter}
        classificationFilter={classificationFilter}
        searchQuery={searchQuery}
        lastRefresh={lastRefresh}
        toastMessage={toastMessage}
        onPinnedOnlyChange={handlePinnedOnlyChange}
        onSourceAppChange={handleSourceAppChange}
        onClassificationChange={handleClassificationChange}
        onSearchChange={handleSearchChange}
        onDeleteAll={handleDeleteAll}
        deleteAllClicked={deleteAllClickedRef.current}
      />

      <ErrorBox error={error} onDismiss={() => setError(null)} />

      <EventList
        events={events}
        searchQuery={debouncedSearchQuery}
        onPin={handlePin}
        onCopy={handleCopy}
        onDelete={handleDelete}
      />
    </div>
  );
}

function matchesEventSearch(event: EventRecord, query: string): boolean {
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);

  if (terms.length === 0) {
    return true;
  }

  const searchable = [
    event.source,
    event.source_app,
    event.classification,
    event.window_title,
    event.payload?.content,
    event.payload?.preview,
    event.content_hash,
    new Date(event.timestamp).toLocaleString(),
    new Date(event.timestamp).toISOString(),
  ]
    .filter((value): value is string => typeof value === "string")
    .join(" ")
    .toLowerCase();

  return terms.every((term) => searchable.includes(term));
}
