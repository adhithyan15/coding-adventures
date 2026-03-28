/**
 * views.ts — View types and filter helpers.
 *
 * === What is a "view"? ===
 *
 * A view is an abstraction over the full task list. You give it all tasks.
 * It decides what to show and how to draw it.
 *
 * Each view is a self-contained, persisted entity (stored in IndexedDB).
 * It owns its own filter criteria AND its own layout config.
 * There is no shared filter bag on the outside — the filter lives INSIDE
 * the view's config, because different view types may filter on different
 * fields in different ways.
 *
 * === View types (V1) ===
 *
 *   "list"     — flat sorted list with optional status/priority/category filter
 *   "kanban"   — tasks grouped into columns by any Task field (stub in V1)
 *   "calendar" — tasks placed on a time grid, pivoted by a date field
 *                 supports granularities: agenda | day | week | month
 *
 * === Extensibility ===
 *
 * To add a new view type (e.g., "gantt", "heatmap", "timeline"):
 *   1. Define a new config interface: GanttViewConfig { type: "gantt"; … }
 *   2. Add it to the ViewConfig union
 *   3. Add a renderer component that handles that config type
 *   4. Update ViewRenderer to dispatch to it
 *
 * No schema migrations needed — old views keep their config, new views use
 * the new type. The discriminated union handles everything at compile time.
 *
 * === DateRange ===
 *
 * Time-based views (calendar, future: gantt) use DateRange to decide which
 * tasks are "in scope."
 *
 *   rolling — computes a window relative to "now":
 *     { type:"rolling", unit:"day",   count:1 } → today only
 *     { type:"rolling", unit:"week",  count:1 } → this week (Mon–Sun or Sun–Sat)
 *     { type:"rolling", unit:"month", count:1 } → this calendar month
 *
 *   absolute — fixed YYYY-MM-DD boundaries:
 *     { type:"absolute", start:"2026-04-01", end:"2026-04-30" }
 *
 *   null — no date filtering; show all tasks regardless of dueDate.
 *
 * === TaskFilter ===
 *
 * A composable filter applied by each view renderer before drawing.
 * Views that don't need a filter can set all fields to their "show all" defaults.
 */

import type { Task, TaskStatus, Priority, SortField } from "./types.js";
import type { CalendarSettings } from "./calendar-settings.js";
import type { CalendarViewGranularity, DayOfWeek } from "@coding-adventures/ui-components";

// ── TaskFilter ─────────────────────────────────────────────────────────────

/**
 * TaskFilter — the filter criteria embedded in every view config.
 *
 * Each view owns its own filter instance. Filters reset to "show all"
 * defaults when not specified. Fields are:
 *
 *   statusFilter    — null = show all statuses
 *   priorityFilter  — null = show all priorities
 *   categoryFilter  — "" = show all categories
 *   searchQuery     — "" = no text filter (matches title + description)
 */
export interface TaskFilter {
  statusFilter: TaskStatus | null;
  priorityFilter: Priority | null;
  categoryFilter: string;
  searchQuery: string;
}

/** All-tasks default filter — no restrictions. */
export const DEFAULT_TASK_FILTER: TaskFilter = {
  statusFilter: null,
  priorityFilter: null,
  categoryFilter: "",
  searchQuery: "",
};

// ── DateRange ──────────────────────────────────────────────────────────────

/**
 * DateRange — which tasks are "in scope" for time-based views.
 *
 * rolling:
 *   Computed relative to "now" each time the view renders. Always up to date.
 *     { type:"rolling", unit:"day",   count:1 } → just today
 *     { type:"rolling", unit:"week",  count:1 } → the current week
 *     { type:"rolling", unit:"month", count:1 } → the current month
 *
 * absolute:
 *   Fixed YYYY-MM-DD start/end. Useful for custom "sprint" views or
 *   quarterly reviews. Doesn't move as time passes.
 *
 * null:
 *   No date range restriction. Show tasks regardless of whether they have
 *   a dueDate (or any other date field).
 */
export type DateRange =
  | { type: "rolling"; unit: "day" | "week" | "month"; count: number }
  | { type: "absolute"; start: string; end: string }
  | null;

// ── Re-exports from ui-components ─────────────────────────────────────────
//
// CalendarViewGranularity and DayOfWeek are defined in the CalendarView
// component package and re-exported here for convenience.
export type { CalendarViewGranularity, DayOfWeek } from "@coding-adventures/ui-components";

