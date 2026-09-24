<script lang="ts">
  import { onMount } from "svelte";
  import EventBlock from "./EventBlock.svelte";
  import NowMarker from "./NowMarker.svelte";
  import { layout } from "../lib/layout";
  import { makeWindow, isVisible, xOf } from "../lib/time";
  import {
    getEvents,
    onEventsUpdated,
    triggerSync,
    getConfig,
    onDockChanged,
    openSettings,
  } from "../lib/ipc";
  import { MOCK_EVENTS } from "../lib/mocks";
  import { isVertical, type Edge, type NormalizedEvent } from "../lib/types";

  const BEFORE_MIN = 60;
  const AFTER_MIN = 480;

  let raw = $state<NormalizedEvent[]>([]);
  let now = $state(Date.now());
  let widthPx = $state(0);
  let heightPx = $state(0);
  let edge = $state<Edge>("bottom");

  const vertical = $derived(isVertical(edge));
  const mainPx = $derived(vertical ? heightPx : widthPx);

  const win = $derived(makeWindow(now, BEFORE_MIN, AFTER_MIN));
  const view = $derived(
    layout(raw.filter((e) => isVisible(Date.parse(e.start), Date.parse(e.end), win)))
  );

  // clock (uses the system locale)
  const wd = $derived(
    new Date(now).toLocaleDateString([], { weekday: "short" }).replace(".", "")
  );
  const dm = $derived(new Date(now).toLocaleDateString([], { day: "2-digit", month: "2-digit" }));
  const hm = $derived(new Date(now).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }));

  onMount(() => {
    (async () => {
      try {
        const from = new Date(now - BEFORE_MIN * 60_000).toISOString();
        const to = new Date(now + AFTER_MIN * 60_000).toISOString();
        raw = await getEvents(from, to);
      } catch {
        raw = MOCK_EVENTS(now);
      }
    })();

    getConfig()
      .then((c) => (edge = c.edge))
      .catch(() => {});

    const unlistenEvents = onEventsUpdated((ev) => (raw = ev)).catch(() => null);
    const unlistenDock = onDockChanged((c) => (edge = c.edge)).catch(() => null);

    triggerSync().catch(() => {});

    const tick = setInterval(() => (now = Date.now()), 30_000);

    return () => {
      clearInterval(tick);
      unlistenEvents.then((u) => u && u());
      unlistenDock.then((u) => u && u());
    };
  });
</script>

<div
  class="strip"
  class:vertical
  bind:clientWidth={widthPx}
  bind:clientHeight={heightPx}
>
  {#each view.laid as ev (ev.id)}
    <EventBlock {ev} {win} {mainPx} {vertical} />
  {/each}

  {#each view.overflow as ov (ov.startMs)}
    <div
      class="overflow-badge"
      class:vertical
      style={vertical
        ? `top:${Math.min(Math.max(xOf(ov.endMs, win, mainPx) - 12, 0), mainPx - 12)}px`
        : `left:${Math.min(Math.max(xOf(ov.endMs, win, mainPx) - 20, 0), mainPx - 20)}px`}
      title="+{ov.count} overlapping event(s) hidden"
    >
      +{ov.count}
    </div>
  {/each}

  <NowMarker {win} {mainPx} {vertical} />

  <button class="clock" class:vertical onclick={() => openSettings().catch(() => {})}>
    <span>{wd}</span>
    <span>{dm}</span>
    <span>{hm}</span>
  </button>
</div>
