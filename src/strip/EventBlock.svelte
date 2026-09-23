<script lang="ts">
  import { xOf, widthOf, type TimeWindow } from "../lib/time";
  import { MAX_LANES } from "../lib/layout";
  import { openDetail } from "../lib/ipc";
  import type { LaidOutEvent } from "../lib/types";

  let {
    ev,
    win,
    mainPx,
    vertical,
  }: { ev: LaidOutEvent; win: TimeWindow; mainPx: number; vertical: boolean } = $props();

  const startMs = $derived(Date.parse(ev.start));
  const endMs = $derived(Date.parse(ev.end));

  // Cap de legibilidade: lanes além de MAX_LANES ficam ocultas (badge +N).
  const lanes = $derived(Math.min(ev.laneCount, MAX_LANES));
  const hidden = $derived(ev.lane >= MAX_LANES);
  const laneSize = $derived(100 / lanes); // % do eixo cruzado

  const mainPos = $derived(xOf(startMs, win, mainPx)); // posição no eixo do tempo
  const mainLen = $derived(widthOf(startMs, endMs, win, mainPx));
  const crossPos = $derived(ev.lane * laneSize);

  // Eixo do tempo = X (horizontal) ou Y (vertical). Lanes no eixo cruzado.
  const style = $derived(
    vertical
      ? `top:${mainPos}px; height:${mainLen}px; left:${crossPos}%; width:${laneSize}%; background:${ev.color};`
      : `left:${mainPos}px; width:${mainLen}px; top:${crossPos}%; height:${laneSize}%; background:${ev.color};`
  );

  function onClick(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    openDetail(ev.id, r).catch((err) => console.error("open_detail", err));
  }
</script>

{#if !hidden}
  <button class="event" {style} onclick={onClick} title={ev.title} aria-label={ev.title}
  ></button>
{/if}
