/**
 * calendar-settings.test.ts — Unit tests for CalendarSettings helpers.
 *
 * We test:
 *   isWorkingDay       — returns true/false based on weeklySchedule + dateOverrides
 *   workingHoursOnDate — total working hours on a specific date
 *   effectiveSchedule  — resolves the DaySchedule for a date
 *   intervalHours      — duration of a TimeInterval in hours
 *   createGregorianCalendar — builds the V1 default calendar
 *
 * === Notation ===
 *
 * Days of week: 0=Sun, 1=Mon, 2=Tue, 3=Wed, 4=Thu, 5=Fri, 6=Sat
 *
 * We pick specific dates to test specific days-of-week:
 *   2026-03-16 = Monday
 *   2026-03-21 = Saturday
 *   2026-03-22 = Sunday
 */

import { describe, it, expect } from "vitest";
import {
  isWorkingDay,
  workingHoursOnDate,
  effectiveSchedule,
  intervalHours,
  createGregorianCalendar,
} from "../calendar-settings.js";
import type { CalendarSettings, DaySchedule } from "../calendar-settings.js";

// ── Test calendar fixtures ────────────────────────────────────────────────────

/**
 * Standard Mon–Fri calendar (like the Gregorian default).
 * No holidays or overrides.
 */
const MON_FRI_CALENDAR: CalendarSettings = {
  id: "test-mf",
  name: "Mon–Fri test",
  weekStartsOn: 0,
  weeklySchedule: {
    1: { hours: [{ start: "09:00", end: "17:00" }] }, // Mon: 8h
    2: { hours: [{ start: "09:00", end: "17:00" }] }, // Tue: 8h
    3: { hours: [{ start: "09:00", end: "17:00" }] }, // Wed: 8h
    4: { hours: [{ start: "09:00", end: "17:00" }] }, // Thu: 8h
    5: { hours: [{ start: "09:00", end: "17:00" }] }, // Fri: 8h
    // Sat (6) and Sun (0) absent = non-working
  },
  dateOverrides: [],
  timezone: "UTC",
  isBuiltIn: false,
  createdAt: 0,
  updatedAt: 0,
};

/**
 * Israeli-style calendar: Sun–Thu full day, Fri half day, Sat off.
 */
const ISRAELI_CALENDAR: CalendarSettings = {
  id: "test-il",
  name: "Israeli test",
  weekStartsOn: 0,
  weeklySchedule: {
    0: { hours: [{ start: "09:00", end: "18:00" }] }, // Sun: 9h
    1: { hours: [{ start: "09:00", end: "18:00" }] }, // Mon: 9h
    2: { hours: [{ start: "09:00", end: "18:00" }] }, // Tue: 9h
    3: { hours: [{ start: "09:00", end: "18:00" }] }, // Wed: 9h
    4: { hours: [{ start: "09:00", end: "18:00" }] }, // Thu: 9h
    5: { hours: [{ start: "09:00", end: "14:00" }] }, // Fri: 5h (half day)
    // Sat (6) absent = Shabbat
  },
  dateOverrides: [],
  timezone: "Asia/Jerusalem",
  isBuiltIn: false,
  createdAt: 0,
  updatedAt: 0,
};

/** Three-day work week (Mon, Wed, Fri only). */
const THREE_DAY_CALENDAR: CalendarSettings = {
  id: "test-3d",
  name: "Mon/Wed/Fri only",
  weekStartsOn: 1,
  weeklySchedule: {
    1: { hours: [{ start: "10:00", end: "16:00" }] }, // Mon: 6h
    3: { hours: [{ start: "10:00", end: "16:00" }] }, // Wed: 6h
    5: { hours: [{ start: "10:00", end: "16:00" }] }, // Fri: 6h
  },
  dateOverrides: [],
  timezone: "UTC",
  isBuiltIn: false,
  createdAt: 0,
  updatedAt: 0,
};

