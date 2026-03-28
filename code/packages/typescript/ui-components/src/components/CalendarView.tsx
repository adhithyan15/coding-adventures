/**
 * CalendarView — A multi-granularity read-only calendar component.
 *
 * Generic over T (any object with id + dueDate). The caller supplies a
 * `renderItem` function that controls how each item appears inside a day
 * cell or agenda slot. The calendar handles navigation, date math, and
 * layout — not item styling.
 *
 * === Why generic? ===
 *
 * A calendar is a general-purpose layout tool. Today it renders Tasks.
 * Tomorrow it might render calendar events, project milestones, invoice
 * deadlines, or medication reminders. By accepting `T extends CalendarItem`
 * and a render prop, we separate layout from display — the same pattern
 * as a generic list component with a `renderRow` prop.
 *
 * === Four granularities ===
 *
 * "month"  — 6-row × 7-column grid (42 cells). One cell per calendar day.
 *            Stable height regardless of month length. Classic calendar view.
 *
 * "week"   — Single row of 7 day columns. Shows one week at a time.
 *            Week start is driven by `weekStartsOn` (default: 0 = Sunday).
 *
 * "agenda" — Date-grouped list. Timed items appear in HH:MM order inside
 *            hour slots; untimed items appear in an "All Day" section at
 *            the top of each day group.
 *
 * "day"    — Like agenda but shows exactly one day. Useful for dense schedules.
 *
 * === Week-start math ===
 *
 * Many cultures start their week on a day other than Sunday:
 *   • USA, Canada, Israel: Sunday (0)
 *   • Most of Europe:      Monday (1)
 *   • Some Middle East:    Saturday (6)
 *
 * For month grids, this determines the column header order.
 * For week grids, this determines which day is the leftmost column.
 *
 * The formula to find how many days back to go from `dow` to reach
 * the week start day:
 *   offset = ((dow - weekStartsOn) + 7) % 7
 *
 * Example: today is Wednesday (3), weekStartsOn is Monday (1):
 *   offset = ((3 - 1) + 7) % 7 = 2 → go back 2 days → Mon
 *
 * === Timezone safety ===
 *
 * All date strings are computed using `getFullYear()`, `getMonth()`,
 * `getDate()` — local time, never `toISOString()` (which is UTC). This
 * prevents "yesterday" / "tomorrow" bugs for users behind UTC.
 *
 * === Item indexing ===
 *
 * Items are indexed into a `Map<YYYY-MM-DD, T[]>` once per items change
 * using `useMemo`. This gives O(1) per-cell lookups regardless of item count.
 * Items with `dueDate = null` are omitted — they have no calendar position.
 *
 * === Accessibility ===
 *
 * - Outer wrapper: `role="region"` with `aria-label`
 * - Grid: `role="grid"` with `role="row"` / `role="gridcell"` / `role="columnheader"`
 * - Navigation buttons: descriptive `aria-label`
 * - Today's cell: `aria-current="date"`
 */

import React, { useState, useMemo } from "react";

// ── Public types ─────────────────────────────────────────────────────────────

/**
 * CalendarViewGranularity — how finely the calendar slices time.
 *
 *   "agenda" — date-grouped list; timed tasks in HH:MM slots, untimed in All Day
 *   "day"    — single day column (agenda for one day)
 *   "week"   — 7-column week grid
 *   "month"  — classic 6×7 month grid
 */
export type CalendarViewGranularity = "agenda" | "day" | "week" | "month";

/**
 * DayOfWeek — 0 = Sunday, 1 = Monday, … 6 = Saturday.
 * Matches JavaScript's Date.getDay() convention.
 */
export type DayOfWeek = 0 | 1 | 2 | 3 | 4 | 5 | 6;

/**
 * CalendarItem — the minimum shape any item must satisfy to appear on the calendar.
 *
 *   id      — unique key for React reconciliation
 *   dueDate — "YYYY-MM-DD" or null. null = omitted from grid.
 *   dueTime — "HH:MM" 24-hour or null. Used in agenda view for hour-slot placement.
 *             Items without dueTime appear in the "All Day" section.
 */
