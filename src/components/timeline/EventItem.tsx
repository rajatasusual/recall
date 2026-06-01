import { VNode } from "preact";
import { EventRecord } from "../../types";
import { formatTimestamp, getPayloadPreview } from "../helpers/formatting";
import { EventActions } from "./EventActions";

interface EventItemProps {
  event: EventRecord;
  onPin: (eventId: string, isPinned: boolean) => Promise<void>;
  onCopy: (event: EventRecord) => Promise<void>;
  onDelete: (eventId: string) => Promise<void>;
}

export function EventItem({
  event,
  onPin,
  onCopy,
  onDelete,
}: EventItemProps): VNode {
  return (
    <div key={event.id} class={`event-item ${event.pinned ? "pinned" : ""}`}>
      <div class="event-header">
        <div class="event-meta">
          <span
            class="time"
            title={new Date(event.timestamp).toLocaleString()}
          >
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
            {getPayloadPreview(event.payload)}
            {event.payload.content && event.payload.content.length > 80 && "..."}
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
