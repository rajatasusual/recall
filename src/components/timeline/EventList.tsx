import { VNode } from "preact";
import { EventRecord } from "../../types";
import { EventItem } from "./EventItem";

interface EventListProps {
  events: EventRecord[];
  onPin: (eventId: string, isPinned: boolean) => Promise<void>;
  onCopy: (event: EventRecord) => Promise<void>;
  onDelete: (eventId: string) => Promise<void>;
}

export function EventList({
  events,
  onPin,
  onCopy,
  onDelete,
}: EventListProps): VNode {
  if (events.length === 0) {
    return (
      <div class="events-list">
        <div class="empty-state">Waiting for clipboard events...</div>
      </div>
    );
  }

  return (
    <div class="events-list">
      <div class="events-container">
        {events.map((event) => (
          <EventItem
            key={event.id}
            event={event}
            onPin={onPin}
            onCopy={onCopy}
            onDelete={onDelete}
          />
        ))}
      </div>
    </div>
  );
}
