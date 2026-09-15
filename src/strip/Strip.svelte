<script lang="ts">
  import { onMount } from "svelte";
  import EventBlock from "./EventBlock.svelte";
  import NowMarker from "./NowMarker.svelte";
  import { layout } from "../lib/layout";
  import { makeWindow, isVisible, xOf } from "../lib/time";
  import { getEvents, onEventsUpdated, triggerSync } from "../lib/ipc";
  import { MOCK_EVENTS } from "../lib/mocks";
  import type { NormalizedEvent } from "../lib/types";

  // Fase 1: janela fixa. Fase 4 lê de AppConfig.
  const BEFORE_MIN = 60;
  const AFTER_MIN = 480;

  let raw = $state<NormalizedEvent[]>([]);
  let now = $state(Date.now());
  let widthPx = $state(0);

  const win = $derived(makeWindow(now, BEFORE_MIN, AFTER_MIN));
  const view = $derived(
    layout(raw.filter((e) => isVisible(Date.parse(e.start), Date.parse(e.end), win)))
  );

  onMount(() => {
    // Carrega eventos: tenta o backend; cai p/ mocks locais no preview de browser.
    (async () => {
      try {
        const from = new Date(now - BEFORE_MIN * 60_000).toISOString();
        const to = new Date(now + AFTER_MIN * 60_000).toISOString();
        raw = await getEvents(from, to);
      } catch {
        raw = MOCK_EVENTS(now);
      }
    })();

    const unlistenP = onEventsUpdated((ev) => (raw = ev)).catch(() => null);

    // Já inscrito no evento → força um sync agora (evita corrida com o sync do startup).
    triggerSync().catch(() => {});

    // Marcador de "agora" e deslize da janela.
    const tick = setInterval(() => (now = Date.now()), 30_000);

    return () => {
      clearInterval(tick);
      unlistenP.then((u) => u && u());
    };
  });
</script>

<div class="strip" bind:clientWidth={widthPx}>
  {#each view.laid as ev (ev.id)}
    <EventBlock {ev} {win} {widthPx} />
  {/each}
  {#each view.overflow as ov (ov.startMs)}
    <div
      class="overflow-badge"
      style="left:{Math.min(Math.max(xOf(ov.endMs, win, widthPx) - 20, 0), widthPx - 20)}px"
      title="+{ov.count} evento(s) sobrepostos ocultos"
    >
      +{ov.count}
    </div>
  {/each}
  <NowMarker {win} {widthPx} />
</div>
