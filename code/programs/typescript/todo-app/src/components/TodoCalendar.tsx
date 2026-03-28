/**
 * TodoCalendar.tsx — Calendar view screen for the todo app.
 *
 * This component wires the generic `CalendarView` from `@coding-adventures/ui-components`
 * to the todo app's store and data model.
 *
 * === What it renders ===
 *
 * A read-only monthly grid. Each day cell shows the todos whose `dueDate`
 * matches that day. Each todo is rendered as a compact "chip" — a one-line
 * label colour-coded by priority. Completed todos are shown with reduced
 * opacity and a strikethrough.
 *
 * The view is intentionally read-only for now. There are no click handlers
 * on the chips. Navigation (prev/next month, today) is built into the
 * CalendarView component.
 *
 * === Store subscription ===
 *
 * `useStore(store)` is the reactive hook from `@coding-adventures/store`.
 * It re-renders this component whenever the store dispatches an action that
 * changes `state.todos`. This means the calendar stays in sync with the
 * list view — completing a todo there immediately updates the chip here.
 *
 * === Layout ===
 *
 * The outer `.todo-calendar` div adds a page heading ("Calendar") and a
 * subtitle ("Tasks due each day"). The `CalendarView` component fills the
 * remaining width.
 */

import { useStore } from "@coding-adventures/store";
import { CalendarView } from "@coding-adventures/ui-components";
import { store } from "../state.js";
import type { TodoItem } from "../types.js";

// ── Priority colour mapping ──────────────────────────────────────────────────
//
// Each chip gets a CSS modifier class that sets its background and text
// colour. The mapping lives here so it is co-located with the component
// rather than buried in a stylesheet.
//
//   low     → slate grey chip
//   medium  → amber chip
//   high    → orange chip
//   urgent  → red chip (animated pulse)

const PRIORITY_CLASS: Record<TodoItem["priority"], string> = {
  low: "cal-chip--low",
  medium: "cal-chip--medium",
  high: "cal-chip--high",
  urgent: "cal-chip--urgent",
};

// ── Props ────────────────────────────────────────────────────────────────────

interface TodoCalendarProps {
  onNavigate: (path: string) => void;
}

// ── Component ────────────────────────────────────────────────────────────────

export function TodoCalendar({ onNavigate: _onNavigate }: TodoCalendarProps) {
  // Subscribe to the store. Re-renders whenever todos change.
  const state = useStore(store);

  return (
    <div className="todo-calendar" id="todo-calendar">
      {/* Page heading */}
      <div className="todo-calendar__heading">
        <h2 className="todo-calendar__title">Calendar</h2>
        <p className="todo-calendar__subtitle">Tasks due each day</p>
      </div>

      {/* Generic calendar — items are TodoItems, renderItem draws chips */}
      <CalendarView<TodoItem>
        items={state.todos}
        ariaLabel="Todo calendar"
        renderItem={(todo) => (
          <span
            className={[
              "cal-chip",
              PRIORITY_CLASS[todo.priority],
              todo.status === "done" ? "cal-chip--done" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            title={todo.title}
          >
            {todo.title}
          </span>
        )}
      />
    </div>
  );
}
