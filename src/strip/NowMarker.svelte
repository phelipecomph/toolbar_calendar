<script lang="ts">
  import { xOf, type TimeWindow } from "../lib/time";

  let { win, widthPx }: { win: TimeWindow; widthPx: number } = $props();

  const left = $derived(xOf(win.now, win, widthPx));
</script>

<div class="now-marker" style="left:{left}px"></div>