/** Calendar with a split-shift Monday. */
const SPLIT_SHIFT_CALENDAR: CalendarSettings = {
  id: "test-split",
  name: "Split shift",
  weekStartsOn: 0,
  weeklySchedule: {
    1: {
      hours: [
        { start: "08:00", end: "12:00" }, // 4h morning
        { start: "14:00", end: "18:00" }, // 4h afternoon (lunch gap)
      ],
    },
  },
  dateOverrides: [],
  timezone: "UTC",
  isBuiltIn: false,
  createdAt: 0,
  updatedAt: 0,
};

// ── intervalHours ─────────────────────────────────────────────────────────────

describe("intervalHours", () => {
  it("calculates full 8-hour day", () => {
    expect(intervalHours({ start: "09:00", end: "17:00" })).toBe(8);
  });

  it("calculates half-day (5 hours)", () => {
    expect(intervalHours({ start: "09:00", end: "14:00" })).toBe(5);
  });

  it("calculates 30-minute interval", () => {
    expect(intervalHours({ start: "09:00", end: "09:30" })).toBe(0.5);
  });

  it("calculates 1-hour interval", () => {
    expect(intervalHours({ start: "10:00", end: "11:00" })).toBe(1);
  });

  it("calculates afternoon block", () => {
    expect(intervalHours({ start: "14:00", end: "18:00" })).toBe(4);
  });

  it("calculates 9-hour workday", () => {
    expect(intervalHours({ start: "09:00", end: "18:00" })).toBe(9);
  });

  it("calculates full 24-hour day (00:00–24:00)", () => {
    // "24:00" is the end-of-day sentinel used by the 24/7 Gregorian calendar.
    // intervalHours: (24*60 + 0 − (0*60 + 0)) / 60 = 1440 / 60 = 24
    expect(intervalHours({ start: "00:00", end: "24:00" })).toBe(24);
  });
});

// ── effectiveSchedule ─────────────────────────────────────────────────────────

describe("effectiveSchedule", () => {
  it("returns the weeklySchedule for a standard workday", () => {
    // 2026-03-16 is a Monday
    const sched = effectiveSchedule("2026-03-16", MON_FRI_CALENDAR);
    expect(sched).not.toBeNull();
    expect(sched!.hours).toHaveLength(1);
    expect(sched!.hours[0]!.start).toBe("09:00");
  });

  it("returns null for a non-working day (Saturday)", () => {
    // 2026-03-21 is a Saturday
    expect(effectiveSchedule("2026-03-21", MON_FRI_CALENDAR)).toBeNull();
  });

  it("returns null for a holiday override", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [{ date: "2026-03-16", type: "holiday", name: "Public Holiday" }],
    };
    // Monday 2026-03-16 is overridden as a holiday
    expect(effectiveSchedule("2026-03-16", calendar)).toBeNull();
  });

  it("returns custom schedule for a custom override", () => {
    const customSchedule: DaySchedule = { hours: [{ start: "09:00", end: "13:00" }] };
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [
        { date: "2026-03-16", type: "custom", name: "Half day", schedule: customSchedule },
      ],
    };
    const sched = effectiveSchedule("2026-03-16", calendar);
    expect(sched).not.toBeNull();
    expect(sched!.hours[0]!.end).toBe("13:00");
  });

  it("returns null for custom override with no schedule", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [{ date: "2026-03-16", type: "custom" }],
    };
    expect(effectiveSchedule("2026-03-16", calendar)).toBeNull();
  });
});

// ── isWorkingDay ──────────────────────────────────────────────────────────────

