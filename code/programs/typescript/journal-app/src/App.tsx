/**
 * App.tsx — Root component with hash-based routing.
 *
 * Hash routing (#/entry/new, #/entry/:id, etc.) is used instead of
 * HTML5 history because:
 *   - Works with file:// protocol in Electron (no web server needed)
 *   - Works on GitHub Pages without server-side redirect rules
 *   - Simple to implement — just listen to hashchange events
 *
 * === Routes ===
 *
 *   #/                  → Timeline (home, entries grouped by date)
 *   #/entry/new         → New Entry (split-pane editor)
 *   #/entry/:id         → View Entry (rendered markdown, read-only)
 *   #/entry/:id/edit    → Edit Entry (split-pane editor, pre-populated)
 */

import { useState, useEffect } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import { Timeline } from "./components/Timeline.js";
import { EntryEditor } from "./components/EntryEditor.js";
import { EntryView } from "./components/EntryView.js";

/**
 * navigate — programmatic hash navigation.
 *
 * Setting window.location.hash triggers a hashchange event, which the
 * App component listens to and re-renders with the new route.
 */
export function navigate(path: string): void {
  window.location.hash = path;
}

/**
 * parseHash — extract the current route from window.location.hash.
 *
 * Strips the leading "#" and normalises empty hash to "/".
 */
function parseHash(): string {
  const hash = window.location.hash.slice(1);
  return hash || "/";
}

export function App(): JSX.Element {
  const [route, setRoute] = useState(parseHash);
  const t = useTranslation();

  useEffect(() => {
    function onHashChange() {
      setRoute(parseHash());
    }
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  // ── Route matching ────────────────────────────────────────────────────
  //
  // Match routes in order of specificity (most specific first).

  // #/entry/new
  if (route === "/entry/new") {
    return <EntryEditor />;
  }

  // #/entry/:id/edit
  const editMatch = route.match(/^\/entry\/([^/]+)\/edit$/);
  if (editMatch) {
    return <EntryEditor entryId={editMatch[1]} />;
  }

  // #/entry/:id
  const viewMatch = route.match(/^\/entry\/([^/]+)$/);
  if (viewMatch) {
    return <EntryView entryId={viewMatch[1]!} />;
  }

  // #/ (default: timeline)
  return <Timeline />;
}
