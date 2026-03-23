/**
 * TodoList — shows all todo items grouped by status.
 *
 * Three groups rendered in order: To Do, In Progress, Done.
 * Each item has a status toggle button (cycles through statuses),
 * an edit button, and a delete button with confirmation.
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { toggleTodoAction, deleteTodoAction } from "../actions.js";
import type { TodoItem, TodoStatus } from "../types.js";

interface TodoListProps {
  onNavigate: (path: string) => void;
}

const STATUS_ORDER: TodoStatus[] = ["todo", "in-progress", "done"];

function StatusGroup({
  status,
  todos,
  onNavigate,
}: {
  status: TodoStatus;
  todos: TodoItem[];
  onNavigate: (path: string) => void;
}) {
  const { t } = useTranslation();
  const statusLabels: Record<TodoStatus, string> = {
    "todo": t("todos.status.todo"),
    "in-progress": t("todos.status.inProgress"),
    "done": t("todos.status.done"),
  };

  if (todos.length === 0) return null;

  return (
    <div className="todo-status-group">
      <h2 className="todo-status-group__header">
        <span className={`status-badge status-badge--${status}`}>
          {statusLabels[status]}
        </span>
        <span className="todo-status-group__count">{todos.length}</span>
      </h2>
      <div className="todo-status-group__items">
        {todos.map((todo) => (
          <TodoItemCard key={todo.id} todo={todo} onNavigate={onNavigate} />
        ))}
      </div>
    </div>
  );
}

function TodoItemCard({
  todo,
  onNavigate,
}: {
  todo: TodoItem;
  onNavigate: (path: string) => void;
}) {
  const { t } = useTranslation();

  function handleToggle() {
    store.dispatch(toggleTodoAction(todo.id));
  }

  function handleDelete() {
    const confirmed = window.confirm(
      t("todos.deleteConfirm").replace("{title}", todo.title),
    );
    if (confirmed) {
      store.dispatch(deleteTodoAction(todo.id));
    }
  }

  return (
    <article className={`todo-item${todo.status === "done" ? " todo-item--done" : ""}`}>
      <button
        className="todo-item__toggle"
        onClick={handleToggle}
        type="button"
        aria-label={`Toggle status of ${todo.title}`}
      >
        {todo.status === "done" ? "✓" : todo.status === "in-progress" ? "◐" : "○"}
      </button>
      <div className="todo-item__content">
        <h3 className="todo-item__title">{todo.title}</h3>
        {todo.description && (
          <p className="todo-item__description">{todo.description}</p>
        )}
        {todo.dueDate && (
          <DueDateBadge dueDate={todo.dueDate} isDone={todo.status === "done"} />
        )}
      </div>
      <div className="todo-item__actions">
        <button
          className="btn--secondary"
          onClick={() => onNavigate(`/todos/${todo.id}/edit`)}
          type="button"
        >
          {t("library.edit")}
        </button>
        <button className="btn--danger" onClick={handleDelete} type="button">
          {t("todos.delete")}
        </button>
      </div>
    </article>
  );
}

function DueDateBadge({ dueDate, isDone }: { dueDate: string; isDone: boolean }) {
  const { t } = useTranslation();
  const today = new Date().toISOString().slice(0, 10);
  const isOverdue = !isDone && dueDate < today;
  return (
    <span className={`todo-item__due${isOverdue ? " todo-item__due--overdue" : ""}`}>
      {isOverdue ? `${t("todos.overdue")}: ${dueDate}` : dueDate}
    </span>
  );
}

export function TodoList({ onNavigate }: TodoListProps) {
  const { t } = useTranslation();
  const state = useStore(store);
  const todos = state.todos;

  return (
    <section aria-label={t("todos.title")}>
      <div className="template-library__actions">
        <button
          className="btn--primary"
          onClick={() => onNavigate("/todos/new")}
        >
          {t("todos.newTodo")}
        </button>
      </div>

      {todos.length === 0 ? (
        <p className="template-library__empty">{t("todos.empty")}</p>
      ) : (
        <div className="todo-list">
          {STATUS_ORDER.map((status) => (
            <StatusGroup
              key={status}
              status={status}
              todos={todos.filter((t) => t.status === status)}
              onNavigate={onNavigate}
            />
          ))}
        </div>
      )}
    </section>
  );
}
