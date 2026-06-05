import { VNode } from "preact";
import { EventRecord } from "../../types";
import { formatTimestamp, getPayloadPreview } from "../helpers/formatting";
import { EventActions } from "./EventActions";

interface EventItemProps {
  event: EventRecord;
  searchQuery: string;
  onPin: (eventId: string, isPinned: boolean) => Promise<void>;
  onCopy: (event: EventRecord) => Promise<void>;
  onDelete: (eventId: string) => Promise<void>;
}

export function EventItem({
  event,
  searchQuery,
  onPin,
  onCopy,
  onDelete,
}: EventItemProps): VNode {
  const preview = getPayloadPreview(event.payload);
  const searchActive = searchQuery.trim().length > 0;
  const eventDate = new Date(event.timestamp);
  const visibleTimestamp = searchActive
    ? `${eventDate.toLocaleString()} ${eventDate.toISOString().slice(0, 10)}`
    : formatTimestamp(event.timestamp);
  const shouldTruncate =
    event.payload.type !== "clipboard_image" &&
    typeof event.payload.content === "string" &&
    event.payload.content.length > 80;

  return (
    <div key={event.id} class={`event-item ${event.pinned ? "pinned" : ""}`}>
      <div class="event-header">
        <div class="event-meta">
          <span
            class="time"
            title={eventDate.toLocaleString()}
          >
            {highlightText(visibleTimestamp, searchQuery)}
          </span>
          
          {event.source_app && (
            <span class="app-context">{highlightText(event.source_app, searchQuery)}</span>
          )}

          {event.classification && (
            <span class="classification-badge">
              {highlightText(event.classification, searchQuery)}
            </span>
          )}

          {event.window_title && (
            <span class="window-title">
              {highlightText(event.window_title, searchQuery)}
            </span>
          )}
          
          {event.payload.is_truncated && (
            <span class="truncation-badge">truncated</span>
          )}
        </div>
      </div>

      <div class="event-content">
        {event.payload.type === "clipboard_image" ? (
          event.payload.preview ? (
            <img
              src={event.payload.preview}
              alt="clipboard"
              style="max-width:80px;max-height:80px;border-radius:6px"
            />
          ) : (
            <span>[image]</span>
          )
        ) : (
          <>
            {highlightText(preview, searchQuery)}
            {shouldTruncate && "..."}
          </>
        )}
      </div>

      <EventActions
        event={event}
        onPin={onPin}
        onCopy={onCopy}
        onDelete={onDelete}
      />
    </div>
  );
}

function highlightText(text: string, query: string): Array<string | VNode> {
  const terms = Array.from(
    new Set(query.trim().split(/\s+/).filter(Boolean).map(escapeRegExp))
  );

  if (terms.length === 0) {
    return [text];
  }

  const matcher = new RegExp(`(${terms.join("|")})`, "gi");

  return text.split(matcher).map((part, index) => {
    if (!part) {
      return "";
    }

    if (terms.some((term) => new RegExp(`^${term}$`, "i").test(part))) {
      return <mark key={`${part}-${index}`}>{part}</mark>;
    }

    return part;
  });
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
