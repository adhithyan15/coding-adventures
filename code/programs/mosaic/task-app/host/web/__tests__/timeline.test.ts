/**
 * timeline.test.ts — Gantt geometry.
 *
 * The rule that governs every assertion here: **task-core dates are day-granular with
 * an inclusive finish**, so a task occupying one day reports `finish === start`. An
 * earlier draft used the bare difference, which made every one-day task zero-width —
 * it fell through to the milestone floor and the whole chart rendered as slivers. Most
 * of these tests exist to keep that from coming back.
 */
import { describe, it, expect } from "vitest";
import { buildTimeline, dayToIso, type GanttBar } from "../src/timeline";

const bar = (name: string, start: number, finish: number, critical = false): GanttBar => ({
  name,
  start,
  finish,
  critical,
});
const pctOf = (s: string) => parseFloat(s);

describe("timeline geometry", () => {
  it("says so plainly when nothing is scheduled", () => {
    expect(buildTimeline([])).toEqual({ scale: "Nothing scheduled yet.", rows: [] });
  });

  it("gives a one-day project a full-width bar, not a sliver", () => {
    // THE regression. finish === start is one day, not zero.
    const view = buildTimeline([bar("Only", 100, 100)]);
    expect(view.rows).toHaveLength(1);
    expect(view.rows[0][1]).toBe("0.00%");
    expect(view.rows[0][2]).toBe("100.00%");
    expect(view.scale).toContain("1 day");
  });

  it("counts an inclusive span in days", () => {
    // 100..101 inclusive is two days.
    const view = buildTimeline([bar("Two", 100, 101)]);
    expect(view.scale).toContain("2 days");
    expect(view.rows[0][2]).toBe("100.00%");
  });

  it("lays a chain out left to right without overflowing the track", () => {
    const view = buildTimeline([
      bar("A", 10, 11),
      bar("B", 12, 13),
      bar("C", 14, 15),
    ]);
    const pads = view.rows.map((r) => pctOf(r[1]));
    const widths = view.rows.map((r) => pctOf(r[2]));

    expect(pads[0]).toBe(0);
    expect(pads[0]).toBeLessThan(pads[1]);
    expect(pads[1]).toBeLessThan(pads[2]);
    widths.forEach((w) => expect(w).toBeGreaterThan(0));
    view.rows.forEach((_, i) => expect(pads[i] + widths[i]).toBeLessThanOrEqual(100.01));
    // Three equal 2-day tasks over a 6-day span.
    widths.forEach((w) => expect(w).toBeCloseTo(100 / 3, 1));
  });

  it("keeps a zero-duration milestone visible", () => {
    // A milestone shares its day with a long task, so its natural width is a rounding
    // error of the span — the floor is what keeps it on screen.
    const view = buildTimeline([bar("Long", 0, 99), bar("Launch", 99, 99)]);
    const milestone = view.rows.find((r) => r[0] === "Launch")!;
    expect(pctOf(milestone[2])).toBeGreaterThan(0);
  });

  it("marks the critical path", () => {
    const view = buildTimeline([bar("Crit", 0, 1, true), bar("Slack", 0, 1, false)]);
    expect(view.rows[0][4]).toBe("critical");
    expect(view.rows[1][4]).toBe("");
  });

  it("reports each bar's own window", () => {
    const view = buildTimeline([bar("A", 19723, 19724)]); // 2024-01-01 → 2024-01-02
    expect(view.rows[0][3]).toBe("2024-01-01 → 2024-01-02");
  });

  // ---- malformed engine output: degrade, never throw or render nonsense ----

  it("ignores bars the engine couldn't place", () => {
    const view = buildTimeline([
      bar("Good", 5, 6),
      { name: "Bad", start: NaN, finish: 6, critical: false },
    ]);
    expect(view.rows.map((r) => r[0])).toEqual(["Good"]);
  });

  it("falls back to the empty view when every bar is unplaceable", () => {
    const view = buildTimeline([{ name: "Bad", start: NaN, finish: NaN, critical: false }]);
    expect(view.rows).toHaveLength(0);
  });

  it("never emits a negative or overflowing width for an inverted bar", () => {
    // finish < start shouldn't be possible from the engine, but a corrupted snapshot
    // must not produce a negative CSS width.
    const view = buildTimeline([bar("Sane", 0, 9), bar("Inverted", 5, 3)]);
    view.rows.forEach((r) => {
      expect(pctOf(r[1])).toBeGreaterThanOrEqual(0);
      expect(pctOf(r[2])).toBeGreaterThan(0);
      expect(pctOf(r[1]) + pctOf(r[2])).toBeLessThanOrEqual(100.01);
    });
  });

  it("survives a date JavaScript can't represent", () => {
    // `new Date(x).toISOString()` throws beyond ±8.64e15 ms; the render must not die.
    expect(dayToIso(1e15)).toBe("—");
    expect(dayToIso(Number.POSITIVE_INFINITY)).toBe("—");
    expect(() => buildTimeline([bar("Far", 1e15, 1e15)])).not.toThrow();
  });

  it("handles a large bar list without spreading it into an argument list", () => {
    // Math.min(...arr) throws "too many arguments" at this size; reduce doesn't.
    const many = Array.from({ length: 200_000 }, (_, i) => bar(`T${i}`, i, i));
    expect(() => buildTimeline(many)).not.toThrow();
  });
});
