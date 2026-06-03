import { EventRecord } from "../../types";

/**
 * Format a Unix timestamp (ms) as a relative time string
 */
export function formatTimestamp(ms: number): string {
  const date = new Date(ms);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  
  if (diff < 60000) return "just now";
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  
  return date.toLocaleString();
}

/**
 * Get a preview string from an event payload
 */
export function getPayloadPreview(payload: Record<string, any>): string {
  if (payload.type === "clipboard_image") {
    return payload.preview || "[image]";
  }

  if (payload.content) {
    return payload.content.substring(0, 80);
  }

  return JSON.stringify(payload).substring(0, 80);
}

/**
 * Format an error object as a readable string
 */
export function formatError(err: unknown): string {
  if (err instanceof Error) {
    return [
      `Message: ${err.message}`,
      err.stack ? `\nStack:\n${err.stack}` : "",
    ].join("");
  }

  try {
    return JSON.stringify(err, null, 2);
  } catch {
    return String(err);
  }
}

/**
 * Get unique source apps from a list of events
 */
export function getUniqueSourceApps(events: EventRecord[]): string[] {
  return Array.from(
    new Set(events.map(ev => ev.source_app).filter(Boolean) as string[])
  );
}

/**
 * Get classifications from a list of events
 */
export function getClassifications(events: EventRecord[]): string[] {
  return Array.from(
    new Set(events.map(ev => ev.classification).filter(Boolean) as string[])
  );
}