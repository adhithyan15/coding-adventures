/**
 * calendar-settings.ts — CalendarSettings domain model.
 *
 * A "calendar" in this app is not a list of events — it is a description of
 * HOW A PERSON WORKS. It answers questions like:
 *
 *   • Which days of the week do they work?
 *   • What hours do they work each day?
 *   • Are there gaps in their day? (split shifts, sabbath boundaries)
 *   • Which specific dates are holidays or have special schedules?
 *   • In which timezone should "today" be calculated?
 *
 * This model is intentionally richer than a simple `workingDays: number[]`.
 * A freelancer might work only Monday + Wednesday. A Jewish observant developer
 * might end their Friday at 14:00. A person in the UAE might work Sunday–Thursday.
 * All of these are representable here with no schema changes.
 *
 * === Key design decisions ===
 *
 * 1. Per-day hours are arrays of TimeInterval, not a single start/end.
 *    This supports split shifts (morning + afternoon), partial days, and
 *    future features like "focus blocks."
 *
 * 2. weeklySchedule is a Partial<Record<DayOfWeek, DaySchedule>>.
 *    A day ABSENT from the map is non-working.
 *    A day PRESENT with hours: [] is explicitly defined as 0-working-hours.
 *    This distinction matters: you can name a day "available but blocked today"
 *    without it being absent from the schedule altogether.
 *
 * 3. dateOverrides take precedence over weeklySchedule.
 *    A holiday on Tuesday overrides that Tuesday completely.
 *    A "custom" override lets you give a specific date a different schedule
 *    (e.g., a half-day before a long weekend).
 *
 * 4. timezone is an IANA zone string ("America/New_York", "Asia/Jerusalem").
 *    This is used to compute "what date is 'today'" correctly across the world.
 *    Two people opening the app at the same UTC instant may see different dates.
 *
 * === Example: Israeli Hi-Tech Calendar ===
 *
 *   weekStartsOn: 0 (Sunday — week starts on Sunday in Israel)
 *   weeklySchedule:
 *     Sun: 09:00–18:00, Mon: 09:00–18:00, Tue: 09:00–18:00,
 *     Wed: 09:00–18:00, Thu: 09:00–18:00,
 *     Fri: 09:00–14:00  (half day before Shabbat)
 *     Sat: (absent — Shabbat, not a working day)
 *   dateOverrides:
 *     "2026-09-21": holiday "Rosh Hashana"
 *     "2026-09-30": holiday "Yom Kippur"
 *
 * === V1 note ===
 *
 * We ship one default calendar ("Gregorian, Mon–Fri, 9–5"). Users will be
 * able to create and customize calendars in a future release. The data
 * structure is designed to support that without breaking changes.
 */

// ── Primitives ─────────────────────────────────────────────────────────────

/**
 * DayOfWeek — 0 = Sunday, 1 = Monday, … 6 = Saturday.
 *
 * Imported from @coding-adventures/ui-components where the CalendarView
 * component defines it as the canonical source. Re-exported here for
 * callers who only import from calendar-settings.ts.
 *
 * This follows the JavaScript convention (Date.getDay() returns 0 for Sunday).
 * Notable cultural differences:
 *   USA / Israel: week starts Sunday (0)
 *   Europe:       week starts Monday (1)
 *   Some Middle East: week starts Saturday (6)
 */
export type { DayOfWeek } from "@coding-adventures/ui-components";

/**
 * TimeInterval — a contiguous block of time within a single day.
 *
 * start and end are HH:MM in 24-hour format. end is exclusive (like a range).
 *
 * Examples:
 *   Standard day:    { start: "09:00", end: "17:00" }
 *   Morning shift:   { start: "06:00", end: "14:00" }
 *   Pre-Shabbat:     { start: "09:00", end: "14:00" }
 *   One-hour focus:  { start: "10:00", end: "11:00" }
 */
export interface TimeInterval {
  /** Start time, "HH:MM" 24-hour. Inclusive. */
  start: string;
  /** End time, "HH:MM" 24-hour. Exclusive (09:00–17:00 = 8 hours). */
  end: string;
}

/**
 * DaySchedule — working hours for a single day-of-week.
 *
 * hours is an ARRAY because real schedules have gaps:
 *   Standard:    [{ start: "09:00", end: "17:00" }]
 *   Split shift: [{ start: "08:00", end: "12:00" }, { start: "14:00", end: "18:00" }]
 *   Half day:    [{ start: "09:00", end: "13:00" }]
 *   Zero hours:  []  — the day is "defined but not working"
 */
export interface DaySchedule {
  /** All working blocks within this day. Empty = no working hours. */
  hours: TimeInterval[];
}

