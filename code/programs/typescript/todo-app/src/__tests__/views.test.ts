/**
 * views.test.ts — Unit tests for view filter helpers and date range logic.
 *
 * We test the two core helper functions exported by views.ts:
 *
 *   applyTaskFilter — filters a task array using a TaskFilter
 *   resolveDateRange — computes a concrete date window from a DateRange
 *
 * And the supporting utilities:
 *   toDateString — Date → YYYY-MM-DD (local time)
 *   isTaskInDateRange — tests whether a task falls within a resolved range
 *
 * All date operations use local time, so we freeze to a known date and
 * use date strings that are unambiguous in any timezone.
 *
 * === Frozen date ===
 * 2026-03-15 (Sunday). In March 2026, weeks with weekStartsOn=0 (Sun):
 *   this week = 2026-03-15 (Sun) through 2026-03-21 (Sat)
 * With weekStartsOn=1 (Mon):
 *   this week = 2026-03-09 (Mon) through 2026-03-15 (Sun)
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  applyTaskFilter,
  resolveDateRange,
  toDateString,
  isTaskInDateRange,
  DEFAULT_TASK_FILTER,
} from "../views.js";
import type { Task } from "../types.js";
import type { CalendarSettings } from "../calendar-settings.js";

// ── Freeze time ──────────────────────────────────────────────────────────────

// 2026-03-15 is a Sunday
const FROZEN_DATE = new Date(2026, 2, 15); // March 15, 2026

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(FROZEN_DATE);
});

afterEach(() => {
  vi.useRealTimers();
});

// ── Fixtures ─────────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "t-1",
    title: "Test task",
    description: "A test task",
    status: "todo",
    priority: "medium",
    category: "work",
    dueDate: null,
    dueTime: null,
    createdAt: new Date(2026, 2, 10).getTime(), // 2026-03-10
    updatedAt: new Date(2026, 2, 10).getTime(),
    completedAt: null,
    sortOrder: 0,
    ...overrides,
  };
}

/** A minimal CalendarSettings with weekStartsOn=0 (Sunday). */
const CALENDAR_SUN: CalendarSettings = {
  id: "test-sun",
  name: "Test (Sun start)",
  weekStartsOn: 0,
  weeklySchedule: {},
  dateOverrides: [],
  timezone: "UTC",
  isBuiltIn: false,
  createdAt: 0,
  updatedAt: 0,
};

/** A calendar with weekStartsOn=1 (Monday). */
const CALENDAR_MON: CalendarSettings = {
  ...CALENDAR_SUN,
  id: "test-mon",
  name: "Test (Mon start)",
  weekStartsOn: 1,
};

// ── applyTaskFilter ───────────────────────────────────────────────────────────