export interface CalendarItem {
  id: string;
  dueDate: string | null;
  dueTime?: string | null;
}

export interface CalendarViewProps<T extends CalendarItem> {
  /**
   * All items to display. Each item is placed in the cell matching its `dueDate`.
   * Items with `dueDate = null` are silently skipped.
   */
  items: T[];

  /**
   * How finely time should be sliced. Required.
   *
   *   "month"  — classic 6×7 month grid
   *   "week"   — single week (7 columns)
   *   "agenda" — date-grouped list with hour slots
   *   "day"    — agenda for a single day
   */
  granularity: CalendarViewGranularity;

  /**
   * Render function for each item inside a day cell or agenda slot.
   * Keep renders compact — cells have limited vertical space.
   *
   * Example:
   * ```tsx
   * renderItem={(task) => (
   *   <span className="chip chip--high">{task.title}</span>
   * )}
   * ```
   */
  renderItem: (item: T) => React.ReactNode;

  /**
   * Which day of the week starts a week row (visual column order).
   *
   *   0 = Sunday   (USA, Canada, Israel, Japan)
   *   1 = Monday   (most of Europe, UK, Australia)
   *   6 = Saturday (some Middle East contexts)
   *
   * Default: 0 (Sunday)
   */
  weekStartsOn?: DayOfWeek;

  /**
   * Initial display date as YYYY-MM-DD.
   * For "month": displays the month containing this date.
   * For "week" / "day" / "agenda": displays the week/day containing this date.
   * Defaults to today.
   */
  initialDate?: string;

  /** @deprecated Use initialDate. Kept for backward compat. */
  initialYear?: number;

  /** @deprecated Use initialDate. Kept for backward compat. */
  initialMonth?: number;

  /** Additional CSS class applied to the root element. */
  className?: string;

  /** Accessible label for the `role="region"` wrapper. Default: "Calendar". */
  ariaLabel?: string;
}

// ── Date helpers ─────────────────────────────────────────────────────────────

/**
 * toDateString — converts a Date to YYYY-MM-DD in LOCAL time.
 *
 * Never use `date.toISOString().slice(0, 10)` for calendar dates — that
 * converts to UTC, which can shift the date for users in timezones behind UTC.
 */
function toDateString(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/**
 * parseLocalDate — parses a YYYY-MM-DD string as a local-time Date at noon.
 *
 * Using noon (T12:00:00) rather than midnight avoids DST edge cases where
 * "midnight" in some zones can shift to the previous day.
 */
function parseLocalDate(dateStr: string): Date {
  return new Date(`${dateStr}T12:00:00`);
}

/**
 * Build the 42-cell month grid (6 weeks × 7 days) anchored to weekStartsOn.
 *
 * Algorithm:
 *   1. firstOfMonth = new Date(year, month, 1)
 *   2. dow = firstOfMonth.getDay() (0–6)
 *   3. offset = ((dow - weekStartsOn) + 7) % 7 — days to back up
 *   4. gridStart = first - offset days
 *   5. Build 42 consecutive dates from gridStart
 *
 * The 7-column headers must match the offset day:
 *   weekStartsOn=0 → Sun Mon Tue Wed Thu Fri Sat
 *   weekStartsOn=1 → Mon Tue Wed Thu Fri Sat Sun
 */
function buildMonthGrid(
  year: number,
  month: number,
  weekStartsOn: DayOfWeek,
): Date[] {
  const firstOfMonth = new Date(year, month, 1);
  const dow = firstOfMonth.getDay();
  const offset = ((dow - weekStartsOn) + 7) % 7;
  const cells: Date[] = [];

  for (let i = 0; i < 42; i++) {
    // new Date(year, month, 1 - offset + i) handles month rollover natively
    cells.push(new Date(year, month, 1 - offset + i));
  }

  return cells;
}

/**
 * buildWeekDates — returns the 7 Date objects for the week containing `date`.
 *
 * "Containing" means the week starting on `weekStartsOn` that includes `date`.
 */
function buildWeekDates(date: Date, weekStartsOn: DayOfWeek): Date[] {
  const dow = date.getDay();
  const offset = ((dow - weekStartsOn) + 7) % 7;
  const weekStart = new Date(date);
  weekStart.setDate(weekStart.getDate() - offset);

  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(weekStart);
    d.setDate(d.getDate() + i);
    return d;
  });
}

