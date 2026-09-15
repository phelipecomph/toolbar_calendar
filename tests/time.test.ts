import { describe, it, expect } from "vitest";
import { makeWindow, fractionOf, xOf, widthOf, isVisible } from "../src/lib/time";

describe("time window", () => {
  const now = 1_000_000_000_000;
  const w = makeWindow(now, 60, 480); // -1h .. +8h => 9h span

  it("now sits at 1/9 of the window", () => {
    expect(fractionOf(now, w)).toBeCloseTo(1 / 9, 5);
  });

  it("window start is fraction 0, end is 1", () => {
    expect(fractionOf(w.from, w)).toBe(0);
    expect(fractionOf(w.to, w)).toBe(1);
  });

  it("xOf maps to pixels", () => {
    expect(xOf(w.from, w, 900)).toBe(0);
    expect(xOf(w.to, w, 900)).toBe(900);
    expect(xOf(now, w, 900)).toBeCloseTo(100, 5); // 1/9 * 900
  });

  it("widthOf clamps to >= 1px and scales linearly", () => {
    expect(widthOf(now, now, w, 900)).toBe(1);
    // 1h over a 9h span at 900px = 100px
    expect(widthOf(now, now + 3_600_000, w, 900)).toBeCloseTo(100, 5);
  });

  it("isVisible detects intersection with the window", () => {
    expect(isVisible(now, now + 1000, w)).toBe(true);
    expect(isVisible(w.from - 10_000, w.from - 5_000, w)).toBe(false);
    expect(isVisible(w.to + 1_000, w.to + 2_000, w)).toBe(false);
  });
});
