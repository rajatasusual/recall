import { VNode } from "preact";
import { EventRecord } from "../../types";
import { EventItem } from "./EventItem";

interface EventListProps {
  events: EventRecord[];
  searchQuery: string;
  onPin: (eventId: string, isPinned: boolean) => Promise<void>;
  onCopy: (event: EventRecord) => Promise<void>;
  onDelete: (eventId: string) => Promise<void>;
}

export function EventList({
  events,
  searchQuery,
  onPin,
  onCopy,
  onDelete,
}: EventListProps): VNode {
  if (events.length === 0) {
    return (
      <div class="events-list">
        <div class="empty-state">
          {searchQuery ? "No matching clips" : "Waiting for clipboard events..."}
        </div>
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
            searchQuery={searchQuery}
            onPin={onPin}
            onCopy={onCopy}
            onDelete={onDelete}
          />
        ))}
      </div>
    </div>
  );
}
