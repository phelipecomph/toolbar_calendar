<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openInBrowser } from "../lib/ipc";
  import type { NormalizedEvent } from "../lib/types";

  let ev = $state<NormalizedEvent | null>(null);

  function fmt(iso: string): string {
    return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  function fmtDay(iso: string): string {
    return new Date(iso).toLocaleDateString([], { weekday: "short", day: "2-digit", month: "short" });
  }

  onMount(() => {
    const w = getCurrentWindow();
    const un1 = listen<NormalizedEvent>("detail://event", (e) => (ev = e.payload));
    // Close on focus loss (borderless always-on-top popover).
    const un2 = w.onFocusChanged(({ payload: focused }) => {
      if (!focused) w.hide();
    });
    return () => {
      un1.then((u) => u());
      un2.then((u) => u());
    };
  });
</script>

{#if ev}
  <div class="detail">
    <div class="dt-title" style="border-left:3px solid {ev.color}">{ev.title}</div>
    <div class="dt-time">{fmtDay(ev.start)} · {fmt(ev.start)}–{fmt(ev.end)}</div>
    {#if ev.location}<div class="dt-row">📍 {ev.location}</div>{/if}
    {#if ev.attendees && ev.attendees.length}
      <div class="dt-row">👥 {ev.attendees.length} attendee(s)</div>
    {/if}
    {#if ev.description}<div class="dt-desc">{ev.description}</div>{/if}
    <div class="dt-actions">
      {#if ev.meeting_link}
        <button onclick={() => openInBrowser(ev!.meeting_link!)}>Join meeting</button>
      {/if}
      {#if ev.html_link}
        <button onclick={() => openInBrowser(ev!.html_link!)}>Open in browser</button>
      {/if}
    </div>
  </div>
{/if}
