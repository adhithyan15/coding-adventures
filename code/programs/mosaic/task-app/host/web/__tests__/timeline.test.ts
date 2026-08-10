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
    expect(buildTimeline([])).toEqual({ scale: "Nothing scheduled yet.", rows: [], grid: [] });
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

  it("skips the day-grid, but still renders bars and a scale, past MAX_GRID_DAYS", () => {
    // A single typo in the due-date composer (any 4-digit year) reaches a
    // multi-million-day span with no malice required — the grid must not try
    // to iterate or render that many elements. The bars/scale still work:
    // only the per-day ruler is skipped.
    const view = buildTimeline([bar("Short", 0, 1), bar("Far future", 20_000, 20_001)]);
    expect(view.grid).toHaveLength(0);
    expect(view.rows).toHaveLength(2);
    expect(view.scale).toContain("20002 days");
  });

  // ---- richer Gantt: day-grid, milestones, percent-complete, tooltips ----

  it("builds one grid cell per calendar day, each the same width as a one-day bar", () => {
    const view = buildTimeline([bar("A", 10, 11), bar("B", 12, 13)]); // span: 10..13, 4 days
    expect(view.grid).toHaveLength(4);
    view.grid.forEach((cell) => expect(cell[0]).toBe("25.00%"));
  });

  it("marks weekend columns — 19723 (2024-01-01) is a Monday", () => {
    // Sat 2024-01-06 = 19728, Sun 2024-01-07 = 19729.
    const view = buildTimeline([bar("Week", 19723, 19729)]);
    const weekdayFlags = view.grid.map((c) => c[1]);
    expect(weekdayFlags).toEqual(["", "", "", "", "", "weekend", "weekend"]);
  });

  it("marks exactly the grid column matching `today`", () => {
    const view = buildTimeline([bar("Span", 100, 104)], 102);
    expect(view.grid.map((c) => c[2])).toEqual(["", "", "today", "", ""]);
  });

  it("marks no column today when `today` falls outside the visible span", () => {
    const view = buildTimeline([bar("Span", 100, 104)], 200);
    expect(view.grid.every((c) => c[2] === "")).toBe(true);
  });

  it("marks a milestone-kind bar and leaves ordinary bars unmarked", () => {
    const view = buildTimeline([
      { ...bar("Ship", 5, 5), kind: "milestone" },
      { ...bar("Build", 0, 4), kind: "leaf" },
    ]);
    const ship = view.rows.find((r) => r[0] === "Ship")!;
    const build = view.rows.find((r) => r[0] === "Build")!;
    expect(ship[5]).toBe("milestone");
    expect(build[5]).toBe("");
  });

  it("defaults an omitted kind to a non-milestone bar", () => {
    const view = buildTimeline([bar("Plain", 0, 1)]);
    expect(view.rows[0][5]).toBe("");
  });

  it("reports percent-complete as a CSS-ready percentage, clamped to 0..100", () => {
    const view = buildTimeline([
      { ...bar("Half", 0, 1), percentComplete: 50 },
      { ...bar("Over", 0, 1), percentComplete: 150 },
      { ...bar("Under", 0, 1), percentComplete: -10 },
      bar("Unset", 0, 1),
    ]);
    const byName = (n: string) => view.rows.find((r) => r[0] === n)!;
    expect(byName("Half")[6]).toBe("50%");
    expect(byName("Over")[6]).toBe("100%");
    expect(byName("Under")[6]).toBe("0%");
    expect(byName("Unset")[6]).toBe("0%");
  });

  it("writes a tooltip with the real day count, not the width-floored one", () => {
    // A milestone sharing a day with a long task gets width-floored (see "keeps a
    // zero-duration milestone visible" above) but its tooltip must still say 1 day,
    // not the floor's fractional value.
    const view = buildTimeline([
      { ...bar("Long", 0, 99), percentComplete: 40 },
      { ...bar("Launch", 99, 99, true), kind: "milestone" },
    ]);
    const launch = view.rows.find((r) => r[0] === "Launch")!;
    expect(launch[7]).toBe(
      "Launch: 1970-04-10 → 1970-04-10 (1 day) — on the critical path · 0% complete",
    );
    const long = view.rows.find((r) => r[0] === "Long")!;
    expect(long[7]).toBe("Long: 1970-01-01 → 1970-04-10 (100 days) · 40% complete");
  });
});
