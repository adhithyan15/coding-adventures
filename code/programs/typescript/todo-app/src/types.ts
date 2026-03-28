/**
 * types.ts — All TypeScript interfaces for the todo app.
 *
 * The data model is intentionally flat and simple. Each TodoItem is a
 * self-contained record that can be stored in IndexedDB, serialized to
 * JSON, or sent over a REST API without transformation.
 *
 * === Design Decisions ===
 *
 * PRIORITY uses a string literal union instead of numeric levels because:
 *   - "urgent" is clearer than remembering that 4 means urgent
 *   - String comparisons work for display without a lookup table
 *   - TypeScript's exhaustive checking catches missing cases in switches
 *
 * STATUS follows the Kanban lifecycle: todo → in-progress → done.
 *   - "todo" means the item exists but work hasn't started
 *   - "in-progress" means active work is happening
 *   - "done" means the work is complete
 *   - There is no "deleted" status — deletion removes the record entirely
 *
 * DATES are stored as ISO 8601 strings (YYYY-MM-DD) for due dates and
 * Unix timestamps (Date.now()) for creation/update times. The reasoning:
 *   - Due dates are calendar dates, not instants in time. "March 28" means
 *     the same thing regardless of timezone. Strings preserve this intent.
 *   - Created/updated times are instants. Timestamps are unambiguous.
 *   - HTML <input type="date"> returns YYYY-MM-DD natively, no conversion.
 *   - String comparison works for sorting: "2026-03-25" < "2026-04-01".
 */

// ── Priority ──────────────────────────────────────────────────────────────
//
// Four levels, from "low" (nice to have) to "urgent" (do it right now).
// The ordering is: low < medium < high < urgent.
//
// We define a numeric weight for sorting. Components convert the string
// to a weight via PRIORITY_WEIGHT when comparing items.

export type Priority = "low" | "medium" | "high" | "urgent";

/**
 * Numeric weights for sorting by priority.
 *
 * Higher number = more urgent = appears first in descending sort.
 * This is a const object (not an enum) so it can be used as a value
 * at runtime AND as a type at compile time.
 */
export const PRIORITY_WEIGHT: Record<Priority, number> = {
  low: 1,
  medium: 2,
  high: 3,
  urgent: 4,
};

// ── Status ────────────────────────────────────────────────────────────────

export type TodoStatus = "todo" | "in-progress" | "done";

// ── TodoItem ──────────────────────────────────────────────────────────────
//
// The central data structure. Every field uses JSON-serializable primitives
// (string, number, null) so the record works with IndexedDB, JSON export,
// REST APIs, and SQL databases without adapters.

export interface TodoItem {
  /** Unique identifier. Generated via crypto.randomUUID(). */
  id: string;

  /** Short title displayed in the list. Required, 1–200 characters. */
  title: string;

  /** Optional longer description. Supports plain text only (no markdown). */
  description: string;

  /** Current lifecycle status. Determines which column/filter shows this item. */
  status: TodoStatus;

  /** Urgency level. Affects visual styling (color, badge) and sort order. */
  priority: Priority;

  /**
   * Free-form category tag. Examples: "work", "personal", "shopping".
   *
   * Empty string means uncategorized. Categories are case-insensitive for
   * filtering (stored as user typed, compared via toLowerCase()).
   */
  category: string;

  /**
   * Due date as ISO 8601 date string (YYYY-MM-DD), or null if no deadline.
   *
   * Used for overdue detection: compare with new Date().toISOString().slice(0,10).
   */
  dueDate: string | null;

  /** Unix timestamp from Date.now(). Set once at creation, never changes. */
  createdAt: number;

  /** Unix timestamp. Updated on every mutation (title, status, priority, etc.). */
  updatedAt: number;

  /** Unix timestamp when status changed to "done". null while incomplete. */
  completedAt: number | null;

  /**
   * Position for manual ordering. Lower number = appears first.
   *
   * When a new item is created, it gets sortOrder = Date.now() (large number,
   * so it appears at the bottom). Users can drag to reorder, which swaps
   * sortOrder values between items.
   */
  sortOrder: number;
}

// ── Filter & Sort ─────────────────────────────────────────────────────────
//
// These types describe the current view state (which items are visible and
// in what order). They are NOT persisted — they reset to defaults on page
// reload. This is intentional: the user's data is persistent, but the view
// is ephemeral.

export type SortField = "createdAt" | "dueDate" | "priority" | "title" | "updatedAt";
export type SortDirection = "asc" | "desc";

export interface FilterState {
  /** Free-text search. Matches against title and description (case-insensitive). */
  search: string;

  /** Filter by status. null = show all statuses. */
  status: TodoStatus | null;

  /** Filter by priority. null = show all priorities. */
  priority: Priority | null;

  /** Filter by category. Empty string = show all categories. */
  category: string;

  /** Which field to sort by. */
  sortField: SortField;

  /** Sort direction. */
  sortDirection: SortDirection;
}

// ── Helper functions ──────────────────────────────────────────────────────

/**
 * todayDateString — returns today's date as YYYY-MM-DD.
 *
 * Used for due date comparison. We use local date (not UTC) because
 * "today" means the user's calendar date, not a UTC date.
 */
export function todayDateString(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * isOverdue — true if the todo has a due date that's in the past.
 *
 * A todo is overdue if:
 *   1. It has a due date (not null)
 *   2. The due date is before today (string comparison works for ISO dates)
 *   3. It is NOT already done (completed items can't be overdue)
 */
export function isOverdue(todo: TodoItem): boolean {
  if (!todo.dueDate || todo.status === "done") return false;
  return todo.dueDate < todayDateString();
}

/**
 * isDueToday — true if the todo is due today and not yet done.
 */
export function isDueToday(todo: TodoItem): boolean {
  if (!todo.dueDate || todo.status === "done") return false;
  return todo.dueDate === todayDateString();
}

/**
 * getUniqueCategories — extracts all unique categories from a todo list.
 *
 * Returns sorted, non-empty category strings. Used for the filter dropdown.
 */
export function getUniqueCategories(todos: TodoItem[]): string[] {
  const categories = new Set<string>();
  for (const todo of todos) {
    if (todo.category.trim() !== "") {
      categories.add(todo.category.trim().toLowerCase());
    }
  }
  return Array.from(categories).sort();
}
