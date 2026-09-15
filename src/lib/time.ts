// Time-window math for the horizontal timeline. X axis = time.
// Window is relative to "now": [now - before, now + after].

export interface TimeWindow {
  from: number; // epoch ms
  to: number; // epoch ms
  now: number; // epoch ms
}

export function makeWindow(nowMs: number, beforeMin: number, afterMin: number): TimeWindow {
  return {
    from: nowMs - beforeMin * 60_000,
    to: nowMs + afterMin * 60_000,
    now: nowMs,
  };
}

/** Fraction 0..1 of a timestamp within the window (can go <0 / >1 off-window). */
export function fractionOf(ms: number, w: TimeWindow): number {
  return (ms - w.from) / (w.to - w.from);
}

/** X pixel of a timestamp given the strip width. */
export function xOf(ms: number, w: TimeWindow, widthPx: number): number {
  return fractionOf(ms, w) * widthPx;
}

/** Width in px of an interval, clamped to >= 1px so zero-length events stay visible. */
export function widthOf(startMs: number, endMs: number, w: TimeWindow, widthPx: number): number {
  const px = xOf(endMs, w, widthPx) - xOf(startMs, w, widthPx);
  return Math.max(1, px);
}

/** Does the interval intersect the window at all? */
export function isVisible(startMs: number, endMs: number, w: TimeWindow): boolean {
  return endMs > w.from && startMs < w.to;
}
