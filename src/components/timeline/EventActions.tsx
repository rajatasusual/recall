import { VNode } from "preact";
import { useState } from "preact/hooks";
import { ClipboardFormat, EventRecord } from "../../types";
import { COPY_FORMATS } from "../helpers/copyFormats";

interface EventActionsProps {
  event: EventRecord;
  onPin: (eventId: string, isPinned: boolean) => Promise<void>;
  onCopy: (event: EventRecord) => Promise<void>;
  onCopyFormat: (event: EventRecord, format: ClipboardFormat) => Promise<void>;
  onDelete: (eventId: string) => Promise<void>;
}

export function EventActions({
  event,
  onPin,
  onCopy,
  onCopyFormat,
  onDelete,
}: EventActionsProps): VNode {
  const [clicked, setClicked] = useState<string | null>(null);
  const isImage = event.payload.type === "clipboard_image";

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

      <div class="copy-format-actions" aria-label="Copy formats">
        {COPY_FORMATS.map((action) => (
          <button
            key={action.format}
            class={`copy-format-btn ${
              clicked === action.format ? "clicked" : ""
            }`}
            disabled={isImage}
            onClick={() => handleClick(action.format, () => onCopyFormat(event, action.format))}
            title={action.title}
          >
            {action.label}
          </button>
        ))}
      </div>

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
