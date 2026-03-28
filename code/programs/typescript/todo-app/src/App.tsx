/**
 * App.tsx — Root component with hash-based routing and view tabs.
 *
 * The app uses URL hash routing (#/path) because it requires zero server
 * configuration. A hash change fires window.onhashchange, which React
 * listens to via useState + useEffect to re-render the active screen.
 *
 * === Route table ===
 *
 *   #/view/:id   → ViewRenderer (the active view — list, kanban, or calendar)
 *   #/new        → TodoEditor (create new task)
 *   #/edit/:id   → TodoEditor (edit existing task)
 *   #/           → redirect to #/view/all-tasks (default landing)
 *   (anything else) → 404 screen
 *
 * === Navigation bar ===
 *
 * The app header contains dynamic view tabs rendered from `state.views`
 * sorted by `sortOrder`. Each tab navigates to `#/view/:viewId`.
 *
 * The active tab is highlighted with `.app-nav__item--active`.
 * We determine the active tab by matching the current URL path to the
 * view id. Non-view paths (#/new, #/edit/:id) keep the "last active view"
 * tab highlighted by tracking `lastViewId` in state.
 *
 * === View icons ===
 *
 * Each view type gets a representative icon prefix:
 *   list     → ☰
 *   kanban   → 🗂️
 *   calendar → 📅
 */

import { useState, useEffect, useMemo } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "./state.js";
import { setActiveViewAction } from "./actions.js";
import { TodoEditor } from "./components/TodoEditor.js";
import { ViewRenderer } from "./components/ViewRenderer.js";
import { VIEW_ID_ALL_TASKS } from "./seed.js";
import type { SavedView } from "./views.js";

// ── Router ───────────────────────────────────────────────────────────────────

function getHash(): string {
  return window.location.hash.slice(1) || "/";
}

function navigate(path: string): void {
  window.location.hash = path;
}

/**
 * View type icons — prefix in the nav tab.
 *
 * These are simple glyphs that convey the renderer type at a glance.
 */
const VIEW_TYPE_ICON: Record<SavedView["config"]["type"], string> = {
  list:     "☰",
  kanban:   "🗂️",
  calendar: "📅",
};

/**
 * renderScreen — resolves the current hash path to a React component.
 *
 * Order of matching:
 *   1. Exact: #/new → TodoEditor (create)
 *   2. Pattern: #/edit/:id → TodoEditor (edit)
 *   3. Pattern: #/view/:id → ViewRenderer
 *   4. Root: #/ → ViewRenderer(defaultViewId)
 *   5. Fallback → 404
 */
function renderScreen(
  path: string,
  defaultViewId: string,
  onNavigate: (p: string) => void,
): React.ReactNode {
  // #/new — Create new task
  if (path === "/new") {
    return <TodoEditor onNavigate={onNavigate} />;
  }

  // #/edit/:id — Edit existing task
  const editMatch = path.match(/^\/edit\/([^/]+)$/);
  if (editMatch) {
    return <TodoEditor todoId={editMatch[1]} onNavigate={onNavigate} />;
  }

  // #/view/:id — Show a named view
  const viewMatch = path.match(/^\/view\/([^/]+)$/);
  if (viewMatch) {
    return <ViewRenderer viewId={viewMatch[1]!} onNavigate={onNavigate} />;
  }

  // #/ — Redirect to default view
  if (path === "/" || path === "") {
    return <ViewRenderer viewId={defaultViewId} onNavigate={onNavigate} />;
  }

  // 404 — Unknown route
  return (
    <div className="empty-state" id="not-found-screen">
      <div className="empty-state__icon">🔍</div>
      <h2>Page not found</h2>
      <p>The page "{path}" doesn't exist.</p>
      <button
        className="btn btn--primary"
        onClick={() => onNavigate(`/view/${defaultViewId}`)}
        type="button"
        id="back-to-list-btn"
      >
        Back to tasks
      </button>
    </div>
  );
}

/**
 * extractViewId — extracts the view id from a #/view/:id path.
 * Returns null for non-view paths (#/new, #/edit/:id, etc.)
 */
function extractViewId(path: string): string | null {
  const match = path.match(/^\/view\/([^/]+)$/);
  return match ? match[1]! : null;
}

// ── App ──────────────────────────────────────────────────────────────────────

export function App() {
  const [path, setPath] = useState<string>(getHash);
  const state = useStore(store);

  useEffect(() => {
    function onHashChange() {
      setPath(getHash());
    }
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  function handleNavigate(newPath: string) {
    // Sync the store's activeViewId when navigating to a view
    const viewId = extractViewId(newPath);
    if (viewId) {
      store.dispatch(setActiveViewAction(viewId));
    }
    navigate(newPath);
    setPath(newPath);
  }

  // ── Sorted view tabs ──────────────────────────────────────────────────────
  const sortedViews = useMemo(
    () => [...state.views].sort((a, b) => a.sortOrder - b.sortOrder),
    [state.views],
  );

  // ── Active view id ────────────────────────────────────────────────────────
  // The active tab is the view currently shown. For non-view paths (#/new,
  // #/edit/:id), we highlight the store's activeViewId (last-used view).
  const currentViewId = extractViewId(path) ?? state.activeViewId;
  const defaultViewId = state.views.length > 0
    ? (state.views.find((v) => v.sortOrder === 0)?.id ?? VIEW_ID_ALL_TASKS)
    : VIEW_ID_ALL_TASKS;

  return (
    <div className="app" id="todo-app">
      <header className="app-header" id="app-header">
        <div className="app-header__content">
          {/* Brand */}
          <h1
            className="app-header__title"
            onClick={() => handleNavigate(`/view/${defaultViewId}`)}
            id="app-title"
          >
            <span className="app-header__icon">✓</span>
            Todo
          </h1>

          {/* Dynamic view tabs */}
          <nav className="app-nav" aria-label="View navigation">
            {sortedViews.map((view) => {
              const isActive = view.id === currentViewId;
              const icon = VIEW_TYPE_ICON[view.config.type];
              return (
                <button
                  key={view.id}
                  type="button"
                  className={[
                    "app-nav__item",
                    isActive ? "app-nav__item--active" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => handleNavigate(`/view/${view.id}`)}
                  aria-current={isActive ? "page" : undefined}
                  id={`nav-view-${view.id}`}
                >
                  {icon} {view.name}
                </button>
              );
            })}
          </nav>
        </div>
      </header>

      <main className="app-main" id="app-main">
        {renderScreen(path, defaultViewId, handleNavigate)}
      </main>
    </div>
  );
}