describe("isWorkingDay", () => {
  // ── Mon–Fri calendar ─────────────────────────────────────────────────────

  it("Monday is a working day (Mon–Fri calendar)", () => {
    expect(isWorkingDay("2026-03-16", MON_FRI_CALENDAR)).toBe(true); // Monday
  });

  it("Wednesday is a working day (Mon–Fri calendar)", () => {
    expect(isWorkingDay("2026-03-18", MON_FRI_CALENDAR)).toBe(true); // Wednesday
  });

  it("Friday is a working day (Mon–Fri calendar)", () => {
    expect(isWorkingDay("2026-03-20", MON_FRI_CALENDAR)).toBe(true); // Friday
  });

  it("Saturday is NOT a working day (Mon–Fri calendar)", () => {
    expect(isWorkingDay("2026-03-21", MON_FRI_CALENDAR)).toBe(false); // Saturday
  });

  it("Sunday is NOT a working day (Mon–Fri calendar)", () => {
    expect(isWorkingDay("2026-03-22", MON_FRI_CALENDAR)).toBe(false); // Sunday
  });

  // ── Israeli calendar ─────────────────────────────────────────────────────

  it("Sunday IS a working day (Israeli calendar)", () => {
    expect(isWorkingDay("2026-03-15", ISRAELI_CALENDAR)).toBe(true); // Sunday
  });

  it("Friday is a (half-day) working day (Israeli calendar)", () => {
    expect(isWorkingDay("2026-03-20", ISRAELI_CALENDAR)).toBe(true); // Friday
  });

  it("Saturday is NOT a working day (Israeli calendar — Shabbat)", () => {
    expect(isWorkingDay("2026-03-21", ISRAELI_CALENDAR)).toBe(false); // Saturday
  });

  // ── Three-day calendar ───────────────────────────────────────────────────

  it("Monday is a working day (Mon/Wed/Fri calendar)", () => {
    expect(isWorkingDay("2026-03-16", THREE_DAY_CALENDAR)).toBe(true); // Monday
  });

  it("Tuesday is NOT a working day (Mon/Wed/Fri calendar)", () => {
    expect(isWorkingDay("2026-03-17", THREE_DAY_CALENDAR)).toBe(false); // Tuesday
  });

  it("Wednesday is a working day (Mon/Wed/Fri calendar)", () => {
    expect(isWorkingDay("2026-03-18", THREE_DAY_CALENDAR)).toBe(true); // Wednesday
  });

  it("Thursday is NOT a working day (Mon/Wed/Fri calendar)", () => {
    expect(isWorkingDay("2026-03-19", THREE_DAY_CALENDAR)).toBe(false); // Thursday
  });

  // ── Date overrides ───────────────────────────────────────────────────────

  it("holiday override blocks an otherwise working day", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [{ date: "2026-03-16", type: "holiday", name: "Purim" }],
    };
    // Monday 2026-03-16 is normally working, but overridden as holiday
    expect(isWorkingDay("2026-03-16", calendar)).toBe(false);
  });

  it("custom override with hours makes a normally-off day working", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [
        {
          date: "2026-03-21", // Saturday — normally off
          type: "custom",
          name: "Special sprint day",
          schedule: { hours: [{ start: "10:00", end: "14:00" }] },
        },
      ],
    };
    expect(isWorkingDay("2026-03-21", calendar)).toBe(true);
  });

  it("custom override with empty hours keeps day non-working", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [
        { date: "2026-03-16", type: "custom", schedule: { hours: [] } },
      ],
    };
    expect(isWorkingDay("2026-03-16", calendar)).toBe(false);
  });

  // ── Explicit zero-hours day ───────────────────────────────────────────────

  it("day with empty hours in weeklySchedule is not a working day", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      weeklySchedule: {
        ...MON_FRI_CALENDAR.weeklySchedule,
        3: { hours: [] }, // Wednesday explicitly defined but zero hours
      },
    };
    expect(isWorkingDay("2026-03-18", calendar)).toBe(false); // Wednesday
  });
});

// ── workingHoursOnDate ────────────────────────────────────────────────────────

