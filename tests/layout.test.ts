import { describe, it, expect } from "vitest";
import { assignLanes, layout, MAX_LANES } from "../src/lib/layout";
import type { NormalizedEvent } from "../src/lib/types";

function ev(id: string, startMin: number, endMin: number): NormalizedEvent {
  const base = Date.parse("2026-01-01T00:00:00Z");
  return {
    id,
    account_id: "a",
    source_id: "a",
    color: "#000",
    title: id,
    start: new Date(base + startMin * 60_000).toISOString(),
    end: new Date(base + endMin * 60_000).toISOString(),
    all_day: false,
    attendees: [],
  };
}

describe("lane assignment", () => {
  it("non-overlapping events share lane 0", () => {
    const out = assignLanes([ev("a", 0, 30), ev("b", 30, 60)]);
    expect(out.every((e) => e.lane === 0)).toBe(true);
    expect(out.every((e) => e.laneCount === 1)).toBe(true);
  });

  it("two overlapping events get lanes 0 and 1", () => {
    const out = assignLanes([ev("a", 0, 60), ev("b", 30, 90)]);
    const a = out.find((e) => e.id === "a")!;
    const b = out.find((e) => e.id === "b")!;
    expect(a.lane).toBe(0);
    expect(b.lane).toBe(1);
    expect(a.laneCount).toBe(2);
    expect(b.laneCount).toBe(2);
  });

  it("three-way overlap yields laneCount 3 across lanes 0,1,2", () => {
    const out = assignLanes([ev("a", 0, 60), ev("b", 10, 70), ev("c", 20, 80)]);
    expect(Math.max(...out.map((e) => e.laneCount))).toBe(3);
    expect(new Set(out.map((e) => e.lane))).toEqual(new Set([0, 1, 2]));
  });

  it("a freed lane is reused by a later non-overlapping event", () => {
    // a: 0-30, b: 10-40 (overlap → lanes 0,1); c: 35-60 overlaps only b → lane 0
    const out = assignLanes([ev("a", 0, 30), ev("b", 10, 40), ev("c", 35, 60)]);
    const c = out.find((e) => e.id === "c")!;
    expect(c.lane).toBe(0);
    expect(c.laneCount).toBe(2); // same cluster as a,b
  });

  it("separate clusters are independent", () => {
    const out = assignLanes([ev("a", 0, 30), ev("b", 10, 40), ev("c", 100, 130)]);
    const c = out.find((e) => e.id === "c")!;
    expect(c.lane).toBe(0);
    expect(c.laneCount).toBe(1);
  });
});

describe("overflow markers", () => {
  it("no overflow when concurrency <= MAX_LANES", () => {
    const { overflow } = layout([ev("a", 0, 60), ev("b", 0, 60), ev("c", 0, 60)]);
    expect(overflow.length).toBe(0);
  });

  it("emits one marker counting hidden events beyond MAX_LANES", () => {
    // 5 concurrent → MAX_LANES shown, rest hidden
    const evs = ["a", "b", "c", "d", "e"].map((id) => ev(id, 0, 60));
    const { overflow } = layout(evs);
    expect(overflow.length).toBe(1);
    expect(overflow[0].count).toBe(5 - MAX_LANES);
    expect(overflow[0].startMs).toBeLessThan(overflow[0].endMs);
  });

  it("does not merge separate clusters into one marker", () => {
    const groupA = ["a1", "a2", "a3", "a4"].map((id) => ev(id, 0, 60));
    const groupB = ["b1", "b2", "b3", "b4"].map((id) => ev(id, 200, 260));
    const { overflow } = layout([...groupA, ...groupB]);
    expect(overflow.length).toBe(2);
  });
});
