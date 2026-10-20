// Theme selection for the web host.
//
// Why this exists at all: mosstyle bakes colours into each emitted component's *inline*
// styles, so there is no CSS variable or class to flip at runtime. `mosaic-compile
// --theme <t>` emits a whole component per theme, and the host chooses between them —
// see `scripts/build-web.{sh,ps1}`, which emit `TaskApp.light.tsx` and `TaskApp.dark.tsx`.
//
// The rule, in priority order:
//   1. an explicit choice the user made here (persisted), else
//   2. the operating system's `prefers-color-scheme`, else
//   3. light.
//
// Only (1) is stored, and only when the user actually picks — so someone who never
// touches the toggle keeps following their OS, including when it flips at sunset.

import { MSL_TOKENS } from "./theme-tokens.generated";

/** The themes the build emits. Keep in step with `build-web`'s loop. */
export type Theme = "light" | "dark";

/** Where the explicit choice lives. Namespaced to avoid clashing with other apps. */
export const THEME_KEY = "task-app.theme";

const isTheme = (v: unknown): v is Theme => v === "light" || v === "dark";

/**
 * The OS preference, or `undefined` where it can't be read (older browsers, jsdom,
 * SSR). Deliberately not defaulted here so callers can tell "no opinion" from "light".
 */
export function systemTheme(): Theme | undefined {
  if (typeof globalThis.matchMedia !== "function") return undefined;
  try {
    // Ask for dark explicitly rather than negating light: a browser that doesn't know
    // the feature reports `matches: false` for BOTH queries, which would otherwise read
    // as a positive vote for light.
    if (globalThis.matchMedia("(prefers-color-scheme: dark)").matches) return "dark";
    if (globalThis.matchMedia("(prefers-color-scheme: light)").matches) return "light";
  } catch {
    // Privacy-hardened builds may throw on fingerprintable media queries rather than
    // answering. This feeds `useState`'s lazy initializer, so letting it escape would
    // blank the whole app — degrade to "no opinion" instead.
  }
  return undefined;
}

/** The user's explicit choice, if they have made one. */
export function storedTheme(): Theme | undefined {
  try {
    const raw = globalThis.localStorage?.getItem(THEME_KEY);
    return isTheme(raw) ? raw : undefined;
  } catch {
    // Private mode / disabled storage — fall through to the OS preference.
    return undefined;
  }
}

/** The theme to render right now, applying the priority order above. */
export function resolveTheme(): Theme {
  return storedTheme() ?? systemTheme() ?? "light";
}

/** Remember an explicit choice. A failed write is not worth breaking the toggle over. */
export function storeTheme(theme: Theme): void {
  try {
    globalThis.localStorage?.setItem(THEME_KEY, theme);
  } catch {
    /* ignore */
  }
}

/**
 * Call `onChange` when the OS preference flips — but only while the user has made no
 * explicit choice, since an explicit choice outranks the OS. Returns an unsubscribe
 * function, or a no-op where the API is unavailable.
 */
export function watchSystemTheme(onChange: (theme: Theme) => void): () => void {
  if (typeof globalThis.matchMedia !== "function") return () => {};
  let query: MediaQueryList;
  try {
    query = globalThis.matchMedia("(prefers-color-scheme: dark)");
  } catch {
    return () => {}; // same reasoning as `systemTheme`
  }
  const listener = (e: MediaQueryListEvent) => {
    if (storedTheme() === undefined) onChange(e.matches ? "dark" : "light");
  };
  // Safari <14 only has the deprecated addListener form.
  if (typeof query.addEventListener === "function") {
    query.addEventListener("change", listener);
    return () => query.removeEventListener("change", listener);
  }
  query.addListener(listener);
  return () => query.removeListener(listener);
}

