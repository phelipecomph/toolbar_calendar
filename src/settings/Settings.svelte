<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getMonitors, setDock } from "../lib/ipc";
  import type { AppConfig, Edge, MonitorDto } from "../lib/types";

  let monitors = $state<MonitorDto[]>([]);
  let selMonitor = $state(0);
  let selEdge = $state<Edge>("bottom");

  const EDGES: { key: Edge; label: string }[] = [
    { key: "top", label: "Topo" },
    { key: "bottom", label: "Base" },
    { key: "left", label: "Esquerda" },
    { key: "right", label: "Direita" },
  ];

  async function refresh(cfg?: AppConfig) {
    try {
      monitors = await getMonitors();
    } catch {
      monitors = [];
    }
    if (cfg) {
      selMonitor = cfg.monitor_index;
      selEdge = cfg.edge;
    }
  }

  async function apply(edge: Edge, monIdx: number) {
    selEdge = edge;
    selMonitor = monIdx;
    try {
      await setDock(monIdx, edge);
    } catch (e) {
      console.error("set_dock", e);
    }
  }

  onMount(() => {
    const w = getCurrentWindow();
    refresh();
    const un1 = listen<AppConfig>("settings://open", (e) => refresh(e.payload));
    const un2 = w.onFocusChanged(({ payload: focused }) => {
      if (!focused) w.hide();
    });
    return () => {
      un1.then((u) => u());
      un2.then((u) => u());
    };
  });
</script>

<div class="settings">
  <div class="st-title">Posição da faixa</div>

  <div class="st-label">Monitor</div>
  <div class="st-monitors">
    {#each monitors as m (m.index)}
      <button
        class="st-btn"
        class:sel={m.index === selMonitor}
        onclick={() => apply(selEdge, m.index)}
        title={`${m.width}×${m.height}`}
      >
        {m.name} · {m.width}×{m.height}
      </button>
    {/each}
    {#if monitors.length === 0}
      <div class="st-empty">nenhum monitor</div>
    {/if}
  </div>

  <div class="st-label">Canto</div>
  <div class="st-edges">
    {#each EDGES as e (e.key)}
      <button
        class="st-btn"
        class:sel={e.key === selEdge}
        onclick={() => apply(e.key, selMonitor)}
      >
        {e.label}
      </button>
    {/each}
  </div>
</div>