/**
 * DateOverride — a rule that overrides the weekly schedule for ONE specific date.
 *
 * Precedence: dateOverrides > weeklySchedule.
 * If 2026-12-25 (Christmas, a Thursday) has a "holiday" override, that
 * Thursday is blocked regardless of what weeklySchedule says for Thursday.
 *
 * type "holiday" — entire day is blocked. No work happens.
 *   schedule should be omitted or set to { hours: [] }.
 *
 * type "custom" — day has a special schedule.
 *   Example: half-day on Dec 24th even though it's a Thursday.
 *   Set schedule to the actual hours for that specific date.
 */
export interface DateOverride {
  /** Specific calendar date in YYYY-MM-DD format. */
  date: string;

  /** "holiday" = full day blocked. "custom" = special hours for this date. */
  type: "holiday" | "custom";

  /** Human-readable name. Displayed in the calendar UI. */
  name?: string;

  /**
   * The schedule for this specific date.
   * For "holiday": omit or set hours: [] — the entire day is unavailable.
   * For "custom": set the actual working hours for this date.
   */
  schedule?: DaySchedule;
}

// ── CalendarSettings ───────────────────────────────────────────────────────

/**
 * CalendarSettings — a full definition of how one person works.
 *
 * This is a PERSISTED entity stored in IndexedDB. Users start with the
 * built-in "Gregorian" calendar and will be able to create custom ones.
 *
 * Fields:
 *
 *   id            — stable unique key, e.g. "gregorian"
 *   name          — display name: "Gregorian (Mon–Fri, 9–5)"
 *   weekStartsOn  — which day begins a week ROW in grid views (purely visual)
 *   weeklySchedule — DayOfWeek → working hours mapping.
 *                    Keys present = working days; absent keys = non-working.
 *   dateOverrides  — list of specific dates that deviate from the weekly pattern.
 *                    Sorted by date for display; order doesn't affect logic.
 *   timezone       — IANA zone string. "Today" is computed in this timezone.
 *   isBuiltIn      — true = cannot be deleted by the user
 *   createdAt/updatedAt — Unix timestamps
 */
export interface CalendarSettings {
  id: string;
  name: string;

  /**
   * Which day-of-week starts a week row in calendar grid views.
   *
   * This is a DISPLAY preference, not a working-days rule.
   *   0 = Sunday start (USA, Canada, Israel, Japan)
   *   1 = Monday start (most of Europe, UK)
   *   6 = Saturday start (some Middle East contexts)
   *
   * It does NOT affect which days are working — that's weeklySchedule.
   */
  weekStartsOn: DayOfWeek;

  /**
   * Weekly recurring schedule. Maps DayOfWeek to working hours.
   *
   * A key ABSENT from the map means that day-of-week is not a working day.
   * A key PRESENT with hours: [] means the day is defined but has zero hours.
   *
   * Example — standard Mon–Fri:
   *   { 1: {hours:[{start:"09:00",end:"17:00"}]},
   *     2: {hours:[{start:"09:00",end:"17:00"}]},
   *     3: {hours:[{start:"09:00",end:"17:00"}]},
   *     4: {hours:[{start:"09:00",end:"17:00"}]},
   *     5: {hours:[{start:"09:00",end:"17:00"}]} }
   *
   * Example — Israeli Sun–Thu + half-day Fri:
   *   { 0: ..., 1: ..., 2: ..., 3: ..., 4: ...,
   *     5: {hours:[{start:"09:00",end:"14:00"}]} }
   *   // 6 (Sat) absent = Shabbat
   */
  weeklySchedule: Partial<Record<DayOfWeek, DaySchedule>>;

  /**
   * Date-specific overrides. Each entry replaces the weeklySchedule entry
   * for that particular date.
   *
   * Example: [ { date:"2026-12-25", type:"holiday", name:"Christmas" } ]
   *
   * Tip: sort by date when displaying in UI (for human readability),
   * but sorting is not required for correctness.
   */
  dateOverrides: DateOverride[];

  /**
   * IANA timezone identifier.
   *
   * Used when computing what YYYY-MM-DD "today" is for this user.
   * Two people opening the app at the same UTC moment in New York vs Tokyo
   * may see different dates — this field encodes that.
   *
   * Examples: "America/New_York", "Europe/London", "Asia/Jerusalem", "Asia/Tokyo"
   *
   * For V1 we default to the browser's local timezone via
   * Intl.DateTimeFormat().resolvedOptions().timeZone.
   */
  timezone: string;

  /** Built-in calendars cannot be deleted. */
  isBuiltIn: boolean;

  createdAt: number;
  updatedAt: number;
}

// ── Helper functions ───────────────────────────────────────────────────────