describe("applyTaskFilter", () => {
  // ── DEFAULT_TASK_FILTER ──────────────────────────────────────────────────

  it("default filter returns all tasks unchanged", () => {
    const tasks = [
      makeTask({ id: "a", status: "todo" }),
      makeTask({ id: "b", status: "done" }),
      makeTask({ id: "c", priority: "urgent" }),
    ];
    expect(applyTaskFilter(tasks, DEFAULT_TASK_FILTER)).toHaveLength(3);
  });

  it("returns empty array when tasks list is empty", () => {
    expect(applyTaskFilter([], DEFAULT_TASK_FILTER)).toHaveLength(0);
  });

  // ── Status filter ────────────────────────────────────────────────────────

  it("filters by status=todo", () => {
    const tasks = [
      makeTask({ id: "a", status: "todo" }),
      makeTask({ id: "b", status: "done" }),
      makeTask({ id: "c", status: "in-progress" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, statusFilter: "todo" });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("a");
  });

  it("filters by status=done", () => {
    const tasks = [
      makeTask({ id: "a", status: "todo" }),
      makeTask({ id: "b", status: "done" }),
      makeTask({ id: "c", status: "done" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, statusFilter: "done" });
    expect(result).toHaveLength(2);
  });

  it("statusFilter=null shows all statuses", () => {
    const tasks = [
      makeTask({ id: "a", status: "todo" }),
      makeTask({ id: "b", status: "done" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, statusFilter: null });
    expect(result).toHaveLength(2);
  });

  // ── Priority filter ──────────────────────────────────────────────────────

  it("filters by priority=high", () => {
    const tasks = [
      makeTask({ id: "a", priority: "low" }),
      makeTask({ id: "b", priority: "high" }),
      makeTask({ id: "c", priority: "high" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, priorityFilter: "high" });
    expect(result).toHaveLength(2);
    expect(result.map((t) => t.id)).toEqual(["b", "c"]);
  });

  it("filters by priority=urgent", () => {
    const tasks = [
      makeTask({ id: "a", priority: "medium" }),
      makeTask({ id: "b", priority: "urgent" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, priorityFilter: "urgent" });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("b");
  });

  it("priorityFilter=null shows all priorities", () => {
    const tasks = ["low", "medium", "high", "urgent"].map((p, i) =>
      makeTask({ id: `p${i}`, priority: p as Task["priority"] }),
    );
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, priorityFilter: null });
    expect(result).toHaveLength(4);
  });

  // ── Category filter ──────────────────────────────────────────────────────

  it("filters by category (exact match, case-insensitive)", () => {
    const tasks = [
      makeTask({ id: "a", category: "work" }),
      makeTask({ id: "b", category: "personal" }),
      makeTask({ id: "c", category: "Work" }),  // different case
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, categoryFilter: "work" });
    expect(result).toHaveLength(2);
    expect(result.map((t) => t.id)).toEqual(["a", "c"]);
  });

  it("categoryFilter='' shows all categories", () => {
    const tasks = [
      makeTask({ id: "a", category: "work" }),
      makeTask({ id: "b", category: "personal" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, categoryFilter: "" });
    expect(result).toHaveLength(2);
  });

  // ── Search filter ────────────────────────────────────────────────────────

  it("searchQuery matches in title (case-insensitive)", () => {
    const tasks = [
      makeTask({ id: "a", title: "Write tests", description: "" }),
      makeTask({ id: "b", title: "Buy milk", description: "" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, searchQuery: "write" });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("a");
  });

  it("searchQuery matches in description", () => {
    const tasks = [
      makeTask({ id: "a", title: "Task A", description: "Important work item" }),
      makeTask({ id: "b", title: "Task B", description: "Nothing special" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, searchQuery: "important" });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("a");
  });

  it("searchQuery='' shows all tasks", () => {
    const tasks = [
      makeTask({ id: "a", title: "First" }),
      makeTask({ id: "b", title: "Second" }),
    ];
    const result = applyTaskFilter(tasks, { ...DEFAULT_TASK_FILTER, searchQuery: "" });
    expect(result).toHaveLength(2);
  });

  // ── Combined filters ─────────────────────────────────────────────────────

  it("combines status + priority filters (AND logic)", () => {
    const tasks = [
      makeTask({ id: "a", status: "todo", priority: "high" }),
      makeTask({ id: "b", status: "todo", priority: "low" }),
      makeTask({ id: "c", status: "done", priority: "high" }),
    ];
    const result = applyTaskFilter(tasks, {
      statusFilter: "todo",
      priorityFilter: "high",
      categoryFilter: "",
      searchQuery: "",
    });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("a");
  });

  it("combines all four filters", () => {
    const tasks = [
      makeTask({ id: "a", status: "todo", priority: "high", category: "work", title: "Deploy app" }),
      makeTask({ id: "b", status: "todo", priority: "high", category: "work", title: "Read book" }),
      makeTask({ id: "c", status: "done", priority: "high", category: "work", title: "Deploy app" }),
    ];
    const result = applyTaskFilter(tasks, {
      statusFilter: "todo",
      priorityFilter: "high",
      categoryFilter: "work",
      searchQuery: "deploy",
    });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("a");
  });
});

// ── toDateString ──────────────────────────────────────────────────────────────

describe("toDateString", () => {
  it("formats a date as YYYY-MM-DD using local time", () => {
    expect(toDateString(new Date(2026, 2, 15))).toBe("2026-03-15");
  });

  it("pads month and day with leading zeros", () => {
    expect(toDateString(new Date(2026, 0, 5))).toBe("2026-01-05");
  });

  it("handles end of year", () => {
    expect(toDateString(new Date(2026, 11, 31))).toBe("2026-12-31");
  });

  it("handles leap day", () => {
    expect(toDateString(new Date(2024, 1, 29))).toBe("2024-02-29");
  });
});

// ── resolveDateRange ──────────────────────────────────────────────────────────

describe("resolveDateRange", () => {
  const now = FROZEN_DATE; // 2026-03-15, Sunday

  it("returns null for null range", () => {
    expect(resolveDateRange(null, now, CALENDAR_SUN)).toBeNull();
  });

  it("returns absolute range as-is", () => {
    const range = { type: "absolute" as const, start: "2026-04-01", end: "2026-04-30" };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-04-01",
      end: "2026-04-30",
    });
  });

  // ── Rolling: day ─────────────────────────────────────────────────────────

  it("rolling day count=1 → just today", () => {
    const range = { type: "rolling" as const, unit: "day" as const, count: 1 };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-03-15",
      end: "2026-03-15",
    });
  });

  it("rolling day count=7 → today + 6 more days", () => {
    const range = { type: "rolling" as const, unit: "day" as const, count: 7 };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-03-15",
      end: "2026-03-21",
    });
  });

  // ── Rolling: week ────────────────────────────────────────────────────────

  it("rolling week count=1 + weekStartsOn=0 (Sun) → Sun–Sat", () => {
    // now = 2026-03-15 (Sunday). Week start = Sunday = 2026-03-15.
    // Week end = 2026-03-21 (Saturday).
    const range = { type: "rolling" as const, unit: "week" as const, count: 1 };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-03-15",
      end: "2026-03-21",
    });
  });

  it("rolling week count=1 + weekStartsOn=1 (Mon) → Mon–Sun", () => {
    // now = 2026-03-15 (Sunday). Monday start: Mon 2026-03-09 to Sun 2026-03-15.
    const range = { type: "rolling" as const, unit: "week" as const, count: 1 };
    expect(resolveDateRange(range, now, CALENDAR_MON)).toEqual({
      start: "2026-03-09",
      end: "2026-03-15",
    });
  });

  it("rolling week count=2 + weekStartsOn=0 → two weeks starting from this Sunday", () => {
    // Sun 2026-03-15 to Sat 2026-03-28 (14 days)
    const range = { type: "rolling" as const, unit: "week" as const, count: 2 };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-03-15",
      end: "2026-03-28",
    });
  });

  it("rolling week when today is mid-week (Wednesday), weekStartsOn=0", () => {
    // Wednesday 2026-03-18. Sun start = 2026-03-15. End = 2026-03-21.
    const wednesday = new Date(2026, 2, 18);
    const range = { type: "rolling" as const, unit: "week" as const, count: 1 };
    expect(resolveDateRange(range, wednesday, CALENDAR_SUN)).toEqual({
      start: "2026-03-15",
      end: "2026-03-21",
    });
  });

  // ── Rolling: month ───────────────────────────────────────────────────────

  it("rolling month count=1 → first to last day of March 2026", () => {
    const range = { type: "rolling" as const, unit: "month" as const, count: 1 };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-03-01",
      end: "2026-03-31",
    });
  });

  it("rolling month count=2 → March 1 to April 30", () => {
    const range = { type: "rolling" as const, unit: "month" as const, count: 2 };
    expect(resolveDateRange(range, now, CALENDAR_SUN)).toEqual({
      start: "2026-03-01",
      end: "2026-04-30",
    });
  });

  it("rolling month handles December (28 or 31 days)", () => {
    const december = new Date(2026, 11, 15); // December 15
    const range = { type: "rolling" as const, unit: "month" as const, count: 1 };
    expect(resolveDateRange(range, december, CALENDAR_SUN)).toEqual({
      start: "2026-12-01",
      end: "2026-12-31",
    });
  });

  it("rolling month handles February in a non-leap year", () => {
    const february = new Date(2026, 1, 10); // Feb 10, 2026 (not a leap year)
    const range = { type: "rolling" as const, unit: "month" as const, count: 1 };
    expect(resolveDateRange(range, february, CALENDAR_SUN)).toEqual({
      start: "2026-02-01",
      end: "2026-02-28",
    });
  });
});

