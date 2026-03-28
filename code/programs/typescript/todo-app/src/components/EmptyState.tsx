/**
 * EmptyState.tsx — Friendly empty state component.
 *
 * Displayed when no todos match the current filters, or when the user
 * has no todos at all. Uses a large icon and encouraging message to
 * guide the user toward creating their first todo.
 *
 * Two modes:
 *   - "no todos" — the user has zero items. Show a welcome message.
 *   - "no matches" — items exist but none match the filter. Show a
 *     "try adjusting filters" message.
 */

interface EmptyStateProps {
  /** True when the user has zero todos total (not just filtered out). */
  hasNoTodos: boolean;
  /** Callback to navigate to the create screen. */
  onCreateClick: () => void;
}

export function EmptyState({ hasNoTodos, onCreateClick }: EmptyStateProps) {
  if (hasNoTodos) {
    return (
      <div className="empty-state" id="empty-state-no-todos">
        <div className="empty-state__icon">📝</div>
        <h2 className="empty-state__title">No todos yet</h2>
        <p className="empty-state__description">
          Create your first task and start getting things done.
          All data stays on your device — no account needed.
        </p>
        <button
          className="btn btn--primary btn--lg"
          onClick={onCreateClick}
          type="button"
          id="create-first-todo-btn"
        >
          Create your first todo
        </button>
      </div>
    );
  }

  return (
    <div className="empty-state" id="empty-state-no-matches">
      <div className="empty-state__icon">🔍</div>
      <h2 className="empty-state__title">No matches</h2>
      <p className="empty-state__description">
        No todos match your current filters. Try adjusting or clearing them.
      </p>
    </div>
  );
}
