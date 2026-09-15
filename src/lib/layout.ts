import type { NormalizedEvent, LaidOutEvent } from "./types";

// Overlapping events stack in Y (the strip's height). laneCount decides row height.
// Beyond MAX_LANES the strip is too thin to read — those lanes are hidden and
// surfaced via a "+N" overflow badge (see overflow markers below + Strip.svelte).
export const MAX_LANES = 3;

interface Item {
  ev: NormalizedEvent;
  start: number;
  end: number;
  lane: number;
  laneCount: number;
}

/** A cluster with more than MAX_LANES concurrent events: how many were hidden and where. */
export interface OverflowMarker {
  startMs: number;
  endMs: number;
  count: number;
}

function toItems(events: NormalizedEvent[]): Item[] {
  const items: Item[] = events.map((ev) => ({
    ev,
    start: Date.parse(ev.start),
    end: Date.parse(ev.end),
    lane: 0,
    laneCount: 1,
  }));
  items.sort((a, b) => a.start - b.start || a.end - b.end);
  return items;
}

/** Split into clusters of transitive overlap. */
function clusters(items: Item[]): Item[][] {
  const out: Item[][] = [];
  let cur: Item[] = [];
  let end = -Infinity;
  for (const it of items) {
    if (cur.length && it.start >= end) {
      out.push(cur);
      cur = [];
      end = -Infinity;
    }
    cur.push(it);
    end = Math.max(end, it.end);
  }
  if (cur.length) out.push(cur);
  return out;
}

/** Greedy first-fit lane assignment inside one cluster; sets lane + shared laneCount. */
function layoutCluster(cluster: Item[]) {
  const laneEnds: number[] = [];
  for (const it of cluster) {
    let placed = false;
    for (let i = 0; i < laneEnds.length; i++) {
      if (it.start >= laneEnds[i]) {
        it.lane = i;
        laneEnds[i] = it.end;
        placed = true;
        break;
      }
    }
    if (!placed) {
      it.lane = laneEnds.length;
      laneEnds.push(it.end);
    }
  }
  const laneCount = laneEnds.length;
  for (const it of cluster) it.laneCount = laneCount;
}

/** Full layout: laid-out events + overflow markers for clusters exceeding MAX_LANES. */
export function layout(events: NormalizedEvent[]): {
  laid: LaidOutEvent[];
  overflow: OverflowMarker[];
} {
  const items = toItems(events);
  const overflow: OverflowMarker[] = [];

  for (const cluster of clusters(items)) {
    layoutCluster(cluster);
    const hidden = cluster.filter((it) => it.lane >= MAX_LANES).length;
    if (hidden > 0) {
      overflow.push({
        startMs: Math.min(...cluster.map((it) => it.start)),
        endMs: Math.max(...cluster.map((it) => it.end)),
        count: hidden,
      });
    }
  }

  const laid = items.map((it) => ({ ...it.ev, lane: it.lane, laneCount: it.laneCount }));
  return { laid, overflow };
}

/**
 * Assign each event a lane (row) index and the lane count of its collision cluster.
 * Thin wrapper over `layout` kept for callers/tests that only need the laid events.
 */
export function assignLanes(events: NormalizedEvent[]): LaidOutEvent[] {
  return layout(events).laid;
}
