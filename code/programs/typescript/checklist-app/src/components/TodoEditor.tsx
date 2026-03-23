/**
 * TodoEditor — create or edit a todo item.
 *
 * Simple form with title (required) and description (optional).
 * When editing, also shows a status selector.
 */

import { useState } from "react";
import { useTranslation, DatePicker } from "@coding-adventures/ui-components";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { createTodoAction, updateTodoAction } from "../actions.js";
import type { TodoStatus } from "../types.js";

interface TodoEditorProps {
  todoId?: string;
  onNavigate: (path: string) => void;
}

export function TodoEditor({ todoId, onNavigate }: TodoEditorProps) {
  const { t } = useTranslation();
  const state = useStore(store);
  const existing = todoId ? state.todos.find((td) => td.id === todoId) : undefined;

  const [title, setTitle] = useState(existing?.title ?? "");
  const [description, setDescription] = useState(existing?.description ?? "");
  const [status, setStatus] = useState<TodoStatus>(existing?.status ?? "todo");
  const [dueDate, setDueDate] = useState(existing?.dueDate ?? "");
  const [error, setError] = useState("");

  function handleSave() {
    if (!title.trim()) {
      setError(t("editor.nameRequired"));
      return;
    }
    const dueDateValue = dueDate || null;
    if (existing) {
      store.dispatch(
        updateTodoAction(existing.id, {
          title: title.trim(),
          description: description.trim(),
          status,
          dueDate: dueDateValue,
        }),
      );
    } else {
      store.dispatch(createTodoAction(title.trim(), description.trim(), dueDateValue));
    }
    onNavigate("/todos");
  }

  return (
    <div className="template-editor">
      <h1 className="app-header__title">
        {existing ? t("todos.editorTitleEdit") : t("todos.editorTitleNew")}
      </h1>

      <div className="template-editor__field">
        <label className="template-editor__label" htmlFor="todo-title">
          Title
        </label>
        <input
          id="todo-title"
          type="text"
          value={title}
          onChange={(e) => {
            setTitle(e.target.value);
            setError("");
          }}
          placeholder={t("todos.titlePlaceholder")}
        />
        {error && <p className="template-editor__error">{error}</p>}
      </div>

      <div className="template-editor__field">
        <label className="template-editor__label" htmlFor="todo-desc">
          Description
        </label>
        <textarea
          id="todo-desc"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder={t("todos.descriptionPlaceholder")}
          rows={3}
        />
      </div>

      <div className="template-editor__field">
        <label className="template-editor__label" htmlFor="todo-due">
          {t("todos.dueDate")}
        </label>
        <DatePicker
          value={dueDate}
          onChange={setDueDate}
          label={t("todos.dueDate")}
          id="todo-due"
        />
      </div>

      {existing && (
        <div className="template-editor__field">
          <label className="template-editor__label" htmlFor="todo-status">
            Status
          </label>
          <select
            id="todo-status"
            value={status}
            onChange={(e) => setStatus(e.target.value as TodoStatus)}
            className="todo-status-select"
          >
            <option value="todo">{t("todos.status.todo")}</option>
            <option value="in-progress">{t("todos.status.inProgress")}</option>
            <option value="done">{t("todos.status.done")}</option>
          </select>
        </div>
      )}

      <div className="template-editor__actions">
        <button
          className="btn--secondary"
          onClick={() => onNavigate("/todos")}
          type="button"
        >
          {t("editor.cancel")}
        </button>
        <button className="btn--primary" onClick={handleSave} type="button">
          {t("editor.save")}
        </button>
      </div>
    </div>
  );
}
