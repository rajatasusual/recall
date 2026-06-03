import { VNode } from "preact";
import { EventRecord } from "../../types";
import { getClassifications, getUniqueSourceApps } from "../helpers/formatting";

interface EventHeaderProps {
  events: EventRecord[];
  pinnedOnly: boolean;
  sourceAppFilter: string | null;
  classificationFilter: string | null;
  lastRefresh: Date;
  toastMessage: string | null;
  onPinnedOnlyChange: (value: boolean) => void;
  onSourceAppChange: (app: string | null) => void;
  onClassificationChange: (classification: string | null) => void;
  onDeleteAll: () => void;
  deleteAllClicked: boolean;
}

export function EventHeader({
  events,
  pinnedOnly,
  sourceAppFilter,
  classificationFilter,
  lastRefresh,
  toastMessage,
  onPinnedOnlyChange,
  onSourceAppChange,
  onClassificationChange,
  onDeleteAll,
  deleteAllClicked,
}: EventHeaderProps): VNode {
  const sourceApps = getUniqueSourceApps(events);
  const classifications = getClassifications(events);

  const handleClassificationSelect = (e: Event) => {
    const select = e.currentTarget as HTMLSelectElement;
    onClassificationChange(select.value || null);
  };
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
          
          <select onChange={handleClassificationSelect} value={classificationFilter ?? ""}>
            <option value="">All classes</option>
            {classifications.map((classification) => (
              <option value={classification}>{classification}</option>
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