// ── View Config variants ───────────────────────────────────────────────────

/**
 * ListViewConfig — flat sorted/filtered list of tasks.
 *
 * The simplest view. All tasks flow through the filter, then get sorted.
 * This is the "All Tasks" and "My Day" style view.
 */
export interface ListViewConfig {
  type: "list";
  filter: TaskFilter;
  sortField: SortField;
  sortDirection: "asc" | "desc";
}

/**
 * KanbanViewConfig — tasks organized into columns by a field value.
 *
 * groupByField determines what the columns represent:
 *   "status"   → columns: todo | in-progress | done
 *   "priority" → columns: low | medium | high | urgent
 *   "category" → columns: one per unique category
 *   future: "assignee", "project", "sprint", etc.
 *
 * columnOrder, if provided, gives an explicit left-to-right column ordering.
 * Values not in columnOrder appear after the ordered ones.
 *
 * V1: rendered as a "coming soon" stub. The data model is complete.
 */
export interface KanbanViewConfig {
  type: "kanban";
  filter: TaskFilter;
  /** Which Task field's values become column headers. */
  groupByField: keyof Task;
  /**
   * Optional explicit ordering of column values.
   * Example for status: ["todo", "in-progress", "done"]
   * Example for priority: ["urgent", "high", "medium", "low"]
   */
  columnOrder?: string[];
}

/**
 * CalendarViewConfig — tasks placed on a time grid.
 *
 * dateField     — which date on the task to use as the "placement" field:
 *                   "dueDate"     — standard: show when the task is due
 *                   "createdAt"   — show when tasks were created
 *                   "completedAt" — show a "what I accomplished" history view
 *
 * granularity   — how finely to slice time (see CalendarViewGranularity)
 *
 * dateRange     — which window of time is visible. Rolling = always current.
 *
 * calendarId    — which CalendarSettings drives week-start and working hours.
 *                 References a CalendarSettings.id in the store.
 */
export interface CalendarViewConfig {
  type: "calendar";
  filter: TaskFilter;
  dateField: "dueDate" | "createdAt" | "completedAt";
  granularity: CalendarViewGranularity;
  dateRange: DateRange;
  calendarId: string;
}

/**
 * ViewConfig — the discriminated union of all view renderer configs.
 *
 * The discriminant is the `type` field. Switch on it to get the correct
 * renderer and type-safe config.
 *
 * To add a new view type, add a new interface here.
 */
export type ViewConfig = ListViewConfig | KanbanViewConfig | CalendarViewConfig;

// ── SavedView ──────────────────────────────────────────────────────────────

/**
 * SavedView — a persisted, named view entity.
 *
 * Stored in the IndexedDB "views" object store. Users start with 5 built-in
 * views and will eventually be able to create their own.
 *
 * id          — stable key; also appears in the URL: #/view/:id
 * name        — display name shown in the nav tab: "Today", "Board"
 * config      — owns its own filter + layout. No shared filter bag.
 * sortOrder   — determines tab order in the navigation bar (lower = leftmost)
 * isBuiltIn   — built-in views cannot be deleted (but may be renamed in future)
 * createdAt   — Unix ms timestamp
 * updatedAt   — Unix ms timestamp; updated when the user modifies the view
 */
export interface SavedView {
  id: string;
  name: string;
  config: ViewConfig;
  sortOrder: number;
  isBuiltIn: boolean;
  createdAt: number;
  updatedAt: number;
}

// ── Helpers ────────────────────────────────────────────────────────────────

/**
 * applyTaskFilter — filters a task array using a TaskFilter.
 *
 * Each filter field is ANDed: a task must match ALL active criteria.
 * A field in its "show all" state (null or "") is skipped entirely.
 *
 * Search matches case-insensitively against title and description.
 *
 * Example:
 *   filter = { statusFilter: "todo", priorityFilter: null, ... }
 *   → returns only tasks with status "todo"
 */
