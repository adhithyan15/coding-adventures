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
