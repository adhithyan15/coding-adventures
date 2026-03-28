/**
 * TodoCard.tsx — Individual todo item card.
 *
 * Displays a single todo with its status, priority, title, description,
 * category, and due date. Provides actions: toggle status, edit, delete.
 *
 * === Visual design ===
 *
 * Each card is a frosted glass panel (glassmorphism). The left border
 * color indicates priority:
 *   - Low: cool blue (#64748b)
 *   - Medium: amber (#f59e0b)
 *   - High: orange (#f97316)
 *   - Urgent: red (#ef4444) with a pulse animation
 *
 * Status is shown as a clickable circle:
 *   - Todo: empty circle (outline only)
 *   - In progress: half-filled circle with spin animation
 *   - Done: filled checkmark, card fades to lower opacity
 *
 * Due date badges:
 *   - Overdue: red badge with "Overdue" text
 *   - Due today: amber badge with "Due today" text
 *   - Future: subtle grey badge with the date
 */

import type { Task } from "../types.js";
import { isOverdue, isDueToday } from "../types.js";

interface TodoCardProps {
  todo: Task;
  onToggleStatus: (id: string) => void;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
}

/**
 * priorityLabel — maps priority to a human-readable label with emoji.
 */
function priorityLabel(priority: Task["priority"]): string {
  switch (priority) {
    case "low": return "Low";
    case "medium": return "Medium";
    case "high": return "High";
    case "urgent": return "Urgent";
  }
}

/**
 * statusIcon — returns an emoji/icon for the current status.
 *
 * The icon doubles as a button — clicking it cycles the status.
 */
function statusIcon(status: Task["status"]): string {
  switch (status) {
    case "todo": return "○";
    case "in-progress": return "◐";
    case "done": return "✓";
  }
}

/**
 * formatDate — converts YYYY-MM-DD to a human-readable format.
 *
 * Uses the browser's Intl.DateTimeFormat for locale-aware formatting.
 * Falls back to the ISO string if formatting fails.
 */
function formatDate(dateStr: string): string {
  try {
    // Parse as local date (not UTC) by replacing hyphens
    const [year, month, day] = dateStr.split("-").map(Number);
    const date = new Date(year!, month! - 1, day);
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: date.getFullYear() !== new Date().getFullYear() ? "numeric" : undefined,
    });
  } catch {
    return dateStr;
  }
}

/**
 * formatTimestamp — converts a Unix timestamp to relative time.
 *
 * Shows "Just now", "5 min ago", "2 hours ago", "Yesterday", or the date.
 */
function formatTimestamp(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (seconds < 60) return "Just now";
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days}d ago`;

  return new Date(timestamp).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

export function TodoCard({ todo, onToggleStatus, onEdit, onDelete }: TodoCardProps) {
  const overdue = isOverdue(todo);
  const dueToday = isDueToday(todo);
  const isDone = todo.status === "done";

  return (
    <article
      className={`todo-card todo-card--${todo.priority} ${isDone ? "todo-card--done" : ""} ${overdue ? "todo-card--overdue" : ""}`}
      id={`todo-card-${todo.id}`}
      data-testid={`todo-card-${todo.id}`}
    >
      {/* ── Status toggle ────────────────────────────────────────────── */}
      <button
        className={`todo-card__status todo-card__status--${todo.status}`}
        onClick={() => onToggleStatus(todo.id)}
        type="button"
        title={`Status: ${todo.status}. Click to change.`}
        aria-label={`Toggle status of "${todo.title}"`}
        id={`toggle-status-${todo.id}`}
      >
        {statusIcon(todo.status)}
      </button>

      {/* ── Content ──────────────────────────────────────────────────── */}
      <div className="todo-card__content" onClick={() => onEdit(todo.id)}>
        <div className="todo-card__header">
          <h3 className={`todo-card__title ${isDone ? "todo-card__title--done" : ""}`}>
            {todo.title}
          </h3>
          <span className={`todo-card__priority todo-card__priority--${todo.priority}`}>
            {priorityLabel(todo.priority)}
          </span>
        </div>

        {todo.description && (
          <p className="todo-card__description">{todo.description}</p>
        )}

        <div className="todo-card__meta">
          {todo.category && (
            <span className="todo-card__category" title="Category">
              {todo.category}
            </span>
          )}

          {todo.dueDate && (
            <span
              className={`todo-card__due ${overdue ? "todo-card__due--overdue" : ""} ${dueToday ? "todo-card__due--today" : ""}`}
              title={`Due: ${todo.dueDate}`}
            >
              {overdue ? "⚠ Overdue" : dueToday ? "📅 Due today" : `📅 ${formatDate(todo.dueDate)}`}
            </span>
          )}

          <span className="todo-card__timestamp" title={`Created: ${new Date(todo.createdAt).toLocaleString()}`}>
            {formatTimestamp(todo.updatedAt)}
          </span>
        </div>
      </div>

      {/* ── Actions ──────────────────────────────────────────────────── */}
      <div className="todo-card__actions">
        <button
          className="todo-card__action"
          onClick={() => onEdit(todo.id)}
          type="button"
          title="Edit"
          aria-label={`Edit "${todo.title}"`}
          id={`edit-todo-${todo.id}`}
        >
          ✎
        </button>
        <button
          className="todo-card__action todo-card__action--delete"
          onClick={() => onDelete(todo.id)}
          type="button"
          title="Delete"
          aria-label={`Delete "${todo.title}"`}
          id={`delete-todo-${todo.id}`}
        >
          🗑
        </button>
      </div>
    </article>
  );
}