/** Ordered column labels starting from weekStartsOn. */
function buildDayLabels(weekStartsOn: DayOfWeek): string[] {
  const ALL_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  const result: string[] = [];
  for (let i = 0; i < 7; i++) {
    result.push(ALL_LABELS[(weekStartsOn + i) % 7]);
  }
  return result;
}

function formatMonthHeader(year: number, month: number): string {
  return new Intl.DateTimeFormat("en-US", {
    month: "long",
    year: "numeric",
  }).format(new Date(year, month, 1));
}

function formatWeekHeader(weekDates: Date[]): string {
  const first = weekDates[0];
  const last = weekDates[6];
  const sameMonth = first.getMonth() === last.getMonth();
  if (sameMonth) {
    return new Intl.DateTimeFormat("en-US", {
      month: "long",
      year: "numeric",
    }).format(first);
  }
  // Spans two months: "Mar – Apr 2026"
  const startLabel = new Intl.DateTimeFormat("en-US", { month: "short" }).format(first);
  const endLabel = new Intl.DateTimeFormat("en-US", {
    month: "short",
    year: "numeric",
  }).format(last);
  return `${startLabel} – ${endLabel}`;
}

function formatDayHeader(date: Date): string {
  return new Intl.DateTimeFormat("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).format(date);
}

function addMonthsTo(year: number, month: number, delta: number): { year: number; month: number } {
  const d = new Date(year, month + delta, 1);
  return { year: d.getFullYear(), month: d.getMonth() };
}

function addDaysTo(date: Date, delta: number): Date {
  const d = new Date(date);
  d.setDate(d.getDate() + delta);
  return d;
}

// ── Agenda helpers ────────────────────────────────────────────────────────────

/**
 * Group items by their dueDate. Items without dueDate are excluded.
 * Returns Map sorted by date key (ascending).
 */
function groupItemsByDate<T extends CalendarItem>(items: T[]): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const item of items) {
    if (!item.dueDate) continue;
    const bucket = map.get(item.dueDate);
    if (bucket) bucket.push(item);
    else map.set(item.dueDate, [item]);
  }
  // Sort by date key
  return new Map([...map.entries()].sort(([a], [b]) => a.localeCompare(b)));
}

/**
 * Split a day's items into timed (sorted by dueTime) and untimed (all-day).
 */
function splitTimedUntimed<T extends CalendarItem>(
  items: T[],
): { allDay: T[]; timed: T[] } {
  const allDay: T[] = [];
  const timed: T[] = [];
  for (const item of items) {
    if (item.dueTime) timed.push(item);
    else allDay.push(item);
  }
  timed.sort((a, b) => (a.dueTime ?? "").localeCompare(b.dueTime ?? ""));
  return { allDay, timed };
}

// ── Component ─────────────────────────────────────────────────────────────────