/**
 * The page ground for a theme. `index.html` paints an OS-derived guess before React
 * mounts; once the real theme is resolved the app must repaint it, or an explicit
 * choice that disagrees with the OS leaves the wrong colour showing in the overscroll
 * area and behind the app.
 *
 * The ground is the `app-shell` background from TaskApp.{light,dark}.msl. The emitted
 * component inlines its styles and exposes no token to read back, so every stylesheet
 * colour the host needs comes from `MSL_TOKENS` in `theme-tokens.generated.ts`. A
 * cargo test in the task-app package writes that module from the stylesheets and fails
 * while it is stale, so nothing in this file spells a stylesheet colour by hand.
 */
export function applyThemeGround(theme: Theme): void {
  const root = globalThis.document?.documentElement;
  if (root) root.style.background = MSL_TOKENS[theme].ground;
}

/**
 * The four values the host's own startup chrome needs.
 *
 * Startup chrome is the loading and failure text shown *before* the Mosaic
 * component can exist. That component is purely presentational: it takes slot
 * values computed by the controller, and the controller needs a live engine. So
 * while the engine is still being fetched and compiled there is, by
 * construction, nothing authored to render — the host has to draw these two
 * states itself.
 *
 * That makes this the one deliberate exception to "never style the app outside
 * mosstyle" (see `index.html`'s comment, which makes the same point about CSS).
 * The exception is kept as small as possible: four values, read from `app-shell`
 * and `storage-warning` in TaskApp.{light,dark}.msl through the generated tokens,
 * so the startup states are visually continuous with the app that replaces them.
 */
export interface StartupChrome {
  ground: string;
  text: string;
  alert: string;
  fontFamily: string;
}

/** The startup chrome palette for a resolved theme. */
export function startupChrome(theme: Theme): StartupChrome {
  const t = MSL_TOKENS[theme];
  return { ground: t.ground, text: t.ink, alert: t.alert, fontFamily: t.fontFamily };
}

/**
 * The ring's `background` value for a percent-complete, 0..100, in the given theme.
 *
 * The progress ring (task-app-icon-assets-v1.md) fills with the honey accent
 * (`brand-mark`'s background) over the hairline track (`rail`'s border). `ring-fill`'s
 * background is a UI36-bound property, so the host computes the `conic-gradient(...)`
 * string itself and takes both colours from the generated tokens.
 */
export function ringGradient(theme: Theme, percent: number): string {
  const { accent, hairline } = MSL_TOKENS[theme];
  const deg = (Math.max(0, Math.min(100, Math.round(percent))) / 100) * 360;
  return `conic-gradient(${accent} ${deg}deg, ${hairline} ${deg}deg)`;
}

/**
 * Board's column accent bars (Board design-fidelity item, BACKLOG.md). Like the
 * ring, `col-bar`'s background is a UI36-bound property (it varies per `For`-loop
 * row, which is knowable from the status id alone but not statically by the
 * stylesheet), and `getProps()` builds `board-columns` without knowing the active
 * theme, so `Root` resolves the real hex per row.
 *
 * Three columns reuse established stylesheet colours through the generated tokens:
 * `next` is `group-head`'s muted ink, `doing` the honey accent, `done` `pill-ok`'s
 * on-track sage. `review` is the one colour the stylesheet has no part for, so it is
 * authored here, once: it is the host's own value, not a copy of anything.
 */
const REVIEW_ACCENT: Record<Theme, string> = {
  light: "#5b84ad",
  dark: "#7fa8d1",
};

function boardAccents(theme: Theme): Record<string, string> {
  const t = MSL_TOKENS[theme];
  return { next: t.muted, doing: t.accent, review: REVIEW_ACCENT[theme], done: t.ok };
}

const BOARD_ACCENT: Record<Theme, Record<string, string>> = {
  light: boardAccents("light"),
  dark: boardAccents("dark"),
};

/** The accent-bar `background` for a status id, in the given theme. */
export function boardAccent(theme: Theme, statusId: string): string {
  return BOARD_ACCENT[theme][statusId] ?? BOARD_ACCENT[theme].next;
}
