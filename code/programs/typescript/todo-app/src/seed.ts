/**
 * seed.ts — Example todos for first-time visitors.
 *
 * When a user opens the app for the first time (no data in IndexedDB),
 * we seed the store with a handful of example todos. This gives the user
 * something to interact with immediately instead of staring at an empty
 * screen.
 *
 * The seed data demonstrates all features: different priorities, categories,
 * statuses, and due dates. Users can delete these and start fresh, or
 * modify them as they wish.
 */

import type { Store } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import { createTodoAction, toggleStatusAction } from "./actions.js";

/**
 * seedTodos — dispatches example todos into the store.
 *
 * Called from main.tsx when IndexedDB returns zero records (first visit).
 * Each dispatch triggers the persistence middleware, so these seed items
 * are automatically written to IndexedDB.
 */
export function seedTodos(store: Store<AppState>): void {
  // ── Example 1: A completed task ────────────────────────────────────────
  store.dispatch(
    createTodoAction(
      "Welcome to Todo! 👋",
      "This is your new offline-first task manager. All your data is stored locally in your browser — no account needed, no internet required.",
      "low",
      "getting started",
      null,
    ),
  );
  // Mark it as done to show the completed state
  const state1 = store.getState();
  const firstTodo = state1.todos[0];
  if (firstTodo) {
    store.dispatch(toggleStatusAction(firstTodo.id));
    store.dispatch(toggleStatusAction(firstTodo.id)); // todo → in-progress → done
  }

  // ── Example 2: An in-progress task ─────────────────────────────────────
  store.dispatch(
    createTodoAction(
      "Try creating a new todo",
      "Click the + button to add your first task. You can set priorities, categories, and due dates.",
      "medium",
      "getting started",
      null,
    ),
  );
  // Mark as in-progress
  const state2 = store.getState();
  const secondTodo = state2.todos[state2.todos.length - 1];
  if (secondTodo) {
    store.dispatch(toggleStatusAction(secondTodo.id)); // todo → in-progress
  }

  // ── Example 3: A high-priority task with due date ──────────────────────
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const tomorrowStr = tomorrow.toISOString().slice(0, 10);

  store.dispatch(
    createTodoAction(
      "Review project proposal",
      "Read through the proposal document and leave comments. Due tomorrow!",
      "high",
      "work",
      tomorrowStr,
    ),
  );

  // ── Example 4: An urgent task ──────────────────────────────────────────
  store.dispatch(
    createTodoAction(
      "Buy groceries",
      "Milk, eggs, bread, avocados, and coffee beans.",
      "urgent",
      "personal",
      null,
    ),
  );

  // ── Example 5: A low-priority task for the future ──────────────────────
  const nextWeek = new Date();
  nextWeek.setDate(nextWeek.getDate() + 7);
  const nextWeekStr = nextWeek.toISOString().slice(0, 10);

  store.dispatch(
    createTodoAction(
      "Plan weekend hiking trip",
      "Research trails, check weather forecast, pack gear.",
      "low",
      "personal",
      nextWeekStr,
    ),
  );
}
