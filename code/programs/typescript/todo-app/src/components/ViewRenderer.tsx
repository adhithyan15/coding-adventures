/**
 * ViewRenderer.tsx — Dispatches to the correct view renderer by config.type.
 *
 * This component is the "router" inside the views engine. Given a viewId,
 * it:
 *   1. Looks up the SavedView in the store
 *   2. Reads its config.type discriminant
 *   3. Renders the appropriate renderer component:
 *        "list"     → TodoList (existing list view)
 *        "kanban"   → KanbanView (stub in V1)
 *        "calendar" → CalendarViewWrapper
 *
 * The three sub-renderers are passed all tasks from the store plus their
 * specific config. Each renderer owns its own filtering logic — ViewRenderer
 * just routes and passes props.
 *
 * === Fallback behavior ===
 *
 * If the viewId is not found in the store (e.g., stale URL), we show the
 * "view not found" error state with a link back to All Tasks.
 *
 * === View not found ===
 *
 * When a user opens a URL like #/view/my-custom-view but that view was
 * deleted (or the ID is wrong), we show a friendly error rather than a
 * crash.
 */

import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { TodoList } from "./TodoList.js";
import { KanbanView } from "./KanbanView.js";
import { CalendarViewWrapper } from "./CalendarViewWrapper.js";
import type { ListViewConfig, KanbanViewConfig, CalendarViewConfig } from "../views.js";

interface ViewRendererProps {
  viewId: string;
  onNavigate: (path: string) => void;
}

export function ViewRenderer({ viewId, onNavigate }: ViewRendererProps) {
  const state = useStore(store);
  const view = state.views.find((v) => v.id === viewId);

  // ── View not found ─────────────────────────────────────────────────────
  if (!view) {
    return (
      <div className="empty-state" id="view-not-found">
        <div className="empty-state__icon">🔍</div>
        <h2>View not found</h2>
        <p>This view doesn't exist or has been removed.</p>
        <button
          type="button"
          className="btn btn--primary"
          onClick={() => onNavigate("/view/all-tasks")}
          id="back-to-all-tasks-btn"
        >
          Go to All Tasks
        </button>
      </div>
    );
  }

  // ── Dispatch by config.type ────────────────────────────────────────────
  const { config } = view;

  if (config.type === "list") {
    // Reuse the existing TodoList component for the list view.
    // The list view has its own local filter state + sort UI built in.
    // In future, we can pass the saved ListViewConfig to pre-populate it.
    return <TodoList onNavigate={onNavigate} />;
  }

  if (config.type === "kanban") {
    return (
      <KanbanView
        tasks={state.tasks}
        config={config as KanbanViewConfig}
        onNavigate={onNavigate}
      />
    );
  }

  if (config.type === "calendar") {
    return <CalendarViewWrapper config={config as CalendarViewConfig} />;
  }

  // Unknown type — defensive fallback (TypeScript should prevent this)
  return (
    <div className="empty-state">
      <div className="empty-state__icon">⚠️</div>
      <h2>Unknown view type</h2>
      <p>This view type is not supported in this version of the app.</p>
    </div>
  );
}
