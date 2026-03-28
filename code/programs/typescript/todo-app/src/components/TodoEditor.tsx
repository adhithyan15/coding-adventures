/**
 * TodoEditor.tsx — Create and edit form for todo items.
 *
 * This component handles both creating new todos and editing existing ones.
 * The mode is determined by the `todoId` prop:
 *   - undefined → create mode (empty form)
 *   - string → edit mode (pre-filled with existing data)
 *
 * === Form design ===
 *
 * Fields:
 *   - Title (required, text input)
 *   - Description (optional, textarea)
 *   - Priority (select dropdown)
 *   - Category (free-form text input with datalist for suggestions)
 *   - Due date (date picker)
 *
 * The form uses controlled components (state-backed inputs). On submit,
 * it dispatches the appropriate action (TODO_CREATE or TODO_UPDATE) and
 * navigates back to the list.
 *
 * === Validation ===
 *
 * Only the title is required. If empty, the form shows an inline error
 * and prevents submission. No external validation library — just a simple
 * state check.
 */

import { useState, useEffect } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { createTaskAction, updateTaskAction } from "../actions.js";
import { getUniqueCategories } from "../types.js";
import type { Priority } from "../types.js";

interface TodoEditorProps {
  /** If provided, edit this todo. If undefined, create a new one. */
  todoId?: string;
  /** Navigate back to list after save/cancel. */
  onNavigate: (path: string) => void;
}

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
  const [dueDate, setDueDate] = useState("");
  const [titleError, setTitleError] = useState(false);

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

  // ── If editing a todo that doesn't exist, show error ────────────────────
  if (todoId && !existingTodo) {
    return (
      <div className="empty-state" id="todo-not-found">
        <div className="empty-state__icon">❓</div>
        <h2>Todo not found</h2>
        <p>The todo you're looking for doesn't exist or was deleted.</p>
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
          {isEditing ? "Edit Todo" : "New Todo"}
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
            {isEditing ? "Save Changes" : "Create Todo"}
          </button>
        </div>
      </form>
    </div>
  );
}