// ── isTaskInDateRange ─────────────────────────────────────────────────────────

describe("isTaskInDateRange", () => {
  it("returns true when range is null (no restriction)", () => {
    const task = makeTask({ dueDate: "2026-03-15" });
    expect(isTaskInDateRange(task, "dueDate", null)).toBe(true);
  });

  it("returns true when dueDate is within range", () => {
    const task = makeTask({ dueDate: "2026-03-15" });
    expect(isTaskInDateRange(task, "dueDate", { start: "2026-03-01", end: "2026-03-31" })).toBe(true);
  });

  it("returns false when dueDate is before range", () => {
    const task = makeTask({ dueDate: "2026-02-28" });
    expect(isTaskInDateRange(task, "dueDate", { start: "2026-03-01", end: "2026-03-31" })).toBe(false);
  });

  it("returns false when dueDate is after range", () => {
    const task = makeTask({ dueDate: "2026-04-01" });
    expect(isTaskInDateRange(task, "dueDate", { start: "2026-03-01", end: "2026-03-31" })).toBe(false);
  });

  it("returns true for exact start boundary", () => {
    const task = makeTask({ dueDate: "2026-03-01" });
    expect(isTaskInDateRange(task, "dueDate", { start: "2026-03-01", end: "2026-03-31" })).toBe(true);
  });

  it("returns true for exact end boundary", () => {
    const task = makeTask({ dueDate: "2026-03-31" });
    expect(isTaskInDateRange(task, "dueDate", { start: "2026-03-01", end: "2026-03-31" })).toBe(true);
  });

  it("returns false when dueDate is null", () => {
    const task = makeTask({ dueDate: null });
    expect(isTaskInDateRange(task, "dueDate", { start: "2026-03-01", end: "2026-03-31" })).toBe(false);
  });

  it("checks createdAt field (converted to YYYY-MM-DD)", () => {
    // createdAt = 2026-03-10 (from makeTask default)
    const task = makeTask({ createdAt: new Date(2026, 2, 10, 12, 0, 0).getTime() });
    expect(isTaskInDateRange(task, "createdAt", { start: "2026-03-01", end: "2026-03-31" })).toBe(true);
    expect(isTaskInDateRange(task, "createdAt", { start: "2026-04-01", end: "2026-04-30" })).toBe(false);
  });

  it("checks completedAt field when set", () => {
    const task = makeTask({ completedAt: new Date(2026, 2, 20, 12, 0, 0).getTime() });
    expect(isTaskInDateRange(task, "completedAt", { start: "2026-03-15", end: "2026-03-25" })).toBe(true);
  });

  it("returns false for completedAt field when completedAt is null", () => {
    const task = makeTask({ completedAt: null });
    expect(isTaskInDateRange(task, "completedAt", { start: "2026-03-01", end: "2026-03-31" })).toBe(false);
  });
});