/**
 * isWorkingDay — returns true if a specific YYYY-MM-DD date is a working day
 * according to this calendar.
 *
 * Resolution order:
 *   1. If there is a dateOverride for this date AND it's a "holiday",
 *      or it's "custom" with hours: [], then NOT a working day.
 *   2. If there is a "custom" override with actual hours, then IS working.
 *   3. Fall through to weeklySchedule: check if the day-of-week has
 *      any non-zero hours.
 *
 * Note: we use new Date(date + "T12:00:00") rather than new Date(date) to
 * avoid the midnight UTC ambiguity — "2026-03-01" parsed as UTC midnight
 * can land on the wrong day in timezones behind UTC.
 */
export function isWorkingDay(date: string, calendar: CalendarSettings): boolean {
  // ── 1. Check date overrides ──────────────────────────────────────────────
  const override = calendar.dateOverrides.find((o) => o.date === date);
  if (override) {
    if (override.type === "holiday") return false;
    // "custom" type: working iff the override has at least one hour
    if (override.schedule) {
      return override.schedule.hours.length > 0;
    }
    // custom override with no schedule = treat as holiday
    return false;
  }

  // ── 2. Fall through to weekly schedule ──────────────────────────────────
  // Parse the date as local noon to avoid UTC-shift issues.
  const d = new Date(`${date}T12:00:00`);
  const dow = d.getDay() as DayOfWeek;
  const daySched = calendar.weeklySchedule[dow];

  // Day absent from weeklySchedule = not a working day
  if (!daySched) return false;

  // Day present but with zero hours = explicitly non-working
  return daySched.hours.length > 0;
}

/**
 * workingHoursOnDate — total working hours on a specific YYYY-MM-DD date.
 *
 * Returns 0 if the date is a non-working day or a holiday.
 * Returns the sum of all interval durations otherwise.
 *
 * Example: [{ start:"09:00", end:"12:00" }, { start:"13:00", end:"17:00" }]
 *   → 3 + 4 = 7 hours
 */
export function workingHoursOnDate(
  date: string,
  calendar: CalendarSettings,
): number {
  // Resolve the effective schedule for this date
  const schedule = effectiveSchedule(date, calendar);
  if (!schedule) return 0;

  return schedule.hours.reduce((total, interval) => {
    return total + intervalHours(interval);
  }, 0);
}

/**
 * effectiveSchedule — resolves the DaySchedule for a specific date,
 * taking dateOverrides into account.
 *
 * Returns null if the date is non-working.
 */
export function effectiveSchedule(
  date: string,
  calendar: CalendarSettings,
): DaySchedule | null {
  // Check overrides first
  const override = calendar.dateOverrides.find((o) => o.date === date);
  if (override) {
    if (override.type === "holiday") return null;
    if (override.schedule) return override.schedule;
    return null; // custom with no schedule = blocked
  }

  // Fall through to weekly schedule
  const d = new Date(`${date}T12:00:00`);
  const dow = d.getDay() as DayOfWeek;
  return calendar.weeklySchedule[dow] ?? null;
}

/**
 * intervalHours — computes the duration of a TimeInterval in hours.
 *
 * "09:00"–"17:00" → 8.0
 * "09:00"–"09:30" → 0.5
 * "09:00"–"14:00" → 5.0
 */
export function intervalHours(interval: TimeInterval): number {
  const [startH, startM] = interval.start.split(":").map(Number);
  const [endH, endM] = interval.end.split(":").map(Number);
  return (endH * 60 + endM - (startH * 60 + startM)) / 60;
}

// ── Default Calendar ───────────────────────────────────────────────────────

/**
 * GREGORIAN_CALENDAR — V1 default.
 *
 * Standard Monday–Friday, 9am–5pm. Timezone defaults to the user's browser
 * timezone via Intl. No holidays pre-loaded (user adds their own in future).
 *
 * This is a FACTORY FUNCTION (not a plain const) because the timezone
 * must be read at runtime — calling Intl at module load time is fine, but
 * using a function makes it easy to test with different timezones.
 */
export function createGregorianCalendar(): CalendarSettings {
  const now = Date.now();
  return {
    id: "gregorian",
    name: "Gregorian (Mon–Fri, 9–5)",
    weekStartsOn: 0, // Sunday — the most globally recognized week start
    weeklySchedule: {
      1: { hours: [{ start: "09:00", end: "17:00" }] }, // Monday
      2: { hours: [{ start: "09:00", end: "17:00" }] }, // Tuesday
      3: { hours: [{ start: "09:00", end: "17:00" }] }, // Wednesday
      4: { hours: [{ start: "09:00", end: "17:00" }] }, // Thursday
      5: { hours: [{ start: "09:00", end: "17:00" }] }, // Friday
      // Saturday (6) and Sunday (0) absent = non-working
    },
    dateOverrides: [],
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    isBuiltIn: true,
    createdAt: now,
    updatedAt: now,
  };
}
