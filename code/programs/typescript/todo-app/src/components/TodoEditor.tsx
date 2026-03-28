/**
 * TodoEditor.tsx — Create and edit form for tasks.
 *
 * This component handles both creating new tasks and editing existing ones.
 * The mode is determined by the `todoId` prop:
 *   - undefined → create mode (empty form, due date defaults to today)
 *   - string → edit mode (pre-filled with existing data + activity history)
 *
 * === Form design ===
 *
 * Fields:
 *   - Title (required, text input)
 *   - Description (optional, textarea)
 *   - Priority (select dropdown)
 *   - Category (free-form text input with datalist for suggestions)
 *   - Due date (date picker, defaults to today for new tasks)
 *
 * The form uses controlled components (state-backed inputs). On submit,
 * it dispatches the appropriate Flux action (TASK_CREATE or TASK_UPDATE) and
 * navigates back to the task list.
 *
 * === Validation ===
 *
 * Only the title is required. If empty, the form shows an inline error
 * and prevents submission. No external validation library — just a simple
 * state check.
 *
 * === Task History (edit mode only) ===
 *
 * When editing, the component queries the audit log via
 * getActivitiesForEntity(storage, taskId) and renders a timeline of
 * every action taken on this task — creation, updates, status changes.
 *
 * This is the first consumer of the audit log we built in audit.ts.
 * Future consumers: streak detection, activity feeds, crash recovery.
 */

import { useState, useEffect } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { createTaskAction, updateTaskAction } from "../actions.js";
import { getUniqueCategories } from "../types.js";
import { getActivitiesForEntity } from "../audit.js";
import { getStorage } from "../storage.js";
import type { Priority } from "../types.js";
import type { AuditEvent } from "../audit.js";

interface TodoEditorProps {
  /** If provided, edit this todo. If undefined, create a new one. */
  todoId?: string;
  /** Navigate back to list after save/cancel. */
  onNavigate: (path: string) => void;
}

// ── Date helpers ──────────────────────────────────────────────────────────────

/**
 * todayLocal — returns today's date as a YYYY-MM-DD string in local time.
 *
 * We use local time (not UTC) because the date picker renders in the
 * user's local timezone. Using toISOString() would return UTC, which
 * could show yesterday's date for users west of UTC at night.
 *
 * Example: A user in UTC-5 at 11pm on March 28 would get "2026-03-27"
 * from toISOString(), but "2026-03-28" from this function — which is
 * what the calendar picker shows and what the user expects.
 */
