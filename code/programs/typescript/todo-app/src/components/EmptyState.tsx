/**
 * EmptyState.tsx — Friendly empty state component.
 *
 * Displayed when no tasks match the current filters, or when the user
 * has no tasks at all. Uses a large icon and encouraging message to
 * guide the user toward creating their first task.
 *
 * Two modes:
 *   - "no tasks" — the user has zero items. Show a welcome message.
 *   - "no matches" — items exist but none match the filter. Show a
 *     "try adjusting filters" message.
 */

import { t } from "../strings.js";

interface EmptyStateProps {
  /** True when the user has zero tasks total (not just filtered out). */
  hasNoTodos: boolean;
  /** Callback to navigate to the create screen. */
  onCreateClick: () => void;
}

export function EmptyState({ hasNoTodos, onCreateClick }: EmptyStateProps) {
  if (hasNoTodos) {
    return (
      <div className="empty-state" id="empty-state-no-todos">
        <div className="empty-state__icon">📝</div>
        <h2 className="empty-state__title">{t("task.empty.noTasksHeading")}</h2>
        <p className="empty-state__description">
          {t("task.empty.noTasksBody")}
        </p>
        <button
          className="btn btn--primary btn--lg"
          onClick={onCreateClick}
          type="button"
          id="create-first-todo-btn"
        >
          {t("task.empty.noTasksButton")}
        </button>
      </div>
    );
  }

  return (
    <div className="empty-state" id="empty-state-no-matches">
      <div className="empty-state__icon">🔍</div>
      <h2 className="empty-state__title">{t("task.empty.noMatchesHeading")}</h2>
      <p className="empty-state__description">
        {t("task.empty.noMatchesBody")}
      </p>
    </div>
  );
}
