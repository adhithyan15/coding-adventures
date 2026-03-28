/**
 * CalendarView — A read-only monthly calendar grid.
 *
 * Generic over T (any object with id + dueDate). The caller supplies a
 * `renderItem` function that controls how each item appears inside a day
 * cell. The calendar itself only handles navigation (prev/next month, jump
 * to today) and date-to-item indexing.
 *
 * === Why generic? ===
 *
 * A calendar is a general-purpose layout tool. Today it renders TodoItems.
 * Tomorrow it might render calendar events, project milestones, invoice due
 * dates, or medication reminders. By accepting `T extends CalendarItem` and
 * a render prop, we separate the layout concern (which day cells exist,
 * which items fall on which day) from the display concern (what an item
 * looks like inside a cell). This is the same pattern as a generic list
 * component that accepts a `renderRow` prop.
 *
 * === Grid construction ===
 *
 * A month view is a 6-row × 7-column grid. We always render 6 weeks (42
 * cells) so the layout height is stable as the user navigates between
 * months. Here is how the grid is built:
 *
 *   1. Find the 1st of the displayed month.
 *   2. Find which day-of-week the 1st falls on (0 = Sunday … 6 = Saturday).
 *   3. Walk back to the nearest preceding Sunday — this is gridStart.
 *   4. Build 42 consecutive Date objects starting at gridStart.
 *
 * Example — March 2026 (1st is a Sunday):
 *
 *   Su  Mo  Tu  We  Th  Fr  Sa
 *    1   2   3   4   5   6   7   ← all current-month
 *    8   9  10  11  12  13  14
 *   15  16  17  18  19  20  21
 *   22  23  24  25  26  27  28
 *   29  30  31   1   2   3   4   ← Apr days are "other-month"
 *    5   6   7   8   9  10  11   ← 6th week fills the grid
 *
 * Days that belong to the previous or next month are rendered with
 * reduced opacity ("other-month" modifier class). Today gets a highlighted
 * circle and `aria-current="date"`.
 *
 * === Item indexing ===
 *
 * Items are indexed into a `Map<YYYY-MM-DD, T[]>` on every render using
 * `useMemo`. This gives O(1) per-cell lookups even with hundreds of items.
 * Items whose `dueDate` is null are silently omitted — they have no
 * calendar position.
 *
 * === Accessibility ===
 *
 * - Outer wrapper: `role="region"` with `aria-label`
 * - Grid: `role="grid"` with `role="row"` / `role="gridcell"` / `role="columnheader"`
 * - Navigation buttons: descriptive `aria-label` ("Previous month", etc.)
 * - Today's cell: `aria-current="date"`
 * - Each cell: `aria-label` includes full date string + item count
 */

import React, { useState, useMemo } from "react";

// ── Public types ────────────────────────────────────────────────────────────

/**
 * The minimum shape any item must satisfy to appear on the calendar.
 *
 * - `id`      — Used as the React key when rendering the item list inside
 *               a cell. Must be unique across all items.
 * - `dueDate` — ISO 8601 date string ("YYYY-MM-DD") or null. Items with
 *               null are omitted from the calendar grid.
 */
export interface CalendarItem {
  id: string;
  dueDate: string | null;
}

export interface CalendarViewProps<T extends CalendarItem> {
  /**
   * All items to display. Each item is placed in the cell matching its
   * `dueDate`. Items with `dueDate = null` are silently skipped.
   */
  items: T[];

  /**
   * Render function for each item inside a day cell.
   * Keep renders compact — day cells have limited vertical space.
   *
   * Example:
   * ```tsx
   * renderItem={(todo) => (
   *   <span className="cal-chip cal-chip--high">{todo.title}</span>
   * )}
   * ```
   */
  renderItem: (item: T) => React.ReactNode;

  /**
   * Override the initially displayed year.
   * Defaults to the current year at mount time.
   */
  initialYear?: number;

  /**
   * Override the initially displayed month (0 = January, 11 = December).
   * Defaults to the current month at mount time.
   */
  initialMonth?: number;

  /** Additional CSS class applied to the root element. */
  className?: string;

  /** Accessible label for the `role="region"` wrapper. Default: "Calendar". */
  ariaLabel?: string;
}

// ── Internal constants ──────────────────────────────────────────────────────

