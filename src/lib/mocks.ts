import type { NormalizedEvent } from "./types";

// Fallback mocks (browser preview / when the backend isn't reachable).
// Positioned relative to `now` so they always land inside the window.
// The Rust backend returns an equivalent set from `get_events` — see model.rs.

const MIN = 60_000;

function iso(ms: number): string {
  return new Date(ms).toISOString();
}

function mk(
  id: string,
  acc: string,
  color: string,
  title: string,
  startMs: number,
  endMs: number
): NormalizedEvent {
  return {
    id,
    account_id: acc,
    source_id: acc,
    color,
    title,
    start: iso(startMs),
    end: iso(endMs),
    all_day: false,
    attendees: [],
  };
}

export function MOCK_EVENTS(now: number): NormalizedEvent[] {
  const blue = "#4285f4"; // personal account
  const green = "#0b8043"; // work account
  const orange = "#f4511e"; // side account

  return [
    mk("m1", "acc-personal", blue, "Standup", now - 20 * MIN, now + 10 * MIN),
    // overlapping trio to exercise lanes (30–120 / 60–90 / 75–105)
    mk("m2", "acc-work", green, "Design review", now + 30 * MIN, now + 120 * MIN),
    mk("m3", "acc-work", green, "1:1 with Ana", now + 60 * MIN, now + 90 * MIN),
    mk("m4", "acc-personal", blue, "Dentist", now + 75 * MIN, now + 105 * MIN),
    mk("m5", "acc-side", orange, "Deploy window", now + 150 * MIN, now + 210 * MIN),
    mk("m6", "acc-work", green, "Lunch", now + 240 * MIN, now + 300 * MIN),
    mk("m7", "acc-personal", blue, "Focus: report", now + 320 * MIN, now + 440 * MIN),
  ];
}
