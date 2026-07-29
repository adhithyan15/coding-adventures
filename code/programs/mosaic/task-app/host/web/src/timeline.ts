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
}

/** A row as the layout consumes it: `[ name, padWidth, barWidth, window, critical ]`. */
export type TimelineRow = [string, string, string, string, string];

export interface TimelineView {
  /** The date ruler shown above the bars. */
  scale: string;
  rows: TimelineRow[];
}

/**
 * The narrowest a bar may render, as a fraction of the whole span. A true milestone
 * has no duration at all; without a floor it would be invisible.
 */
const MIN_BAR_FRACTION = 0.004;

const DAY_MS = 86_400_000;

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
 * empty view with an explanatory scale rather than throwing.
 */
export function buildTimeline(bars: readonly GanttBar[]): TimelineView {
  // Ignore bars the engine couldn't place — a NaN would poison min/max and every
  // percentage downstream.
  const usable = bars.filter((b) => Number.isFinite(b.start) && Number.isFinite(b.finish));
  if (usable.length === 0) return { scale: "Nothing scheduled yet.", rows: [] };

  // `reduce`, not `Math.min(...bars)`: spreading a large array throws
  // "too many arguments" somewhere past ~100k elements.
  const first = usable.reduce((m, b) => Math.min(m, b.start), usable[0].start);
  const last = usable.reduce((m, b) => Math.max(m, b.finish), usable[0].finish);

  // Inclusive: a project that starts and ends on the same day spans one day, not zero.
  const span = Math.max(1, last - first + 1);
  const pct = (days: number) => `${((days / span) * 100).toFixed(2)}%`;

  return {
    scale: `${dayToIso(first)} → ${dayToIso(last)} · ${span} day${span === 1 ? "" : "s"}`,
    rows: usable.map((b): TimelineRow => {
      // Clamp defensively: a malformed bar with finish < start would otherwise produce
      // a negative width, and one starting before `first` a negative pad.
      const length = Math.max(b.finish - b.start + 1, span * MIN_BAR_FRACTION);
      const pad = Math.max(0, b.start - first);
      return [
        b.name,
        pct(pad),
        pct(Math.min(length, span - pad)),
        `${dayToIso(b.start)} → ${dayToIso(b.finish)}`,
        b.critical ? "critical" : "",
      ];
    }),
  };
}