/**
 * Column headers for the day-of-week row.
 * Index 0 = Sunday, consistent with `Date.getDay()`.
 */
const DAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const;

// ── Date helpers ────────────────────────────────────────────────────────────

/**
 * Convert a Date to an ISO date string (YYYY-MM-DD) in local time.
 *
 * We cannot use `date.toISOString()` because that converts to UTC, which
 * shifts the date for users in timezones behind UTC. For example, a user
 * in UTC-5 at 11 PM on March 15 would see `2026-03-16T04:00:00Z` — wrong
 * day. Using `getFullYear`, `getMonth`, `getDate` always gives local time.
 */
function toDateString(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/**
 * Build the 42-cell (6 week × 7 day) flat array of Date objects for the
 * given year and month.
 *
 * The algorithm:
 *   1. `firstOfMonth = new Date(year, month, 1)`
 *   2. `startDow = firstOfMonth.getDay()` — 0–6 (Sun–Sat)
 *   3. `gridStart = new Date(year, month, 1 - startDow)` — the Sunday anchor
 *   4. Append 42 consecutive days starting from gridStart
 *
 * Using `new Date(year, month, day)` with day values outside [1..31] is
 * perfectly valid JavaScript — the engine rolls over into adjacent months.
 * E.g., `new Date(2026, 2, 0)` = Feb 28, 2026 (last day of February).
 */
function buildGrid(year: number, month: number): Date[] {
  const firstOfMonth = new Date(year, month, 1);
  const startDow = firstOfMonth.getDay(); // 0=Sun … 6=Sat
  const cells: Date[] = [];

  for (let i = 0; i < 42; i++) {
    // 1 - startDow can be 0 or negative — JS Date handles this gracefully
    cells.push(new Date(year, month, 1 - startDow + i));
  }

  return cells;
}

/**
 * Format a display header like "March 2026".
 *
 * Uses `Intl.DateTimeFormat` so month names are automatically localised
 * based on the user's browser locale.
 */
function formatMonthHeader(year: number, month: number): string {
  return new Intl.DateTimeFormat("en-US", {
    month: "long",
    year: "numeric",
  }).format(new Date(year, month, 1));
}

/**
 * Advance (or rewind) a year/month by `delta` months.
 *
 * `new Date(2026, -1, 1)` = December 2025 — JavaScript handles the
 * rollover, so we never need manual modular arithmetic on month/year.
 */
function addMonths(
  year: number,
  month: number,
  delta: number,
): { year: number; month: number } {
  const d = new Date(year, month + delta, 1);
  return { year: d.getFullYear(), month: d.getMonth() };
}

// ── Component ───────────────────────────────────────────────────────────────

/**
 * CalendarView renders a monthly grid. It is read-only — there are no
 * click handlers for creating or editing items. Navigation (prev/next
 * month, jump to today) is the only interaction.
 *
 * Example usage:
 * ```tsx
 * <CalendarView
 *   items={todos}
 *   renderItem={(todo) => (
 *     <span className={`chip chip--${todo.priority}`}>{todo.title}</span>
 *   )}
 *   ariaLabel="Todo calendar"
 * />
 * ```
 */
export function CalendarView<T extends CalendarItem>({
  items,
  renderItem,
  initialYear,
  initialMonth,
  className = "",
  ariaLabel = "Calendar",
}: CalendarViewProps<T>): React.ReactElement {
  // ── Snapshot today once on mount ────────────────────────────────────────
  // We capture today at mount time. Technically the date can change while
  // the component is mounted (midnight), but for a calendar view this edge
  // case is acceptable — a page refresh will fix it.
  const today = new Date();
  const todayStr = toDateString(today);

  // ── Which month is currently displayed ──────────────────────────────────
  const [year, setYear] = useState<number>(initialYear ?? today.getFullYear());
  const [month, setMonth] = useState<number>(initialMonth ?? today.getMonth());

  // ── Build the 42-cell grid for the displayed month ──────────────────────
  // Recomputed only when year or month changes (not on every items update).
  const grid = useMemo(() => buildGrid(year, month), [year, month]);

  // ── Index items by date for O(1) per-cell lookup ─────────────────────────
  //
  // We walk all items once and build a Map<YYYY-MM-DD → T[]>.
  // When rendering 30–42 cells per month view, this reduces the work from
  // O(cells × items) to O(items) for building + O(1) per cell for lookup.
  const itemsByDate = useMemo<Map<string, T[]>>(() => {
    const map = new Map<string, T[]>();
    for (const item of items) {
      if (item.dueDate === null) continue;
      const bucket = map.get(item.dueDate);
      if (bucket) {
        bucket.push(item);
      } else {
        map.set(item.dueDate, [item]);
      }
    }
    return map;
  }, [items]);

  // ── Navigation handlers ──────────────────────────────────────────────────
  function goToPrevMonth(): void {
    const next = addMonths(year, month, -1);
    setYear(next.year);
    setMonth(next.month);
  }

  function goToNextMonth(): void {
    const next = addMonths(year, month, 1);
    setYear(next.year);
    setMonth(next.month);
  }

  function goToToday(): void {
    setYear(today.getFullYear());
    setMonth(today.getMonth());
  }

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div
      className={["calendar-view", className].filter(Boolean).join(" ")}
      role="region"
      aria-label={ariaLabel}
    >
      {/* ── Month navigation header ─────────────────────────────────────── */}
      <div className="calendar-view__header">
        <button
          type="button"
          className="calendar-view__nav-btn"
          onClick={goToPrevMonth}
          aria-label="Previous month"
        >
          ‹
        </button>

        <h2 className="calendar-view__month-label">
          {formatMonthHeader(year, month)}
        </h2>

        <button
          type="button"
          className="calendar-view__nav-btn"
          onClick={goToNextMonth}
          aria-label="Next month"
        >
          ›
        </button>

        <button
          type="button"
          className="calendar-view__today-btn"
          onClick={goToToday}
          aria-label="Go to today"
        >
          Today
        </button>
      </div>

      {/* ── Calendar grid ───────────────────────────────────────────────── */}
      <div className="calendar-view__grid" role="grid" aria-label={ariaLabel}>
        {/* Day-of-week column headers */}
        <div
          className="calendar-view__row calendar-view__row--header"
          role="row"
        >
          {DAY_LABELS.map((label) => (
            <div
              key={label}
              className="calendar-view__day-header"
              role="columnheader"
              aria-label={label}
            >
              {label}
            </div>
          ))}
        </div>

        {/*
         * Six week rows. We slice the flat 42-cell array into 7-cell chunks.
         * Array.from({ length: 6 }, (_, i) => i) gives us [0, 1, 2, 3, 4, 5].
         */}
        {Array.from({ length: 6 }, (_, weekIdx) => (
          <div key={weekIdx} className="calendar-view__row" role="row">
            {grid.slice(weekIdx * 7, weekIdx * 7 + 7).map((cellDate) => {
              const dateStr = toDateString(cellDate);
              const isCurrentMonth = cellDate.getMonth() === month;
              const isToday = dateStr === todayStr;
              const cellItems = itemsByDate.get(dateStr) ?? [];
              const itemCount = cellItems.length;

              // Build class list for the cell
              const cellClass = [
                "calendar-view__cell",
                isCurrentMonth
                  ? "calendar-view__cell--current-month"
                  : "calendar-view__cell--other-month",
                isToday ? "calendar-view__cell--today" : "",
                itemCount > 0 ? "calendar-view__cell--has-items" : "",
              ]
                .filter(Boolean)
                .join(" ");

              // Human-readable aria-label for the cell
              const cellLabel =
                itemCount > 0
                  ? `${dateStr}, ${itemCount} item${itemCount !== 1 ? "s" : ""}`
                  : dateStr;

              return (
                <div
                  key={dateStr}
                  className={cellClass}
                  role="gridcell"
                  aria-label={cellLabel}
                  aria-current={isToday ? "date" : undefined}
                >
                  {/* Day number — highlighted circle when today */}
                  <span className="calendar-view__day-number">
                    {cellDate.getDate()}
                  </span>

                  {/* Items due on this day */}
                  {cellItems.length > 0 && (
                    <ul
                      className="calendar-view__items"
                      aria-label="Items due this day"
                    >
                      {cellItems.map((item) => (
                        <li key={item.id} className="calendar-view__item">
                          {renderItem(item)}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
