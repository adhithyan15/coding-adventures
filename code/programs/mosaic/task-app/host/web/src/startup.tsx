// Startup chrome: the two states the host draws around engine initialization.
//
// startApp() cannot render anything until the engine is live: the Mosaic
// component is presentational, its slots come from the controller, and the
// controller needs the engine. Before #13695 that whole wait happened with #root
// empty, and the boot promise was floated — so a failed fetch, a failed compile,
// or a browser without WebAssembly left the page blank forever, with the error
// visible only in the console.
//
// These live in their own module rather than in main.tsx because main.tsx is
// exempt from the coverage gate as DOM boot glue (see vitest.config.ts). Startup
// states are not glue — they are the user-visible half of the fix — so they get
// a real seam and real tests.
//
// They are deliberately plain: no engine, no controller, and no persisted state
// exists yet, so anything richer would need one of them.
import { type ReactNode } from "react";

import { startupChrome, type Theme } from "./theme";

/** Shared frame for both startup states: centred, themed, screen-reader live. */
function StartupFrame({
  theme,
  role,
  children,
}: {
  theme: Theme;
  role: "status" | "alert";
  children: ReactNode;
}) {
  const chrome = startupChrome(theme);
  return (
    <div
      role={role}
      // Assistive tech should hear the whole state when it swaps in, not the
      // individual words as React commits them.
      aria-live={role === "alert" ? "assertive" : "polite"}
      aria-atomic="true"
      style={{
        background: chrome.ground,
        color: chrome.text,
        fontFamily: chrome.fontFamily,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 12,
        padding: 24,
        textAlign: "center",
        boxSizing: "border-box",
      }}
    >
      {children}
    </div>
  );
}

/** Shown while the engine and the saved workspace are still initializing. */
export function StartupLoading({ theme }: { theme: Theme }) {
  return (
    <StartupFrame theme={theme} role="status">
      <p style={{ margin: 0, fontSize: 15 }}>Starting Trestle…</p>
    </StartupFrame>
  );
}

/**
 * Shown when initialization fails. Retry is offered because every failure this
 * can reach is plausibly transient — an interrupted download, a cold cache, a
 * flaky network. The detail line is kept verbatim so a report is actionable;
 * it is rendered as text, never as markup.
 */
export function StartupFailure({
  theme,
  detail,
  onRetry,
}: {
  theme: Theme;
  detail: string;
  onRetry: () => void;
}) {
  const chrome = startupChrome(theme);
  return (
    <StartupFrame theme={theme} role="alert">
      <p style={{ margin: 0, fontSize: 15, fontWeight: "bold", color: chrome.alert }}>
        Trestle could not start.
      </p>
      <p style={{ margin: 0, fontSize: 13, maxWidth: 420 }}>
        Your saved tasks have not been changed. Retrying is safe.
      </p>
      <button
        type="button"
        onClick={onRetry}
        autoFocus
        style={{
          font: "inherit",
          fontSize: 13,
          padding: "7px 16px",
          borderRadius: 8,
          border: `1px solid ${chrome.text}`,
          background: "transparent",
          color: chrome.text,
          cursor: "pointer",
        }}
      >
        Try again
      </button>
      <p style={{ margin: 0, fontSize: 12, maxWidth: 420, opacity: 0.75 }}>{detail}</p>
    </StartupFrame>
  );
}
