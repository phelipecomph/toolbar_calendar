<script lang="ts">
  import { xOf, widthOf, type TimeWindow } from "../lib/time";
  import { MAX_LANES } from "../lib/layout";
  import { openDetail } from "../lib/ipc";
  import type { LaidOutEvent } from "../lib/types";

  let {
    ev,
    win,
    widthPx,
  }: { ev: LaidOutEvent; win: TimeWindow; widthPx: number } = $props();

  const startMs = $derived(Date.parse(ev.start));
  const endMs = $derived(Date.parse(ev.end));

  // Cap de legibilidade: lanes além de MAX_LANES ficam ocultas (badge +N é Fase futura).
  const lanes = $derived(Math.min(ev.laneCount, MAX_LANES));
  const hidden = $derived(ev.lane >= MAX_LANES);
  const laneH = $derived(100 / lanes); // % da altura da faixa

  const left = $derived(xOf(startMs, win, widthPx));
  const width = $derived(widthOf(startMs, endMs, win, widthPx));
  const top = $derived(ev.lane * laneH);

  function onClick(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    openDetail(ev.id, r).catch((err) => console.error("open_detail", err));
  }
</script>

{#if !hidden}
  <button
    class="event"
    style="left:{left}px; width:{width}px; top:{top}%; height:{laneH}%; background:{ev.color};"
    onclick={onClick}
    title={ev.title}
    aria-label={ev.title}
  ></button>
{/if}
