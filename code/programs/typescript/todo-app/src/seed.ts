/**
 * seed.ts — Default data for first-time visitors.
 *
 * When a user opens the app for the first time (no data in IndexedDB),
 * we seed the store with:
 *
 *   1. seedTasks     — example tasks demonstrating all features
 *   2. seedViews     — the 5 built-in views (All Tasks, Board, Today,
 *                      This Week, This Month)
 *   3. seedCalendars — the default Gregorian calendar
 *
 * Each dispatch triggers the persistence middleware, so all seeded data
 * is automatically written to IndexedDB for the next visit.
 *
 * === View IDs ===
 *
 * The 5 built-in view IDs are stable constants exported from this file.
 * They appear in URLs (#/view/all-tasks) and in CalendarViewConfig.calendarId.
 * Don't change them after shipping — users' stored activeViewId values would break.
 */

import type { Store } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import {
  createTaskAction,
  toggleStatusAction,
  upsertViewAction,
  upsertCalendarAction,
  setActiveViewAction,
} from "./actions.js";
import type { SavedView } from "./views.js";
import { DEFAULT_TASK_FILTER } from "./views.js";
import { createGregorianCalendar } from "./calendar-settings.js";

// ── Built-in view IDs ─────────────────────────────────────────────────────
//
// Stable identifiers. Appear in URLs and cross-references.

export const VIEW_ID_ALL_TASKS = "all-tasks";
export const VIEW_ID_BOARD = "board";
export const VIEW_ID_TODAY = "today";
export const VIEW_ID_THIS_WEEK = "this-week";
export const VIEW_ID_THIS_MONTH = "this-month";

// ── seedTasks ─────────────────────────────────────────────────────────────

/**
 * seedTasks — dispatches example tasks into the store.
 *
 * Called from main.tsx when IndexedDB returns zero records (first visit).
 * Demonstrates all features: different priorities, categories, statuses,
 * due dates, and due times.
 */
export function seedTasks(store: Store<AppState>): void {
  // ── Example 1: A completed task ──────────────────────────────────────────
  store.dispatch(
    createTaskAction(
      "Welcome to Todo! 👋",
      "This is your offline-first task manager. All your data is stored locally in your browser — no account needed.",
      "low",
      "getting started",
      null,
      null,
    ),
  );
  // Mark it done: todo → in-progress → done
  const firstTask = store.getState().tasks[0];
  if (firstTask) {
    store.dispatch(toggleStatusAction(firstTask.id)); // → in-progress
    store.dispatch(toggleStatusAction(firstTask.id)); // → done
  }

  // ── Example 2: An in-progress task ──────────────────────────────────────
  store.dispatch(
    createTaskAction(
      "Try creating a new task",
      "Click the + button to add your first task. You can set priorities, categories, due dates, and times.",
      "medium",
      "getting started",
      null,
      null,
    ),
  );
  const state2 = store.getState();
  const secondTask = state2.tasks[state2.tasks.length - 1];
  if (secondTask) {
    store.dispatch(toggleStatusAction(secondTask.id)); // → in-progress
  }

  // ── Example 3: High-priority work task with due date + time ─────────────
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const tomorrowStr = tomorrow.toISOString().slice(0, 10);

  store.dispatch(
    createTaskAction(
      "Review project proposal",
      "Read through the proposal document and leave comments. Due tomorrow morning!",
      "high",
      "work",
      tomorrowStr,
      "09:00",
    ),
  );

  // ── Example 4: Urgent personal task ─────────────────────────────────────
  store.dispatch(
    createTaskAction(
      "Buy groceries",
      "Milk, eggs, bread, avocados, and coffee beans.",
      "urgent",
      "personal",
      null,
      null,
    ),
  );

  // ── Example 5: Low-priority future task ─────────────────────────────────
  const nextWeek = new Date();
  nextWeek.setDate(nextWeek.getDate() + 7);
  const nextWeekStr = nextWeek.toISOString().slice(0, 10);

  store.dispatch(
    createTaskAction(
      "Plan weekend hiking trip",
      "Research trails, check weather forecast, pack gear.",
      "low",
      "personal",
      nextWeekStr,
      null,
    ),
  );
}

