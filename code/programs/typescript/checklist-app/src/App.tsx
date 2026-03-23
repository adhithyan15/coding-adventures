/**
 * App.tsx — Root component: hash-based router + screen switcher.
 *
 * Navigation model:
 *   The app uses URL hash routing (#/path) because it requires zero server
 *   configuration. A hash change fires window.onhashchange, which React
 *   listens to via useState + useEffect to re-render the active screen.
 *
 * Route table:
 *   #/                          → TemplateLibrary
 *   #/template/new              → TemplateEditor (create)
 *   #/template/:id/edit         → TemplateEditor (edit)
 *   #/instance/:id              → InstanceRunner
 *   #/instance/:id/stats        → StatsView
 *
 * The navigate(path) function is passed down to every screen component.
 * Components call it instead of touching window.location directly. This
 * keeps navigation logic testable and centralised.
 */

import { useState, useEffect } from "react";
import { TemplateLibrary } from "./components/TemplateLibrary.js";
import { TemplateEditor } from "./components/TemplateEditor.js";
import { InstanceRunner } from "./components/InstanceRunner.js";
import { StatsView } from "./components/StatsView.js";
import { TodoList } from "./components/TodoList.js";
import { TodoEditor } from "./components/TodoEditor.js";

// ── Router ─────────────────────────────────────────────────────────────────

function getHash(): string {
  // Remove the leading '#' from location.hash, default to "/"
  return window.location.hash.slice(1) || "/";
}

function navigate(path: string): void {
  window.location.hash = path;
}

// Resolve the current hash path to a rendered screen.
function renderScreen(
  path: string,
  onNavigate: (p: string) => void,
): React.ReactNode {
  // #/
  if (path === "/" || path === "") {
    return <TemplateLibrary onNavigate={onNavigate} />;
  }

  // #/template/new
  if (path === "/template/new") {
    return <TemplateEditor onNavigate={onNavigate} />;
  }

  // #/template/:id/edit
  const editMatch = path.match(/^\/template\/([^/]+)\/edit$/);
  if (editMatch) {
    return <TemplateEditor templateId={editMatch[1]} onNavigate={onNavigate} />;
  }

  // #/instance/:id/stats
  const statsMatch = path.match(/^\/instance\/([^/]+)\/stats$/);
  if (statsMatch) {
    return <StatsView instanceId={statsMatch[1]} onNavigate={onNavigate} />;
  }

  // #/instance/:id
  const runMatch = path.match(/^\/instance\/([^/]+)$/);
  if (runMatch) {
    return <InstanceRunner instanceId={runMatch[1]} onNavigate={onNavigate} />;
  }

  // #/todos
  if (path === "/todos") {
    return <TodoList onNavigate={onNavigate} />;
  }

  // #/todos/new
  if (path === "/todos/new") {
    return <TodoEditor onNavigate={onNavigate} />;
  }

  // #/todos/:id/edit
  const todoEditMatch = path.match(/^\/todos\/([^/]+)\/edit$/);
  if (todoEditMatch) {
    return <TodoEditor todoId={todoEditMatch[1]} onNavigate={onNavigate} />;
  }

  // 404
  return (
    <div>
      <p>Page not found: {path}</p>
      <button onClick={() => onNavigate("/")} type="button">
        Back to Library
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
    <div>
      <header className="app-header">
        <h1
          className="app-header__title"
          style={{ cursor: "pointer" }}
          onClick={() => handleNavigate("/")}
        >
          Checklist
        </h1>
        <nav className="app-nav">
          <button
            className={`app-nav__tab${!path.startsWith("/todos") ? " app-nav__tab--active" : ""}`}
            onClick={() => handleNavigate("/")}
            type="button"
          >
            Checklists
          </button>
          <button
            className={`app-nav__tab${path.startsWith("/todos") ? " app-nav__tab--active" : ""}`}
            onClick={() => handleNavigate("/todos")}
            type="button"
          >
            Todos
          </button>
        </nav>
      </header>
      <main>{renderScreen(path, handleNavigate)}</main>
    </div>
  );
}
