/**
 * App.tsx — Root component: hash-based router + screen switcher.
 *
 * Route table:
 *   #/                                  → DeckList (home)
 *   #/deck/new                          → DeckForm (create)
 *   #/deck/:id/edit                     → DeckForm (edit)
 *   #/deck/:id/cards                    → CardList
 *   #/deck/:id/cards/new                → CardForm (create)
 *   #/deck/:id/cards/:cardId/edit       → CardForm (edit)
 *   #/deck/:id/stats                    → DeckStats
 *   #/session                           → StudySession
 *   #/session/complete                  → SessionComplete
 *
 * Hash routing requires zero server configuration and works identically
 * in Electron (file:// protocol) and a web browser.
 */

import { useState, useEffect } from "react";
import { DeckList } from "./components/DeckList.js";
import { DeckForm } from "./components/DeckForm.js";
import { CardList } from "./components/CardList.js";
import { CardForm } from "./components/CardForm.js";
import { StudySession } from "./components/StudySession.js";
import { SessionComplete } from "./components/SessionComplete.js";
import { DeckStats } from "./components/DeckStats.js";

// ── Router ──────────────────────────────────────────────────────────────────

function getHash(): string {
  return window.location.hash.slice(1) || "/";
}

function navigate(path: string): void {
  window.location.hash = path;
}

function renderScreen(
  path: string,
  onNavigate: (p: string) => void,
): React.ReactNode {
  if (path === "/" || path === "") {
    return <DeckList onNavigate={onNavigate} />;
  }

  if (path === "/deck/new") {
    return <DeckForm onNavigate={onNavigate} />;
  }

  if (path === "/session") {
    return <StudySession onNavigate={onNavigate} />;
  }

  if (path === "/session/complete") {
    return <SessionComplete onNavigate={onNavigate} />;
  }

  // #/deck/:id/cards/:cardId/edit — must match BEFORE the shorter patterns
  const cardEditMatch = path.match(/^\/deck\/([^/]+)\/cards\/([^/]+)\/edit$/);
  if (cardEditMatch) {
    return (
      <CardForm
        deckId={cardEditMatch[1]!}
        cardId={cardEditMatch[2]!}
        onNavigate={onNavigate}
      />
    );
  }

  // #/deck/:id/cards/new
  const cardNewMatch = path.match(/^\/deck\/([^/]+)\/cards\/new$/);
  if (cardNewMatch) {
    return <CardForm deckId={cardNewMatch[1]!} onNavigate={onNavigate} />;
  }

  // #/deck/:id/cards
  const cardListMatch = path.match(/^\/deck\/([^/]+)\/cards$/);
  if (cardListMatch) {
    return <CardList deckId={cardListMatch[1]!} onNavigate={onNavigate} />;
  }

  // #/deck/:id/edit
  const deckEditMatch = path.match(/^\/deck\/([^/]+)\/edit$/);
  if (deckEditMatch) {
    return <DeckForm deckId={deckEditMatch[1]!} onNavigate={onNavigate} />;
  }

  // #/deck/:id/stats
  const statsMatch = path.match(/^\/deck\/([^/]+)\/stats$/);
  if (statsMatch) {
    return <DeckStats deckId={statsMatch[1]!} onNavigate={onNavigate} />;
  }

  return (
    <div>
      <p>Page not found: {path}</p>
      <button type="button" onClick={() => onNavigate("/")} className="btn--secondary">
        Back to Home
      </button>
    </div>
  );
}

// ── App ─────────────────────────────────────────────────────────────────────

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
          Engram
        </h1>
      </header>
      <main>{renderScreen(path, handleNavigate)}</main>
    </div>
  );
}
