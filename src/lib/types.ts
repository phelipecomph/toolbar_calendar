// Mirror of src-tauri/src/model.rs — keep in sync by hand (fonte de verdade = Rust).

export type AccountKind = "ics" | "google" | "graph";

export interface Attendee {
  name?: string;
  email: string;
  status?: string;
}

export interface NormalizedEvent {
  id: string;
  account_id: string;
  source_id: string;
  title: string;
  start: string; // ISO8601 UTC
  end: string; // ISO8601 UTC
  all_day: boolean;
  description?: string;
  location?: string;
  meeting_link?: string;
  html_link?: string;
  attendees: Attendee[];
  color: string; // hex "#RRGGBB"
}

export interface AppConfig {
  window_before_minutes: number;
  window_after_minutes: number;
  sync_interval_minutes: number;
  strip_height_logical: number;
}

// NormalizedEvent + computed lane placement
export interface LaidOutEvent extends NormalizedEvent {
  lane: number;
  laneCount: number;
}
