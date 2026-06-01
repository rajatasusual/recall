import { VNode } from "preact";
import { EventRecord } from "../../types";
import { getUniqueSourceApps } from "../helpers/formatting";

interface EventHeaderProps {
  events: EventRecord[];
  pinnedOnly: boolean;
  sourceAppFilter: string | null;
  lastRefresh: Date;
  toastMessage: string | null;
  onPinnedOnlyChange: (value: boolean) => void;
  onSourceAppChange: (app: string | null) => void;
  onDeleteAll: () => void;
  deleteAllClicked: boolean;
}

export function EventHeader({
  events,
  pinnedOnly,
  sourceAppFilter,
  lastRefresh,
  toastMessage,
  onPinnedOnlyChange,
  onSourceAppChange,
  onDeleteAll,
  deleteAllClicked,
}: EventHeaderProps): VNode {
  const sourceApps = getUniqueSourceApps(events);

  const handleSourceAppSelect = (e: Event) => {
    const select = e.currentTarget as HTMLSelectElement;
    onSourceAppChange(select.value || null);
  };

  return (
    <div class="timeline-header">
      <div class="header-top">
        <div style="display:flex;gap:8px;align-items:center">
          <label style="font-size:12px;color:var(--muted)">
            <input
              type="checkbox"
              checked={pinnedOnly}
              onChange={() => onPinnedOnlyChange(!pinnedOnly)}
            />
            &nbsp;Pinned only
          </label>
          
          <select onChange={handleSourceAppSelect} value={sourceAppFilter ?? ""}>
            <option value="">All apps</option>
            {sourceApps.map((app) => (
              <option value={app}>{app}</option>
            ))}
          </select>
          
          <span class="event-count">
            {events.length} item{events.length !== 1 ? "s" : ""}
          </span>
          
          <span
            class="last-refresh"
            style="font-size:12px;color:var(--muted);margin-left:8px"
          >
            Last: {lastRefresh.toLocaleTimeString()}
          </span>
          
          {toastMessage && (
            <span class="toast">{toastMessage}</span>
          )}
        </div>

        <div>
          <button
            class={`delete-all-btn ${deleteAllClicked ? "clicked" : ""}`}
            onClick={onDeleteAll}
            title="Delete all unpinned events"
          >
            🧹
          </button>
        </div>
      </div>
    </div>
  );
}
