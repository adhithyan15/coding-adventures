/**
 * types.test.ts — Unit tests for type helper functions.
 *
 * Tests the date utility functions (todayDateString, isOverdue, isDueToday)
 * and the category extraction function (getUniqueCategories).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  todayDateString,
  isOverdue,
  isDueToday,
  getUniqueCategories,
  PRIORITY_WEIGHT,
} from "../types.js";
import type { TodoItem } from "../types.js";

// ── Helpers ─────────────────────────────────────────────────────────────────

function makeTodo(overrides: Partial<TodoItem> = {}): TodoItem {
  return {
    id: "test-id",
    title: "Test",
    description: "",
    status: "todo",
    priority: "medium",
    category: "",
    dueDate: null,
    createdAt: 0,
    updatedAt: 0,
    completedAt: null,
    sortOrder: 0,
    ...overrides,
  };
}

// ── Tests ───────────────────────────────────────────────────────────────────

describe("todayDateString", () => {
  it("returns a YYYY-MM-DD formatted string", () => {
    const result = todayDateString();
    expect(result).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it("returns today's date", () => {
    const now = new Date();
    const expected = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
    expect(todayDateString()).toBe(expected);
  });

  it("pads single-digit months and days", () => {
    // Create a date in January (month 0) day 5
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 0, 5)); // January 5, 2026
    expect(todayDateString()).toBe("2026-01-05");
    vi.useRealTimers();
  });
});

describe("isOverdue", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 2, 28)); // March 28, 2026
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns true when due date is in the past", () => {
    const todo = makeTodo({ dueDate: "2026-03-27", status: "todo" });
    expect(isOverdue(todo)).toBe(true);
  });

  it("returns false when due date is today", () => {
    const todo = makeTodo({ dueDate: "2026-03-28", status: "todo" });
    expect(isOverdue(todo)).toBe(false);
  });

  it("returns false when due date is in the future", () => {
    const todo = makeTodo({ dueDate: "2026-03-29", status: "todo" });
    expect(isOverdue(todo)).toBe(false);
  });

  it("returns false when no due date", () => {
    const todo = makeTodo({ dueDate: null, status: "todo" });
    expect(isOverdue(todo)).toBe(false);
  });

  it("returns false when status is done (completed items cannot be overdue)", () => {
    const todo = makeTodo({ dueDate: "2026-03-01", status: "done" });
    expect(isOverdue(todo)).toBe(false);
  });

  it("returns true for in-progress items past due", () => {
    const todo = makeTodo({ dueDate: "2026-03-15", status: "in-progress" });
    expect(isOverdue(todo)).toBe(true);
  });
});

describe("isDueToday", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 2, 28)); // March 28, 2026
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns true when due date is today", () => {
    const todo = makeTodo({ dueDate: "2026-03-28", status: "todo" });
    expect(isDueToday(todo)).toBe(true);
  });

  it("returns false when due date is yesterday", () => {
    const todo = makeTodo({ dueDate: "2026-03-27", status: "todo" });
    expect(isDueToday(todo)).toBe(false);
  });

  it("returns false when due date is tomorrow", () => {
    const todo = makeTodo({ dueDate: "2026-03-29", status: "todo" });
    expect(isDueToday(todo)).toBe(false);
  });

  it("returns false when no due date", () => {
    const todo = makeTodo({ dueDate: null });
    expect(isDueToday(todo)).toBe(false);
  });

  it("returns false when status is done", () => {
    const todo = makeTodo({ dueDate: "2026-03-28", status: "done" });
    expect(isDueToday(todo)).toBe(false);
  });
});

describe("getUniqueCategories", () => {
  it("returns empty array for empty list", () => {
    expect(getUniqueCategories([])).toEqual([]);
  });

  it("extracts unique categories", () => {
    const todos = [
      makeTodo({ category: "work" }),
      makeTodo({ category: "personal" }),
      makeTodo({ category: "work" }),
    ];
    expect(getUniqueCategories(todos)).toEqual(["personal", "work"]);
  });

  it("ignores empty categories", () => {
    const todos = [
      makeTodo({ category: "" }),
      makeTodo({ category: "work" }),
      makeTodo({ category: "   " }),
    ];
    expect(getUniqueCategories(todos)).toEqual(["work"]);
  });

  it("normalizes categories to lowercase", () => {
    const todos = [
      makeTodo({ category: "Work" }),
      makeTodo({ category: "WORK" }),
      makeTodo({ category: "work" }),
    ];
    expect(getUniqueCategories(todos)).toEqual(["work"]);
  });

  it("returns categories sorted alphabetically", () => {
    const todos = [
      makeTodo({ category: "shopping" }),
      makeTodo({ category: "health" }),
      makeTodo({ category: "work" }),
      makeTodo({ category: "arts" }),
    ];
    expect(getUniqueCategories(todos)).toEqual(["arts", "health", "shopping", "work"]);
  });
});

describe("PRIORITY_WEIGHT", () => {
  it("low < medium < high < urgent", () => {
    expect(PRIORITY_WEIGHT.low).toBeLessThan(PRIORITY_WEIGHT.medium);
    expect(PRIORITY_WEIGHT.medium).toBeLessThan(PRIORITY_WEIGHT.high);
    expect(PRIORITY_WEIGHT.high).toBeLessThan(PRIORITY_WEIGHT.urgent);
  });

  it("has entries for all four priorities", () => {
    expect(Object.keys(PRIORITY_WEIGHT)).toHaveLength(4);
    expect(PRIORITY_WEIGHT).toHaveProperty("low");
    expect(PRIORITY_WEIGHT).toHaveProperty("medium");
    expect(PRIORITY_WEIGHT).toHaveProperty("high");
    expect(PRIORITY_WEIGHT).toHaveProperty("urgent");
  });
});
