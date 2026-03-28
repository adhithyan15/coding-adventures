/**
 * types.ts — All TypeScript interfaces for the todo app.
 *
 * The data model is intentionally flat and simple. Each Task is a
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
 *
 * DUE TIME is stored separately as "HH:MM" (24-hour) or null.
 *   - Keeping it separate from dueDate lets you have a deadline date with
 *     no specific time (all-day) vs. a precise time-boxed appointment.
 *   - The calendar's agenda view places timed tasks in hour slots and
 *     untimed tasks in an "All Day" section at the top.
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

export type TaskStatus = "todo" | "in-progress" | "done";

/**
 * @deprecated Use TaskStatus instead.
 * Kept temporarily for any legacy references.
 */
export type TodoStatus = TaskStatus;

// ── Task ──────────────────────────────────────────────────────────────────
//
// The central data structure. Every field uses JSON-serializable primitives
// (string, number, null) so the record works with IndexedDB, JSON export,
// REST APIs, and SQL databases without adapters.
//
// Previously named "TodoItem" — renamed to "Task" to reflect that this data
// model is the foundation for views, the constraint engine, and future features
// like projects, assignments, and scheduling.

export interface Task {
  /** Unique identifier. Generated via crypto.randomUUID(). */
  id: string;

  /** Short title displayed in the list. Required, 1–200 characters. */
  title: string;

  /** Optional longer description. Supports plain text only (no markdown). */
  description: string;

  /** Current lifecycle status. Determines which column/filter shows this item. */
  status: TaskStatus;

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

  /**
   * Due time as "HH:MM" 24-hour string, or null if no specific time.
   *
   * Kept separate from dueDate so you can have:
   *   - dueDate: "2026-03-28", dueTime: null  → all-day deadline
   *   - dueDate: "2026-03-28", dueTime: "14:30" → specific appointment
   *
   * The agenda view uses dueTime to place tasks in hour slots.
   * Tasks without a dueTime appear in an "All Day" section.
   *
   * Backward compatibility: existing tasks loaded from IndexedDB without
   * this field are normalized to { dueTime: null, ...task } on startup.
   */
  dueTime: string | null;

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

/**
 * @deprecated Use Task instead.
 * Kept as a type alias for any legacy references during the transition.
 */
export type TodoItem = Task;

// ── Filter & Sort ─────────────────────────────────────────────────────────
//
// SortField and SortDirection are used by list and kanban views to order tasks.

export type SortField = "createdAt" | "dueDate" | "priority" | "title" | "updatedAt";
export type SortDirection = "asc" | "desc";

/**
 * FilterState — ephemeral UI filter state (not persisted).
 *
 * This is used by the legacy TodoList component for its local filter bar.
 * New view-based filtering uses TaskFilter from views.ts instead.
 */
export interface FilterState {
  /** Free-text search. Matches against title and description (case-insensitive). */
  search: string;

  /** Filter by status. null = show all statuses. */
  status: TaskStatus | null;

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
 * isOverdue — true if the task has a due date that's in the past.
 *
 * A task is overdue if:
 *   1. It has a due date (not null)
 *   2. The due date is before today (string comparison works for ISO dates)
 *   3. It is NOT already done (completed items can't be overdue)
 */
export function isOverdue(task: Task): boolean {
  if (!task.dueDate || task.status === "done") return false;
  return task.dueDate < todayDateString();
}

/**
 * isDueToday — true if the task is due today and not yet done.
 */
export function isDueToday(task: Task): boolean {
  if (!task.dueDate || task.status === "done") return false;
  return task.dueDate === todayDateString();
}

/**
 * getUniqueCategories — extracts all unique categories from a task list.
 *
 * Returns sorted, non-empty category strings. Used for the filter dropdown.
 */
export function getUniqueCategories(tasks: Task[]): string[] {
  const categories = new Set<string>();
  for (const task of tasks) {
    if (task.category.trim() !== "") {
      categories.add(task.category.trim().toLowerCase());
    }
  }
  return Array.from(categories).sort();
}