export function CalendarView<T extends CalendarItem>({
  items,
  granularity,
  renderItem,
  weekStartsOn = 0,
  initialDate,
  initialYear,
  initialMonth,
  className = "",
  ariaLabel = "Calendar",
}: CalendarViewProps<T>): React.ReactElement {
  // ── Snapshot today once on mount ──────────────────────────────────────────
  const today = new Date();
  const todayStr = toDateString(today);

  // ── Determine initial display date ────────────────────────────────────────
  // Priority: initialDate > initialYear/initialMonth (legacy) > today
  const initDate = useMemo<Date>(() => {
    if (initialDate) return parseLocalDate(initialDate);
    if (initialYear !== undefined && initialMonth !== undefined) {
      return new Date(initialYear, initialMonth, 1);
    }
    return new Date(today);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── Navigation state ──────────────────────────────────────────────────────
  // For month: track year + month. For week/day/agenda: track a reference date.
  const [year, setYear] = useState<number>(initDate.getFullYear());
  const [month, setMonth] = useState<number>(initDate.getMonth());
  const [refDate, setRefDate] = useState<Date>(initDate);

  // ── Item index for month/week grids: Map<YYYY-MM-DD, T[]> ─────────────────
  const itemsByDate = useMemo<Map<string, T[]>>(() => {
    const map = new Map<string, T[]>();
    for (const item of items) {
      if (!item.dueDate) continue;
      const bucket = map.get(item.dueDate);
      if (bucket) bucket.push(item);
      else map.set(item.dueDate, [item]);
    }
    return map;
  }, [items]);

  // ── Dispatch to the correct layout ───────────────────────────────────────
  if (granularity === "month") {
    return (
      <MonthView
        items={items}
        itemsByDate={itemsByDate}
        year={year}
        month={month}
        todayStr={todayStr}
        weekStartsOn={weekStartsOn}
        renderItem={renderItem}
        className={className}
        ariaLabel={ariaLabel}
        onPrev={() => { const n = addMonthsTo(year, month, -1); setYear(n.year); setMonth(n.month); }}
        onNext={() => { const n = addMonthsTo(year, month, 1); setYear(n.year); setMonth(n.month); }}
        onToday={() => { setYear(today.getFullYear()); setMonth(today.getMonth()); }}
      />
    );
  }

  if (granularity === "week") {
    const weekDates = buildWeekDates(refDate, weekStartsOn);
    return (
      <WeekView
        items={items}
        itemsByDate={itemsByDate}
        weekDates={weekDates}
        todayStr={todayStr}
        weekStartsOn={weekStartsOn}
        renderItem={renderItem}
        className={className}
        ariaLabel={ariaLabel}
        onPrev={() => setRefDate((d) => addDaysTo(d, -7))}
        onNext={() => setRefDate((d) => addDaysTo(d, 7))}
        onToday={() => setRefDate(new Date(today))}
      />
    );
  }

  // agenda and day — list-based layouts
  const agendaDays = granularity === "day" ? 1 : 7;
  return (
    <AgendaView
      items={items}
      refDate={refDate}
      todayStr={todayStr}
      singleDay={granularity === "day"}
      agendaDays={agendaDays}
      renderItem={renderItem}
      className={className}
      ariaLabel={ariaLabel}
      onPrev={() => setRefDate((d) => addDaysTo(d, granularity === "day" ? -1 : -7))}
      onNext={() => setRefDate((d) => addDaysTo(d, granularity === "day" ? 1 : 7))}
      onToday={() => setRefDate(new Date(today))}
    />
  );
}

// ── MonthView ─────────────────────────────────────────────────────────────────

interface MonthViewProps<T extends CalendarItem> {
  items: T[];
  itemsByDate: Map<string, T[]>;
  year: number;
  month: number;
  todayStr: string;
  weekStartsOn: DayOfWeek;
  renderItem: (item: T) => React.ReactNode;
  className: string;
  ariaLabel: string;
  onPrev: () => void;
  onNext: () => void;
  onToday: () => void;
}

function MonthView<T extends CalendarItem>({
  itemsByDate,
  year,
  month,
  todayStr,
  weekStartsOn,
  renderItem,
  className,
  ariaLabel,
  onPrev,
  onNext,
  onToday,
}: MonthViewProps<T>): React.ReactElement {
  const grid = useMemo(
    () => buildMonthGrid(year, month, weekStartsOn),
    [year, month, weekStartsOn],
  );
  const dayLabels = buildDayLabels(weekStartsOn);

  return (
    <div
      className={["calendar-view", "calendar-view--month", className].filter(Boolean).join(" ")}
      role="region"
      aria-label={ariaLabel}
    >
      <div className="calendar-view__header">
        <button type="button" className="calendar-view__nav-btn" onClick={onPrev} aria-label="Previous month">‹</button>
        <h2 className="calendar-view__month-label">{formatMonthHeader(year, month)}</h2>
        <button type="button" className="calendar-view__nav-btn" onClick={onNext} aria-label="Next month">›</button>
        <button type="button" className="calendar-view__today-btn" onClick={onToday} aria-label="Go to today">Today</button>
      </div>

      <div className="calendar-view__grid" role="grid" aria-label={ariaLabel}>
        {/* Column headers */}
        <div className="calendar-view__row calendar-view__row--header" role="row">
          {dayLabels.map((label) => (
            <div key={label} className="calendar-view__day-header" role="columnheader" aria-label={label}>
              {label}
            </div>
          ))}
        </div>

        {/* 6 week rows */}
        {Array.from({ length: 6 }, (_, weekIdx) => (
          <div key={weekIdx} className="calendar-view__row" role="row">
            {grid.slice(weekIdx * 7, weekIdx * 7 + 7).map((cellDate) => {
              const dateStr = toDateString(cellDate);
              const isCurrentMonth = cellDate.getMonth() === month;
              const isToday = dateStr === todayStr;
              const cellItems = itemsByDate.get(dateStr) ?? [];
              const itemCount = cellItems.length;

              const cellClass = [
                "calendar-view__cell",
                isCurrentMonth ? "calendar-view__cell--current-month" : "calendar-view__cell--other-month",
                isToday ? "calendar-view__cell--today" : "",
                itemCount > 0 ? "calendar-view__cell--has-items" : "",
              ].filter(Boolean).join(" ");

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
                  <span className="calendar-view__day-number">{cellDate.getDate()}</span>
                  {cellItems.length > 0 && (
                    <ul className="calendar-view__items" aria-label="Items due this day">
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

// ── WeekView ──────────────────────────────────────────────────────────────────

interface WeekViewProps<T extends CalendarItem> {
  items: T[];
  itemsByDate: Map<string, T[]>;
  weekDates: Date[];
  todayStr: string;
  weekStartsOn: DayOfWeek;
  renderItem: (item: T) => React.ReactNode;
  className: string;
  ariaLabel: string;
  onPrev: () => void;
  onNext: () => void;
  onToday: () => void;
}

function WeekView<T extends CalendarItem>({
  itemsByDate,
  weekDates,
  todayStr,
  weekStartsOn,
  renderItem,
  className,
  ariaLabel,
  onPrev,
  onNext,
  onToday,
}: WeekViewProps<T>): React.ReactElement {
  const dayLabels = buildDayLabels(weekStartsOn);

  return (
    <div
      className={["calendar-view", "calendar-view--week", className].filter(Boolean).join(" ")}
      role="region"
      aria-label={ariaLabel}
    >
      <div className="calendar-view__header">
        <button type="button" className="calendar-view__nav-btn" onClick={onPrev} aria-label="Previous week">‹</button>
        <h2 className="calendar-view__month-label">{formatWeekHeader(weekDates)}</h2>
        <button type="button" className="calendar-view__nav-btn" onClick={onNext} aria-label="Next week">›</button>
        <button type="button" className="calendar-view__today-btn" onClick={onToday} aria-label="Go to today">Today</button>
      </div>

      <div className="calendar-view__grid calendar-view__grid--week" role="grid" aria-label={ariaLabel}>
        {/* Column headers */}
        <div className="calendar-view__row calendar-view__row--header" role="row">
          {dayLabels.map((label) => (
            <div key={label} className="calendar-view__day-header" role="columnheader" aria-label={label}>
              {label}
            </div>
          ))}
        </div>

        {/* Single week row */}
        <div className="calendar-view__row" role="row">
          {weekDates.map((cellDate) => {
            const dateStr = toDateString(cellDate);
            const isToday = dateStr === todayStr;
            const cellItems = itemsByDate.get(dateStr) ?? [];
            const itemCount = cellItems.length;

            const cellClass = [
              "calendar-view__cell",
              "calendar-view__cell--current-month",
              isToday ? "calendar-view__cell--today" : "",
              itemCount > 0 ? "calendar-view__cell--has-items" : "",
            ].filter(Boolean).join(" ");

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
                <span className="calendar-view__day-number">{cellDate.getDate()}</span>
                {cellItems.length > 0 && (
                  <ul className="calendar-view__items" aria-label="Items due this day">
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
      </div>
    </div>
  );
}

// ── AgendaView ────────────────────────────────────────────────────────────────

interface AgendaViewProps<T extends CalendarItem> {
  items: T[];
  refDate: Date;
  todayStr: string;
  singleDay: boolean;
  agendaDays: number;
  renderItem: (item: T) => React.ReactNode;
  className: string;
  ariaLabel: string;
  onPrev: () => void;
  onNext: () => void;
  onToday: () => void;
}

function AgendaView<T extends CalendarItem>({
  items,
  refDate,
  todayStr,
  singleDay,
  renderItem,
  className,
  ariaLabel,
  onPrev,
  onNext,
  onToday,
}: AgendaViewProps<T>): React.ReactElement {
  // Filter items to those with a dueDate matching today (for day view)
  // or group all items by date for agenda view
  const groupedItems = useMemo(() => groupItemsByDate(items), [items]);

  const refDateStr = toDateString(refDate);
  const header = singleDay ? formatDayHeader(refDate) : `Week of ${refDateStr}`;
  const prevLabel = singleDay ? "Previous day" : "Previous week";
  const nextLabel = singleDay ? "Next day" : "Next week";

  return (
    <div
      className={["calendar-view", "calendar-view--agenda", className].filter(Boolean).join(" ")}
      role="region"
      aria-label={ariaLabel}
    >
      <div className="calendar-view__header">
        <button type="button" className="calendar-view__nav-btn" onClick={onPrev} aria-label={prevLabel}>‹</button>
        <h2 className="calendar-view__month-label">{header}</h2>
        <button type="button" className="calendar-view__nav-btn" onClick={onNext} aria-label={nextLabel}>›</button>
        <button type="button" className="calendar-view__today-btn" onClick={onToday} aria-label="Go to today">Today</button>
      </div>

      <div className="calendar-view__agenda">
        {groupedItems.size === 0 ? (
          <div className="calendar-view__agenda-empty">No items to display</div>
        ) : (
          Array.from(groupedItems.entries()).map(([dateStr, dayItems]) => {
            const isToday = dateStr === todayStr;
            const { allDay, timed } = splitTimedUntimed(dayItems);

            return (
              <div
                key={dateStr}
                className={[
                  "calendar-view__agenda-day",
                  isToday ? "calendar-view__agenda-day--today" : "",
                ].filter(Boolean).join(" ")}
                aria-current={isToday ? "date" : undefined}
              >
                <div className="calendar-view__agenda-date-label">
                  {new Intl.DateTimeFormat("en-US", {
                    weekday: "short",
                    month: "short",
                    day: "numeric",
                  }).format(parseLocalDate(dateStr))}
                </div>

                {/* All-day tasks (no dueTime) */}
                {allDay.length > 0 && (
                  <div className="calendar-view__agenda-section calendar-view__agenda-section--allday">
                    <span className="calendar-view__agenda-time-label">All Day</span>
                    <ul className="calendar-view__items">
                      {allDay.map((item) => (
                        <li key={item.id} className="calendar-view__item">
                          {renderItem(item)}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {/* Timed tasks — sorted by dueTime */}
                {timed.map((item) => (
                  <div key={item.id} className="calendar-view__agenda-section calendar-view__agenda-section--timed">
                    <span className="calendar-view__agenda-time-label">{item.dueTime}</span>
                    <div className="calendar-view__item">{renderItem(item)}</div>
                  </div>
                ))}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
