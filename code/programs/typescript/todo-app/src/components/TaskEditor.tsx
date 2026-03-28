/**
 * TaskEditor.tsx — Create and edit form for tasks.
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
 * The history panel transitions through explicit async states:
 *   idle → loading → ready
 * so the form never blocks on the IDB query.
 *
 * === String catalog ===
 *
 * All user-visible text comes from t() (src/strings.ts), keyed into
 * strings.en.json. Swapping the catalog to strings.fr.json translates
 * the entire component without touching this file.
 */

import { useState, useEffect } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { createTaskAction, updateTaskAction } from "../actions.js";
import { getUniqueCategories } from "../types.js";
import { getActivitiesForEntity } from "../audit.js";
import { getStorage } from "../storage.js";
import { t } from "../strings.js";
import type { Priority } from "../types.js";
import type { AuditEvent } from "../audit.js";

interface TaskEditorProps {
  /** If provided, edit this task. If undefined, create a new one. */
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
 * Translates raw action type names to prose via the string catalog.
 * For TASK_UPDATE, the patch field lists exactly which fields changed.
 *
 * Field name → display label mapping:
 *   title       → "title"
 *   description → "description"
 *   priority    → "priority"
 *   category    → "category"
 *   dueDate     → "due date"
 *   dueTime     → "due time"
 */
function describeEvent(event: AuditEvent): string {
  const action = event.action as Record<string, unknown>;

  switch (event.actionType) {
    case "TASK_CREATE":
      return t("task.form.history.eventCreated");

    case "TASK_UPDATE": {
      const patch = action.patch as Record<string, unknown> | undefined;
      if (!patch) return t("task.form.history.eventUpdated");

      const changed = Object.keys(patch).filter((k) => k in patch);
      if (changed.length === 0) return t("task.form.history.eventUpdated");

      // Map field names to user-friendly labels.
      const labels: Record<string, string> = {
        title:       "title",
        description: "description",
        priority:    "priority",
        category:    "category",
        dueDate:     "due date",
        dueTime:     "due time",
      };
      const readable = changed.map((f) => labels[f] ?? f);
      return t("task.form.history.eventUpdatedFields", { fields: readable.join(", ") });
    }

    case "TASK_TOGGLE_STATUS":
      return t("task.form.history.eventStatusToggled");

    case "TASK_SET_STATUS": {
      const status = action.status as string | undefined;
      return status
        ? t("task.form.history.eventStatusSet", { status })
        : t("task.form.history.eventStatusToggled");
    }

    case "TASK_DELETE":
      return t("task.form.history.eventDeleted");

    default:
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
 */
function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;

  if (diff < 60_000) return t("task.form.history.timeJustNow");
  if (diff < 3_600_000) return t("task.form.history.timeMinutes", { n: Math.floor(diff / 60_000) });
  if (diff < 86_400_000) return t("task.form.history.timeHours", { n: Math.floor(diff / 3_600_000) });

  return new Date(timestamp).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ── Component ─────────────────────────────────────────────────────────────────

export function TaskEditor({ todoId, onNavigate }: TaskEditorProps) {
  const state = useStore(store);
  const existingTodo = todoId ? state.tasks.find((task) => task.id === todoId) : undefined;
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
  //   "idle"     — not in edit mode; no query issued; no section rendered
  //   "loading"  — IDB query in flight; show a "Loading…" placeholder
  //   "ready"    — query resolved; render the event list (or nothing if empty)
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
  // The `cancelled` flag prevents setState on an unmounted component
  // (user navigates away before IDB resolves).
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
        setHistory([]);
        setHistoryStatus("ready");
      });

    return () => {
      cancelled = true;
    };
  }, [todoId]);

  // ── If editing a task that doesn't exist, show error ─────────────────────
  if (todoId && !existingTodo) {
    return (
      <div className="empty-state" id="todo-not-found">
        <div className="empty-state__icon">❓</div>
        <h2>{t("task.form.notFound.heading")}</h2>
        <p>{t("task.form.notFound.body")}</p>
        <button
          className="btn btn--primary"
          onClick={() => onNavigate("/")}
          type="button"
        >
          {t("task.form.notFound.back")}
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
          {isEditing ? t("task.form.headingEdit") : t("task.form.headingNew")}
        </h2>

        {/* ── Title ──────────────────────────────────────────────────── */}
        <div className={`editor__field ${titleError ? "editor__field--error" : ""}`}>
          <label htmlFor="todo-title" className="editor__label">
            {t("task.form.titleLabel")} <span className="editor__required">{t("task.form.titleRequired")}</span>
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
            placeholder={t("task.form.titlePlaceholder")}
            maxLength={200}
            autoFocus
          />
          {titleError && (
            <span className="editor__error" id="title-error">
              {t("task.form.titleError")}
            </span>
          )}
        </div>

        {/* ── Description ────────────────────────────────────────────── */}
        <div className="editor__field">
          <label htmlFor="todo-description" className="editor__label">
            {t("task.form.descLabel")}
          </label>
          <textarea
            id="todo-description"
            className="editor__textarea"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={t("task.form.descPlaceholder")}
            rows={4}
          />
        </div>

        {/* ── Priority + Category (side by side) ─────────────────────── */}
        <div className="editor__row">
          <div className="editor__field">
            <label htmlFor="todo-priority" className="editor__label">
              {t("task.form.priorityLabel")}
            </label>
            <select
              id="todo-priority"
              className="editor__select"
              value={priority}
              onChange={(e) => setPriority(e.target.value as Priority)}
            >
              <option value="low">🟢 {t("task.card.priorityLow")}</option>
              <option value="medium">🟡 {t("task.card.priorityMedium")}</option>
              <option value="high">🟠 {t("task.card.priorityHigh")}</option>
              <option value="urgent">🔴 {t("task.card.priorityUrgent")}</option>
            </select>
          </div>

          <div className="editor__field">
            <label htmlFor="todo-category" className="editor__label">
              {t("task.form.categoryLabel")}
            </label>
            <input
              type="text"
              id="todo-category"
              className="editor__input"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              placeholder={t("task.form.categoryPlaceholder")}
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
            {t("task.form.dueDateLabel")}
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
            {t("task.form.cancel")}
          </button>
          <button
            type="submit"
            className="btn btn--primary"
            id="save-btn"
          >
            {isEditing ? t("task.form.submitEdit") : t("task.form.submitNew")}
          </button>
        </div>
      </form>

      {/* ── Activity history (edit mode only) ────────────────────────────── */}
      {/*
       * Shows in edit mode only (new tasks have no history yet).
       * Transitions through idle → loading → ready async states.
       * "loading" shows a placeholder. "ready" with empty events shows nothing.
       */}
      {isEditing && historyStatus === "loading" && (
        <section className="history" aria-label="Task activity history">
          <h3 className="history__heading">{t("task.form.history.heading")}</h3>
          <p className="history__loading">{t("task.form.history.loading")}</p>
        </section>
      )}

      {isEditing && historyStatus === "ready" && history.length > 0 && (
        <section className="history" aria-label="Task activity history">
          <h3 className="history__heading">{t("task.form.history.heading")}</h3>
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
