/**
 * CalendarViewWrapper.tsx — Bridges SavedView + CalendarSettings → CalendarView.
 *
 * The generic `CalendarView` component in @coding-adventures/ui-components
 * knows nothing about the app's domain (Tasks, views engine, store). This
 * wrapper translates between the two worlds:
 *
 *   1. Looks up the CalendarSettings for this view (by calendarId)
 *   2. Resolves the dateRange to a concrete window (start/end strings)
 *   3. Filters tasks to those whose dateField falls in the window
 *   4. Applies the view's TaskFilter (status, priority, category, search)
 *   5. Passes the result to CalendarView with the right granularity + weekStartsOn
 *
 * === Priority chip styling ===
 *
 * Tasks are rendered as compact "chips" in calendar cells. Chip color encodes
 * priority so the user can scan urgency at a glance even in the compact view:
 *   urgent → red    high → orange    medium → amber    low → grey
 *
 * === Agenda view ===
 *
 * The CalendarView handles agenda layout automatically when granularity="agenda".
 * Tasks with a dueTime appear in hour slots; tasks without go to "All Day".
 * We pass dueTime through via the CalendarItem interface.
 */

import { useMemo } from "react";
import { useStore } from "@coding-adventures/store";
import { CalendarView } from "@coding-adventures/ui-components";
import type { CalendarItem } from "@coding-adventures/ui-components";
import { store } from "../state.js";
import type { Task, Priority } from "../types.js";
import type { CalendarViewConfig } from "../views.js";
import { applyTaskFilter, isTaskInDateRange, resolveDateRange } from "../views.js";

// ── Priority chip color classes ────────────────────────────────────────────

const PRIORITY_CHIP_CLASS: Record<Priority, string> = {
  low:    "cal-chip--low",
  medium: "cal-chip--medium",
  high:   "cal-chip--high",
  urgent: "cal-chip--urgent",
};

// ── CalendarTask — Task shaped as CalendarItem ─────────────────────────────

/**
 * CalendarTask extends CalendarItem so tasks can be passed to CalendarView.
 * We keep a reference to the full Task for renderItem.
 */
interface CalendarTask extends CalendarItem {
  task: Task;
}

// ── Props ──────────────────────────────────────────────────────────────────

interface CalendarViewWrapperProps {
  config: CalendarViewConfig;
}

// ── Component ──────────────────────────────────────────────────────────────

export function CalendarViewWrapper({ config }: CalendarViewWrapperProps) {
  const state = useStore(store);

  // ── Find the calendar settings ────────────────────────────────────────────
  const calendar = state.calendars.find((c) => c.id === config.calendarId)
    ?? state.calendars[0]; // fallback to first if calendarId not found

  // ── Resolve the date window for filtering ─────────────────────────────────
  const dateWindow = useMemo(() => {
    if (!calendar) return null;
    return resolveDateRange(config.dateRange, new Date(), calendar);
  }, [config.dateRange, calendar]);

  // ── Apply filters + date range ────────────────────────────────────────────
  const calendarTasks = useMemo<CalendarTask[]>(() => {
    // 1. Apply the view's own task filter
    let filtered = applyTaskFilter(state.tasks, config.filter);

    // 2. Apply date range filter (tasks whose dateField falls in the window)
    filtered = filtered.filter((task) =>
      isTaskInDateRange(task, config.dateField, dateWindow),
    );

    // 3. Shape each Task as a CalendarTask (with the dueDate field CalendarView needs)
    return filtered.map((task) => ({
      id: task.id,
      dueDate: task.dueDate,
      dueTime: task.dueTime,
      task,
    }));
  }, [state.tasks, config.filter, config.dateField, dateWindow]);

  // ── weekStartsOn from calendar settings ───────────────────────────────────
  const weekStartsOn = calendar?.weekStartsOn ?? 0;

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <CalendarView<CalendarTask>
      items={calendarTasks}
      granularity={config.granularity}
      weekStartsOn={weekStartsOn}
      ariaLabel="Task calendar"
      renderItem={(calTask) => {
        const task = calTask.task;
        const isDone = task.status === "done";
        return (
          <span
            className={[
              "cal-chip",
              PRIORITY_CHIP_CLASS[task.priority],
              isDone ? "cal-chip--done" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            title={task.title}
          >
            {task.title}
          </span>
        );
      }}
    />
  );
}
