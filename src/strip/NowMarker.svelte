<script lang="ts">
  import { xOf, type TimeWindow } from "../lib/time";

  let {
    win,
    mainPx,
    vertical,
  }: { win: TimeWindow; mainPx: number; vertical: boolean } = $props();

  const pos = $derived(xOf(win.now, win, mainPx));
</script>

<div
  class="now-marker"
  class:vertical
  style={vertical ? `top:${pos}px` : `left:${pos}px`}
></div>
