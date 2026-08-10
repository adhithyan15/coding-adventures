// Timeline (Gantt) geometry.
//
// Pure arithmetic, deliberately separated from the controller so it can be tested
// without a wasm engine or a DOM. The ENGINE decides when every task starts and
// finishes and which are critical; this file only answers "how far across the track
// is that, as a percentage".
//
// The one thing to know before reading: **task-core dates are day-granular with an
// INCLUSIVE finish.** A task occupying a single day reports `finish === start`. So a
// length is `finish - start + 1`, never the bare difference — getting that wrong makes
// every one-day task zero-width, which renders the whole chart as a row of slivers.

/** The engine's gantt bar, narrowed to the fields the geometry needs. */
export interface GanttBar {
  name: string;
  /** Days since the Unix epoch, inclusive. */
  start: number;
  /** Days since the Unix epoch, inclusive — equals `start` for a one-day task. */
  finish: number;
  critical: boolean;
  /**
   * 0..=100. Optional (defaults to 0) so existing fixtures/tests that predate
   * this field keep compiling — real engine payloads always send it.
   */
  percentComplete?: number;
  /**
   * Optional (defaults to "leaf") for the same reason as `percentComplete`. A
   * "milestone" bar renders as a diamond instead of the usual proportional bar
   * — see task-app-richer-gantt-v1.md.
   */
  kind?: "leaf" | "summary" | "milestone";
}

/**
 * A row as the layout consumes it:
 * `[ name, padWidth, barWidth, window, critical, milestone, percentComplete, tooltip ]`.
 * Appended fields (milestone/percentComplete/tooltip), not inserted — no
 * existing `t[n]` reference in TaskApp.mll shifts meaning.
 */
export type TimelineRow = [
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
];

/** One calendar day of the day-grid: `[ widthPct, weekend, today ]`. */
export type TimelineGridCell = [string, string, string];

export interface TimelineView {
  /** The date ruler shown above the bars. */
  scale: string;
  rows: TimelineRow[];
  /** One cell per calendar day in the visible span — see task-app-richer-gantt-v1.md. */
  grid: TimelineGridCell[];
}

/**
 * The narrowest a bar may render, as a fraction of the whole span. A true milestone
 * has no duration at all; without a floor it would be invisible.
 */
const MIN_BAR_FRACTION = 0.004;

const DAY_MS = 86_400_000;

/**
 * The day-grid renders one element per calendar day in the span — reasonable
 * for the years-long projects this app is actually for, but nothing bounds
 * `span` itself: a single typo in the due-date composer (`main.tsx`'s
 * `isoToDays` accepts any 4-digit year, e.g. a "0202" for "2026") reaches a
 * multi-million-day span with no malice required. Past this cap, the grid is
 * skipped — the bars and the scale caption above them still render — rather
 * than iterating millions of times and asking the DOM to hold that many
 * elements. ~27 years is generous headroom over any real project's length.
 */
const MAX_GRID_DAYS = 10_000;

/** `days since epoch` → `YYYY-MM-DD`, or `"—"` for a date JavaScript can't represent. */
export function dayToIso(days: number): string {
  if (!Number.isFinite(days)) return "—";
  const ms = days * DAY_MS;
  // `new Date(x).toISOString()` THROWS outside ±8.64e15 ms. A corrupted snapshot
  // shouldn't take the render down with it.
  if (Math.abs(ms) > 8.64e15) return "—";
  return new Date(ms).toISOString().slice(0, 10);
}

/**
 * Lay the engine's bars out on one shared date scale.
 *
 * Returns render-ready percentage strings; an empty or unusable bar list yields an
 * empty view with an explanatory scale rather than throwing. `today` (days since
 * epoch) highlights that day's grid column — omit it (or pass a day outside the
 * visible span) and no column is marked "today".
 */
export function buildTimeline(bars: readonly GanttBar[], today = NaN): TimelineView {
  // Ignore bars the engine couldn't place — a NaN would poison min/max and every
  // percentage downstream.
  const usable = bars.filter((b) => Number.isFinite(b.start) && Number.isFinite(b.finish));
  if (usable.length === 0) return { scale: "Nothing scheduled yet.", rows: [], grid: [] };

  // `reduce`, not `Math.min(...bars)`: spreading a large array throws
  // "too many arguments" somewhere past ~100k elements.
  const first = usable.reduce((m, b) => Math.min(m, b.start), usable[0].start);
  const last = usable.reduce((m, b) => Math.max(m, b.finish), usable[0].finish);

  // Inclusive: a project that starts and ends on the same day spans one day, not zero.
  const span = Math.max(1, last - first + 1);
  const pct = (days: number) => `${((days / span) * 100).toFixed(2)}%`;
  // Every grid cell is exactly one day wide, as a fraction of the shared track —
  // the same `pct` a bar's own width uses, so the grid and the bars agree on scale.
  const dayWidth = pct(1);

  // One cell per calendar day in [first, last]. `getUTCDay()` (0 = Sunday, 6 =
  // Saturday) is enough for weekend shading; there's no per-project week-start
  // setting wired through here yet (ProjectSettings.weekStart exists but nothing
  // in the host reads it today — a pre-existing gap, not something this widens).
  // See MAX_GRID_DAYS above for why this loop is bounded.
  const grid: TimelineGridCell[] = [];
  if (span <= MAX_GRID_DAYS) {
    for (let day = first; day <= last; day += 1) {
      const weekday = new Date(day * DAY_MS).getUTCDay();
      const isWeekend = weekday === 0 || weekday === 6;
      grid.push([dayWidth, isWeekend ? "weekend" : "", day === today ? "today" : ""]);
    }
  }

  return {
    scale: `${dayToIso(first)} → ${dayToIso(last)} · ${span} day${span === 1 ? "" : "s"}`,
    grid,
    rows: usable.map((b): TimelineRow => {
      // Clamp defensively: a malformed bar with finish < start would otherwise produce
      // a negative width, and one starting before `first` a negative pad.
      const length = Math.max(b.finish - b.start + 1, span * MIN_BAR_FRACTION);
      const pad = Math.max(0, b.start - first);
      // The RAW inclusive day count, not the width-floored `length` above — a
      // tooltip should say how long the task really is, even for a bar too
      // short to show its true width without the visual floor.
      const rawDays = Math.max(1, b.finish - b.start + 1);
      const percent = Math.max(0, Math.min(100, Math.round(b.percentComplete ?? 0)));
      const tooltip =
        `${b.name}: ${dayToIso(b.start)} → ${dayToIso(b.finish)}` +
        ` (${rawDays} day${rawDays === 1 ? "" : "s"})` +
        (b.critical ? " — on the critical path" : "") +
        ` · ${percent}% complete`;
      return [
        b.name,
        pct(pad),
        pct(Math.min(length, span - pad)),
        `${dayToIso(b.start)} → ${dayToIso(b.finish)}`,
        b.critical ? "critical" : "",
        b.kind === "milestone" ? "milestone" : "",
        `${percent}%`,
        tooltip,
      ];
    }),
  };
}
