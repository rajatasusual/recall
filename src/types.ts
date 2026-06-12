export type Status = "idle" | "recording" | "transcribing" | "answering";

export interface EventRecord {
  id: string;
  timestamp: number;
  source: string;
  payload_type: string;
  payload: Record<string, any>;
  window_title?: string | null;
  source_app?: string | null;
  content_hash?: string | null;
  pinned: boolean;
  created_at: number;
  classification?: string;
}

export type ClipboardFormat =
  | "original"
  | "plain_text"
  | "uppercase"
  | "lowercase"
  | "remove_formatting"
  | "convert_quotes"
  | "strip_tracking_params";
