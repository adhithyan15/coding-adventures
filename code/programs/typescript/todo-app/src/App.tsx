/**
 * App.tsx — Root component with hash-based routing.
 *
 * The app uses URL hash routing (#/path) because it requires zero server
 * configuration. A hash change fires window.onhashchange, which React
 * listens to via useState + useEffect to re-render the active screen.
 *
 * Route table:
 *   #/           → TodoList (main list view)
 *   #/calendar   → TodoCalendar (monthly calendar view)
 *   #/new        → TodoEditor (create)
 *   #/edit/:id   → TodoEditor (edit existing)
 *
 * The navigate(path) function is passed down to every screen component.
 * Components call it instead of touching window.location directly. This
 * keeps navigation logic testable and centralised.
 *
 * === Navigation bar ===
 *
 * The app header now contains a two-item nav bar:
 *   • List       — goes to #/
 *   • Calendar   — goes to #/calendar
 *
 * The active nav item gets the `.app-nav__item--active` modifier class.
 * We compute the active route from the current path rather than storing
 * it separately, so the nav always reflects the real URL.
 */

import { useState, useEffect } from "react";
import { TodoList } from "./components/TodoList.js";
import { TodoEditor } from "./components/TodoEditor.js";
import { TodoCalendar } from "./components/TodoCalendar.js";

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

  // #/calendar — Monthly calendar view
  if (path === "/calendar") {
    return <TodoCalendar onNavigate={onNavigate} />;
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

/**
 * activeNavRoute — maps any path to the top-level nav key.
 *
 * The list route covers /, /new, and /edit/:id because those are all
 * actions within the list context. The calendar is its own top-level route.
 */
function activeNavRoute(path: string): "list" | "calendar" | null {
  if (path === "/" || path === "" || path === "/new" || path.startsWith("/edit/")) {
    return "list";
  }
  if (path === "/calendar") {
    return "calendar";
  }
  return null;
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

  const activeNav = activeNavRoute(path);

  return (
    <div className="app" id="todo-app">
      <header className="app-header" id="app-header">
        <div className="app-header__content">
          {/* Brand */}
          <h1
            className="app-header__title"
            onClick={() => handleNavigate("/")}
            id="app-title"
          >
            <span className="app-header__icon">✓</span>
            Todo
          </h1>

          {/* Top-level navigation */}
          <nav className="app-nav" aria-label="Main navigation">
            <button
              type="button"
              className={[
                "app-nav__item",
                activeNav === "list" ? "app-nav__item--active" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => handleNavigate("/")}
              aria-current={activeNav === "list" ? "page" : undefined}
            >
              ☰ List
            </button>
            <button
              type="button"
              className={[
                "app-nav__item",
                activeNav === "calendar" ? "app-nav__item--active" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => handleNavigate("/calendar")}
              aria-current={activeNav === "calendar" ? "page" : undefined}
            >
              📅 Calendar
            </button>
          </nav>
        </div>
      </header>
      <main className="app-main" id="app-main">
        {renderScreen(path, handleNavigate)}
      </main>
    </div>
  );
}