// ── seedCalendars ─────────────────────────────────────────────────────────

/**
 * seedCalendars — dispatches the default Gregorian calendar into the store.
 *
 * V1 ships with one built-in calendar: Gregorian Mon–Fri 9–5 in the user's
 * local timezone. Users will be able to create custom calendars in future.
 */
export function seedCalendars(store: Store<AppState>): void {
  const gregorian = createGregorianCalendar();
  store.dispatch(upsertCalendarAction(gregorian));
}

// ── seedViews ─────────────────────────────────────────────────────────────

/**
 * seedViews — dispatches the 5 built-in SavedViews into the store.
 *
 * View tab ordering (left to right):
 *   0: All Tasks  — flat list, all statuses, sorted by creation date
 *   1: Board      — kanban grouped by status (rendered as stub in V1)
 *   2: Today      — agenda view, tasks due today
 *   3: This Week  — week grid, tasks due this week
 *   4: This Month — month grid, tasks due this month
 *
 * After seeding, set the active view to "All Tasks" so the user lands
 * on the familiar list view.
 */
export function seedViews(store: Store<AppState>): void {
  const now = Date.now();

  const builtInViews: SavedView[] = [
    // ── All Tasks — flat list ────────────────────────────────────────────
    {
      id: VIEW_ID_ALL_TASKS,
      name: "All Tasks",
      config: {
        type: "list",
        filter: DEFAULT_TASK_FILTER,
        sortField: "createdAt",
        sortDirection: "desc",
      },
      sortOrder: 0,
      isBuiltIn: true,
      createdAt: now,
      updatedAt: now,
    },

    // ── Board — kanban by status (V1 stub) ───────────────────────────────
    {
      id: VIEW_ID_BOARD,
      name: "Board",
      config: {
        type: "kanban",
        filter: DEFAULT_TASK_FILTER,
        groupByField: "status",
        columnOrder: ["todo", "in-progress", "done"],
      },
      sortOrder: 1,
      isBuiltIn: true,
      createdAt: now,
      updatedAt: now,
    },

    // ── Today — agenda view ──────────────────────────────────────────────
    {
      id: VIEW_ID_TODAY,
      name: "Today",
      config: {
        type: "calendar",
        filter: DEFAULT_TASK_FILTER,
        dateField: "dueDate",
        granularity: "agenda",
        dateRange: { type: "rolling", unit: "day", count: 1 },
        calendarId: "gregorian",
      },
      sortOrder: 2,
      isBuiltIn: true,
      createdAt: now,
      updatedAt: now,
    },

    // ── This Week — week grid ────────────────────────────────────────────
    {
      id: VIEW_ID_THIS_WEEK,
      name: "This Week",
      config: {
        type: "calendar",
        filter: DEFAULT_TASK_FILTER,
        dateField: "dueDate",
        granularity: "week",
        dateRange: { type: "rolling", unit: "week", count: 1 },
        calendarId: "gregorian",
      },
      sortOrder: 3,
      isBuiltIn: true,
      createdAt: now,
      updatedAt: now,
    },

    // ── This Month — month grid ──────────────────────────────────────────
    {
      id: VIEW_ID_THIS_MONTH,
      name: "This Month",
      config: {
        type: "calendar",
        filter: DEFAULT_TASK_FILTER,
        dateField: "dueDate",
        granularity: "month",
        dateRange: { type: "rolling", unit: "month", count: 1 },
        calendarId: "gregorian",
      },
      sortOrder: 4,
      isBuiltIn: true,
      createdAt: now,
      updatedAt: now,
    },
  ];

  for (const view of builtInViews) {
    store.dispatch(upsertViewAction(view));
  }

  // Default to "All Tasks" so the user sees the familiar list on first load
  store.dispatch(setActiveViewAction(VIEW_ID_ALL_TASKS));
}
