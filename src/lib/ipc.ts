import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { NormalizedEvent } from "./types";

// Thin wrappers over the Tauri command / event contract (see PLAN.md §2.4).
// Every call throws if not running inside Tauri (e.g. plain browser preview);
// callers should fall back gracefully.

export async function getEvents(from: string, to: string): Promise<NormalizedEvent[]> {
  return invoke<NormalizedEvent[]>("get_events", { from, to });
}

export async function openInBrowser(url: string): Promise<void> {
  return invoke("open_in_browser", { url });
}

export async function triggerSync(): Promise<void> {
  return invoke("trigger_sync");
}

// Fase 2: abre a janela `detail` posicionada perto do bloco clicado.
export async function openDetail(eventId: string, anchor: DOMRect): Promise<void> {
  return invoke("open_detail", {
    eventId,
    anchor: { x: anchor.x, y: anchor.y, width: anchor.width, height: anchor.height },
  });
}

interface EventsUpdatedPayload {
  events: NormalizedEvent[];
  generated_at: string;
}

export function onEventsUpdated(cb: (events: NormalizedEvent[]) => void): Promise<UnlistenFn> {
  return listen<EventsUpdatedPayload>("calendar://events-updated", (e) => cb(e.payload.events));
}
