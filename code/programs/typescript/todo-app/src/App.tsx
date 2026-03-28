/**
 * App.tsx — Root component with hash-based routing.
 *
 * The app uses URL hash routing (#/path) because it requires zero server
 * configuration. A hash change fires window.onhashchange, which React
 * listens to via useState + useEffect to re-render the active screen.
 *
 * Route table:
 *   #/                → TodoList (main view)
 *   #/new             → TodoEditor (create)
 *   #/edit/:id        → TodoEditor (edit existing)
 *
 * The navigate(path) function is passed down to every screen component.
 * Components call it instead of touching window.location directly. This
 * keeps navigation logic testable and centralised.
 */

import { useState, useEffect } from "react";
import { TodoList } from "./components/TodoList.js";
import { TodoEditor } from "./components/TodoEditor.js";

// ── Router ─────────────────────────────────────────────────────────────────

function getHash(): string {
  return window.location.hash.slice(1) || "/";
}

function navigate(path: string): void {
  window.location.hash = path;
}

/**
 * renderScreen — resolves the current hash path to a React component.
 *
 * Uses regex matching for parameterized routes (e.g., #/edit/:id).
 * Falls back to a 404 screen with a navigation link.
 */
function renderScreen(
  path: string,
  onNavigate: (p: string) => void,
): React.ReactNode {
  // #/ — Main todo list
  if (path === "/" || path === "") {
    return <TodoList onNavigate={onNavigate} />;
  }

  // #/new — Create new todo
  if (path === "/new") {
    return <TodoEditor onNavigate={onNavigate} />;
  }

  // #/edit/:id — Edit existing todo
  const editMatch = path.match(/^\/edit\/([^/]+)$/);
  if (editMatch) {
    return <TodoEditor todoId={editMatch[1]} onNavigate={onNavigate} />;
  }

  // 404 — Unknown route
  return (
    <div className="empty-state" id="not-found-screen">
      <div className="empty-state__icon">🔍</div>
      <h2>Page not found</h2>
      <p>The page "{path}" doesn't exist.</p>
      <button
        className="btn btn--primary"
        onClick={() => onNavigate("/")}
        type="button"
        id="back-to-list-btn"
      >
        Back to list
      </button>
    </div>
  );
}

// ── App ────────────────────────────────────────────────────────────────────

export function App() {
  const [path, setPath] = useState<string>(getHash);

  useEffect(() => {
    function onHashChange() {
      setPath(getHash());
    }
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  function handleNavigate(newPath: string) {
    navigate(newPath);
    // hashchange fires synchronously in some browsers, but not all.
    // Set state directly to guarantee an immediate re-render.
    setPath(newPath);
  }

  return (
    <div className="app" id="todo-app">
      <header className="app-header" id="app-header">
        <div className="app-header__content">
          <h1
            className="app-header__title"
            onClick={() => handleNavigate("/")}
            id="app-title"
          >
            <span className="app-header__icon">✓</span>
            Todo
          </h1>
          <p className="app-header__subtitle">Offline Task Manager</p>
        </div>
      </header>
      <main className="app-main" id="app-main">
        {renderScreen(path, handleNavigate)}
      </main>
    </div>
  );
}
