import { VNode } from "preact";
import { useState } from "preact/hooks";
import { EventRecord } from "../../types";

interface EventActionsProps {
  event: EventRecord;
  onPin: (eventId: string, isPinned: boolean) => Promise<void>;
  onCopy: (event: EventRecord) => Promise<void>;
  onDelete: (eventId: string) => Promise<void>;
}

export function EventActions({
  event,
  onPin,
  onCopy,
  onDelete,
}: EventActionsProps): VNode {
  const [clicked, setClicked] = useState<string | null>(null);

  const handleClick = (action: string, callback: () => Promise<void>) => {
    setClicked(action);
    setTimeout(() => setClicked(null), 420);
    callback();
  };

  return (
    <div class="event-actions">
      <button
        class={`pin-btn ${event.pinned ? "pinned" : ""} ${
          clicked === "pin" ? "clicked" : ""
        }`}
        onClick={() => handleClick("pin", () => onPin(event.id, event.pinned))}
        title={event.pinned ? "Unpin" : "Pin"}
      >
        {event.pinned ? "📌" : "📍"}
      </button>

      <button
        class={`copy-btn ${clicked === "copy" ? "clicked" : ""}`}
        onClick={() => handleClick("copy", () => onCopy(event))}
        title="Copy to clipboard"
      >
        📋
      </button>

      <button
        class={`delete-btn ${clicked === "delete" ? "clicked" : ""}`}
        onClick={() => handleClick("delete", () => onDelete(event.id))}
        title="Delete"
      >
        🗑️
      </button>
    </div>
  );
}