function todayLocal(): string {
  const d = new Date();
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

// ── History helpers ───────────────────────────────────────────────────────────

/**
 * describeEvent — converts an AuditEvent into a human-readable summary.
 *
 * Rather than showing raw action type names like "TASK_UPDATE", we
 * generate friendly descriptions:
 *
 *   TASK_CREATE           → "Task created"
 *   TASK_UPDATE (title)   → "Updated: title"
 *   TASK_UPDATE (multi)   → "Updated: priority, due date"
 *   TASK_TOGGLE_STATUS    → "Status toggled"
 *   TASK_SET_STATUS       → "Status set to done"
 *   TASK_DELETE           → "Task deleted"
 *
 * The patch field in TASK_UPDATE tells us exactly which fields changed,
 * so we can list them by name.
 */
function describeEvent(event: AuditEvent): string {
  const action = event.action as Record<string, unknown>;

  switch (event.actionType) {
    case "TASK_CREATE":
      return "Task created";

    case "TASK_UPDATE": {
      // The patch is a partial record of just the changed fields.
      const patch = action.patch as Record<string, unknown> | undefined;
      if (!patch) return "Task updated";

      // Only mention fields that are actually present (were set by the user).
      const changed = Object.keys(patch).filter((k) => k in patch);
      if (changed.length === 0) return "Task updated";

      // Map field names to user-friendly labels.
      const labels: Record<string, string> = {
        title: "title",
        description: "description",
        priority: "priority",
        category: "category",
        dueDate: "due date",
        dueTime: "due time",
      };
      const readable = changed.map((f) => labels[f] ?? f);
      return `Updated: ${readable.join(", ")}`;
    }

    case "TASK_TOGGLE_STATUS":
      return "Status toggled";

    case "TASK_SET_STATUS": {
      const status = action.status as string | undefined;
      return status ? `Status set to ${status}` : "Status changed";
    }

    case "TASK_DELETE":
      return "Task deleted";

    default:
      // Fallback: show a cleaned-up version of the action type string.
      return event.actionType.toLowerCase().replace(/_/g, " ");
  }
}

/**
 * formatRelativeTime — converts a Unix timestamp to a relative label.
 *
 * Examples:
 *   < 60s  → "Just now"
 *   < 1h   → "5m ago"
 *   < 24h  → "3h ago"
 *   older  → "Mar 28, 2:45 PM"
 *
 * We use relative labels for recent events because "3m ago" is more
 * meaningful than "2:47:32 PM" when glancing at a history list.
 */
function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;

  if (diff < 60_000) return "Just now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;

  return new Date(timestamp).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ── Component ─────────────────────────────────────────────────────────────────

export function TodoEditor({ todoId, onNavigate }: TodoEditorProps) {
  const state = useStore(store);
  const existingTodo = todoId ? state.tasks.find((t) => t.id === todoId) : undefined;
  const isEditing = Boolean(existingTodo);
  const categories = getUniqueCategories(state.tasks);

  // ── Form state ──────────────────────────────────────────────────────────
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState<Priority>("medium");
  const [category, setCategory] = useState("");
  // Default to today for new tasks. For edits, the useEffect below
  // overrides this with the existing task's stored dueDate.
  const [dueDate, setDueDate] = useState(() => todoId ? "" : todayLocal());
  const [titleError, setTitleError] = useState(false);

  // ── Activity history (edit mode only) ───────────────────────────────────
  //
  // Three states form the async boundary for history loading:
  //   "loading"  — IDB query in flight; show a skeleton/spinner
  //   "ready"    — query resolved; events array (may be empty)
  //   "idle"     — not in edit mode; don't query at all
  //
  // We use an explicit status string rather than a boolean isLoading flag
  // so that future states (e.g., "error") can be added without changing
  // the shape of the state.
  const [history, setHistory] = useState<AuditEvent[]>([]);
  const [historyStatus, setHistoryStatus] = useState<"idle" | "loading" | "ready">("idle");

  // ── Pre-fill form when editing ──────────────────────────────────────────
  useEffect(() => {
    if (existingTodo) {
      setTitle(existingTodo.title);
      setDescription(existingTodo.description);
      setPriority(existingTodo.priority);
      setCategory(existingTodo.category);
      setDueDate(existingTodo.dueDate ?? "");
    }
  }, [existingTodo]);

  // ── Load activity history when editing ──────────────────────────────────
  //
  // Async boundary: the form renders immediately (no blocking) and the
  // history section transitions through: idle → loading → ready.
  //
  // We use a `cancelled` flag to guard against setting state on an
  // unmounted component (e.g., user navigates away before IDB resolves).
  // This is the standard pattern for async effects in React — without it,
  // a slow IDB query could resolve after navigation and try to setState
  // on a component that no longer exists.
  //
  //   ┌──────────┐  mount/todoId   ┌─────────┐  IDB resolves  ┌───────┐
  //   │   idle   │ ──────────────▶ │ loading │ ─────────────▶ │ ready │
  //   └──────────┘                 └─────────┘                └───────┘
  //
  useEffect(() => {
    if (!todoId) {
      setHistoryStatus("idle");
      return;
    }

    setHistoryStatus("loading");
    let cancelled = false;

    getActivitiesForEntity(getStorage(), todoId)
      .then((events) => {
        if (cancelled) return;
        setHistory(events);
        setHistoryStatus("ready");
      })
      .catch(() => {
        if (cancelled) return;
        // Silently swallow storage errors — history is informational only.
        // The form still works; we just show an empty history.
        setHistory([]);
        setHistoryStatus("ready");
      });

    return () => {
      cancelled = true;
    };
  }, [todoId]);

  // ── If editing a todo that doesn't exist, show error ────────────────────
  if (todoId && !existingTodo) {
    return (
      <div className="empty-state" id="todo-not-found">
        <div className="empty-state__icon">❓</div>
        <h2>Task not found</h2>
        <p>The task you're looking for doesn't exist or was deleted.</p>
        <button
          className="btn btn--primary"
          onClick={() => onNavigate("/")}
          type="button"
        >
          Back to list
        </button>
      </div>
    );
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();

    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      setTitleError(true);
      return;
    }

    if (isEditing && todoId) {
      // Update existing todo
      store.dispatch(
        updateTaskAction(todoId, {
          title: trimmedTitle,
          description: description.trim(),
          priority,
          category: category.trim(),
          dueDate: dueDate || null,
        }),
      );
    } else {
      // Create new task
      store.dispatch(
        createTaskAction(
          trimmedTitle,
          description.trim(),
          priority,
          category.trim(),
          dueDate || null,
          null,
        ),
      );
    }

    onNavigate("/");
  }

  function handleCancel() {
    onNavigate("/");
  }

  return (
    <div className="editor" id="todo-editor">
      <form className="editor__form" onSubmit={handleSubmit}>
        <h2 className="editor__heading">
          {isEditing ? "Edit Task" : "New Task"}
        </h2>

        {/* ── Title ──────────────────────────────────────────────────── */}
        <div className={`editor__field ${titleError ? "editor__field--error" : ""}`}>
          <label htmlFor="todo-title" className="editor__label">
            Title <span className="editor__required">*</span>
          </label>
          <input
            type="text"
            id="todo-title"
            className="editor__input"
            value={title}
            onChange={(e) => {
              setTitle(e.target.value);
              if (e.target.value.trim()) setTitleError(false);
            }}
            placeholder="What needs to be done?"
            maxLength={200}
            autoFocus
          />
          {titleError && (
            <span className="editor__error" id="title-error">
              Title is required
            </span>
          )}
        </div>

        {/* ── Description ────────────────────────────────────────────── */}
        <div className="editor__field">
          <label htmlFor="todo-description" className="editor__label">
            Description
          </label>
          <textarea
            id="todo-description"
            className="editor__textarea"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Add details, notes, or context..."
            rows={4}
          />
        </div>

        {/* ── Priority + Category (side by side) ─────────────────────── */}
        <div className="editor__row">
          <div className="editor__field">
            <label htmlFor="todo-priority" className="editor__label">
              Priority
            </label>
            <select
              id="todo-priority"
              className="editor__select"
              value={priority}
              onChange={(e) => setPriority(e.target.value as Priority)}
            >
              <option value="low">🟢 Low</option>
              <option value="medium">🟡 Medium</option>
              <option value="high">🟠 High</option>
              <option value="urgent">🔴 Urgent</option>
            </select>
          </div>

          <div className="editor__field">
            <label htmlFor="todo-category" className="editor__label">
              Category
            </label>
            <input
              type="text"
              id="todo-category"
              className="editor__input"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              placeholder="e.g. work, personal"
              list="category-suggestions"
            />
            <datalist id="category-suggestions">
              {categories.map((cat) => (
                <option key={cat} value={cat} />
              ))}
            </datalist>
          </div>
        </div>

        {/* ── Due date ───────────────────────────────────────────────── */}
        <div className="editor__field">
          <label htmlFor="todo-due-date" className="editor__label">
            Due Date
          </label>
          <input
            type="date"
            id="todo-due-date"
            className="editor__input"
            value={dueDate}
            onChange={(e) => setDueDate(e.target.value)}
          />
        </div>

        {/* ── Actions ────────────────────────────────────────────────── */}
        <div className="editor__actions">
          <button
            type="button"
            className="btn btn--ghost"
            onClick={handleCancel}
            id="cancel-btn"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="btn btn--primary"
            id="save-btn"
          >
            {isEditing ? "Save Changes" : "Create Task"}
          </button>
        </div>
      </form>

      {/* ── Activity history (edit mode only) ────────────────────────────── */}
      {/*
       * We only show history when editing an existing task, because a new
       * task has no events yet (the CREATE event is written asynchronously
       * by the audit middleware after dispatch, so it wouldn't be available
       * here anyway).
       *
       * The history loads asynchronously from IndexedDB. If there are no
       * events yet (first edit, or audit log was compacted), we show nothing
       * rather than a "no history" message — to keep the UI clean.
       */}
      {isEditing && history.length > 0 && (
        <section className="history" aria-label="Task activity history">
          <h3 className="history__heading">Activity</h3>
          <ol className="history__list">
            {history.map((event) => (
              <li key={event.id} className="history__item">
                <span className="history__dot" aria-hidden="true" />
                <span className="history__label">{describeEvent(event)}</span>
                <time
                  className="history__time"
                  dateTime={new Date(event.timestamp).toISOString()}
                  title={new Date(event.timestamp).toLocaleString()}
                >
                  {formatRelativeTime(event.timestamp)}
                </time>
              </li>
            ))}
          </ol>
        </section>
      )}
    </div>
  );
}