describe("workingHoursOnDate", () => {
  it("returns 8 for a standard 9–5 workday", () => {
    expect(workingHoursOnDate("2026-03-16", MON_FRI_CALENDAR)).toBe(8); // Monday
  });

  it("returns 9 for a 9–18 workday (Israeli calendar)", () => {
    expect(workingHoursOnDate("2026-03-16", ISRAELI_CALENDAR)).toBe(9); // Monday
  });

  it("returns 5 for the Israeli half-day Friday", () => {
    expect(workingHoursOnDate("2026-03-20", ISRAELI_CALENDAR)).toBe(5); // Friday
  });

  it("returns 0 for a non-working day (Saturday)", () => {
    expect(workingHoursOnDate("2026-03-21", MON_FRI_CALENDAR)).toBe(0);
  });

  it("returns 0 for a holiday override", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [{ date: "2026-03-16", type: "holiday", name: "Holiday" }],
    };
    expect(workingHoursOnDate("2026-03-16", calendar)).toBe(0);
  });

  it("returns correct hours for a split-shift day (8h total with lunch gap)", () => {
    // Mon: 08:00–12:00 (4h) + 14:00–18:00 (4h) = 8h total
    expect(workingHoursOnDate("2026-03-16", SPLIT_SHIFT_CALENDAR)).toBe(8); // Monday
  });

  it("returns 4 for a custom half-day override", () => {
    const calendar: CalendarSettings = {
      ...MON_FRI_CALENDAR,
      dateOverrides: [
        {
          date: "2026-03-20", // Friday
          type: "custom",
          name: "Short Friday",
          schedule: { hours: [{ start: "09:00", end: "13:00" }] },
        },
      ],
    };
    expect(workingHoursOnDate("2026-03-20", calendar)).toBe(4);
  });

  it("returns 6 for a Mon/Wed/Fri 10–16 calendar", () => {
    expect(workingHoursOnDate("2026-03-16", THREE_DAY_CALENDAR)).toBe(6); // Monday
  });

  it("returns 0 on a non-working day of the three-day calendar", () => {
    expect(workingHoursOnDate("2026-03-17", THREE_DAY_CALENDAR)).toBe(0); // Tuesday
  });
});

// ── createGregorianCalendar ───────────────────────────────────────────────────

describe("createGregorianCalendar", () => {
  it("returns a CalendarSettings with id 'gregorian'", () => {
    expect(createGregorianCalendar().id).toBe("gregorian");
  });

  it("has name 'Gregorian'", () => {
    expect(createGregorianCalendar().name).toBe("Gregorian");
  });

  it("has weekStartsOn=0 (Sunday)", () => {
    expect(createGregorianCalendar().weekStartsOn).toBe(0);
  });

  // The V2 default is 24/7 — all 7 days, midnight-to-midnight.
  // We use a full week of dates: 2026-03-15 (Sun) through 2026-03-21 (Sat).
  it("all 7 days of the week are working days", () => {
    const cal = createGregorianCalendar();
    const allSevenDays = [
      "2026-03-15", // Sunday
      "2026-03-16", // Monday
      "2026-03-17", // Tuesday
      "2026-03-18", // Wednesday
      "2026-03-19", // Thursday
      "2026-03-20", // Friday
      "2026-03-21", // Saturday
    ];
    for (const d of allSevenDays) {
      expect(isWorkingDay(d, cal)).toBe(true);
    }
  });

  it("each day has exactly 24 working hours (00:00–24:00)", () => {
    const cal = createGregorianCalendar();
    const allSevenDays = [
      "2026-03-15", // Sunday
      "2026-03-16", // Monday
      "2026-03-17", // Tuesday
      "2026-03-18", // Wednesday
      "2026-03-19", // Thursday
      "2026-03-20", // Friday
      "2026-03-21", // Saturday
    ];
    for (const d of allSevenDays) {
      expect(workingHoursOnDate(d, cal)).toBe(24);
    }
  });

  it("is marked as built-in", () => {
    expect(createGregorianCalendar().isBuiltIn).toBe(true);
  });

  it("has a non-empty timezone string", () => {
    const tz = createGregorianCalendar().timezone;
    expect(typeof tz).toBe("string");
    expect(tz.length).toBeGreaterThan(0);
  });

  it("starts with empty dateOverrides", () => {
    expect(createGregorianCalendar().dateOverrides).toHaveLength(0);
  });
});