export function applyTaskFilter(tasks: Task[], filter: TaskFilter): Task[] {
  return tasks.filter((task) => {
    // Status filter
    if (filter.statusFilter !== null && task.status !== filter.statusFilter) {
      return false;
    }

    // Priority filter
    if (
      filter.priorityFilter !== null &&
      task.priority !== filter.priorityFilter
    ) {
      return false;
    }

    // Category filter (case-insensitive)
    if (
      filter.categoryFilter !== "" &&
      task.category.toLowerCase() !== filter.categoryFilter.toLowerCase()
    ) {
      return false;
    }

    // Search filter (title + description, case-insensitive)
    if (filter.searchQuery !== "") {
      const q = filter.searchQuery.toLowerCase();
      const inTitle = task.title.toLowerCase().includes(q);
      const inDesc = task.description.toLowerCase().includes(q);
      if (!inTitle && !inDesc) return false;
    }

    return true;
  });
}

/**
 * resolveDateRange — computes a concrete { start, end } YYYY-MM-DD window.
 *
 * For "rolling" ranges, we compute relative to `now` using the calendar's
 * weekStartsOn to anchor week boundaries correctly.
 *
 * Returns null if the dateRange is null (no date restriction).
 *
 * === Rolling unit math ===
 *
 * "day" / count=1 → { start: today, end: today }
 * "week" / count=1 → { start: this-week-start, end: this-week-end }
 *   Week start is determined by calendar.weekStartsOn:
 *     0=Sun → week runs Sun–Sat
 *     1=Mon → week runs Mon–Sun
 * "month" / count=1 → { start: first-of-month, end: last-of-month }
 *
 * All dates are computed in local time (not UTC) to match what the user sees.
 */
export function resolveDateRange(
  range: DateRange,
  now: Date,
  calendar: CalendarSettings,
): { start: string; end: string } | null {
  if (range === null) return null;

  if (range.type === "absolute") {
    return { start: range.start, end: range.end };
  }

  // Rolling
  const { unit, count } = range;
  const weekStartsOn: DayOfWeek = calendar.weekStartsOn;

  if (unit === "day") {
    // today × count days
    const startDate = new Date(now);
    const endDate = new Date(now);
    endDate.setDate(endDate.getDate() + count - 1);
    return {
      start: toDateString(startDate),
      end: toDateString(endDate),
    };
  }

  if (unit === "week") {
    // Find the start of the current week (anchored to weekStartsOn)
    const dow = now.getDay() as DayOfWeek;
    // How many days back to go to reach the week start day:
    //   offset = ((dow - weekStartsOn) + 7) % 7
    const offset = ((dow - weekStartsOn) + 7) % 7;
    const weekStart = new Date(now);
    weekStart.setDate(weekStart.getDate() - offset);

    const weekEnd = new Date(weekStart);
    weekEnd.setDate(weekEnd.getDate() + 7 * count - 1);

    return {
      start: toDateString(weekStart),
      end: toDateString(weekEnd),
    };
  }

  if (unit === "month") {
    // Start = first day of current month; end = last day of month × count
    const start = new Date(now.getFullYear(), now.getMonth(), 1);
    const endMonth = now.getMonth() + count;
    // Last day of the final month: go to first of NEXT month, back 1 day
    const end = new Date(now.getFullYear(), endMonth, 0);
    return {
      start: toDateString(start),
      end: toDateString(end),
    };
  }

  return null;
}

/**
 * toDateString — converts a Date to YYYY-MM-DD using LOCAL time (not UTC).
 *
 * Why local time? Because "today" is a calendar date, not a UTC timestamp.
 * A user in GMT-5 opening the app at 23:00 local time would see the wrong
 * date if we used toISOString() (which is UTC).
 */
export function toDateString(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/**
 * isTaskInDateRange — checks whether a task's date field falls within
 * the resolved date range.
 *
 * Returns true if:
 *   - range is null (no restriction)
 *   - the task's dateField value (as YYYY-MM-DD) falls within [start, end]
 *   - string comparison works for ISO dates: "2026-03-15" <= "2026-03-28"
 *
 * Timestamps (createdAt, completedAt) are converted to YYYY-MM-DD local first.
 */
export function isTaskInDateRange(
  task: Task,
  dateField: CalendarViewConfig["dateField"],
  range: { start: string; end: string } | null,
): boolean {
  if (range === null) return true;

  let dateStr: string | null = null;

  if (dateField === "dueDate") {
    dateStr = task.dueDate;
  } else if (dateField === "createdAt") {
    dateStr = toDateString(new Date(task.createdAt));
  } else if (dateField === "completedAt") {
    dateStr = task.completedAt ? toDateString(new Date(task.completedAt)) : null;
  }

  if (dateStr === null) return false;
  return dateStr >= range.start && dateStr <= range.end;
}
