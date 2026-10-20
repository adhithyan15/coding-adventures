/**
 * theme.test.ts — the theme-selection rules.
 *
 * The priority order (explicit choice > OS preference > light) is the whole contract,
 * and each rung has a failure mode worth pinning: a stored value must beat the OS, a
 * browser with no `matchMedia` must not crash, and a browser that reports `false` for
 * *both* media queries (i.e. doesn't understand the feature) must not be mistaken for
 * a positive vote for light.
 */
import { describe, it, expect, afterEach, vi } from "vitest";
import {
  applyThemeGround,
  boardAccent,
  resolveTheme,
  ringGradient,
  startupChrome,
  storeTheme,
  storedTheme,
  systemTheme,
  watchSystemTheme,
  THEME_KEY,
} from "../src/theme";
import { MSL_TOKENS } from "../src/theme-tokens.generated";

/** Install a `matchMedia` that answers `dark` / `light` as told. */
function fakeMatchMedia(answers: Record<string, boolean>) {
  const listeners: Array<(e: any) => void> = [];
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: answers[query] ?? false,
    addEventListener: (_: string, fn: (e: any) => void) => listeners.push(fn),
    removeEventListener: (_: string, fn: (e: any) => void) => {
      const i = listeners.indexOf(fn);
      if (i >= 0) listeners.splice(i, 1);
    },
  }));
  return { fire: (matches: boolean) => listeners.forEach((fn) => fn({ matches })) };
}

afterEach(() => {
  vi.unstubAllGlobals();
  try {
    globalThis.localStorage?.removeItem(THEME_KEY);
  } catch {
    /* ignore */
  }
});

describe("theme selection", () => {
  it("follows the OS when the user hasn't chosen", () => {
    fakeMatchMedia({ "(prefers-color-scheme: dark)": true });
    expect(systemTheme()).toBe("dark");
    expect(resolveTheme()).toBe("dark");
  });

  it("lets an explicit choice outrank the OS", () => {
    fakeMatchMedia({ "(prefers-color-scheme: dark)": true });
    storeTheme("light");
    expect(storedTheme()).toBe("light");
    expect(resolveTheme()).toBe("light");
  });

  it("treats 'neither query matches' as no opinion, not as light", () => {
    // A browser that doesn't understand the feature reports false for BOTH queries.
    // Reading that as a vote for light would ignore a genuine OS dark preference on
    // the next browser up.
    fakeMatchMedia({});
    expect(systemTheme()).toBeUndefined();
    expect(resolveTheme()).toBe("light"); // the documented final fallback
  });

  it("survives a browser with no matchMedia at all", () => {
    vi.stubGlobal("matchMedia", undefined);
    expect(systemTheme()).toBeUndefined();
    expect(() => resolveTheme()).not.toThrow();
    expect(watchSystemTheme(() => {})).toBeTypeOf("function"); // a no-op unsubscribe
  });

  it("ignores a corrupted stored value", () => {
    fakeMatchMedia({ "(prefers-color-scheme: dark)": true });
    globalThis.localStorage?.setItem(THEME_KEY, "chartreuse");
    expect(storedTheme()).toBeUndefined();
    expect(resolveTheme()).toBe("dark"); // falls through to the OS
  });

  it("reports OS flips only while the user hasn't overridden", () => {
    const media = fakeMatchMedia({ "(prefers-color-scheme: dark)": false });
    const seen: string[] = [];
    const stop = watchSystemTheme((t) => seen.push(t));

    media.fire(true);
    expect(seen).toEqual(["dark"]);

    // Once the user picks, the OS must stop steering.
    storeTheme("light");
    media.fire(false);
    expect(seen).toEqual(["dark"]);

    stop();
  });

  it("degrades when reading storage throws", () => {
    // Some browsers throw on localStorage access in private mode rather than
    // returning null; that must fall through to the OS, not crash.
    fakeMatchMedia({ "(prefers-color-scheme: dark)": true });
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {
        throw new Error("SecurityError");
      },
      removeItem: () => {},
    });
    expect(storedTheme()).toBeUndefined();
    expect(resolveTheme()).toBe("dark");
    expect(() => storeTheme("light")).not.toThrow(); // a failed persist isn't fatal
  });

  it("degrades when matchMedia itself throws", () => {
    // Privacy-hardened builds may throw on a fingerprintable media query instead of
    // answering. This feeds useState's lazy initializer, so an escape would blank the
    // whole app.
    vi.stubGlobal("matchMedia", () => {
      throw new Error("blocked");
    });
    expect(systemTheme()).toBeUndefined();
    expect(resolveTheme()).toBe("light");
    expect(() => watchSystemTheme(() => {})).not.toThrow();
    expect(watchSystemTheme(() => {})).toBeTypeOf("function");
  });

  it("uses the legacy addListener API when addEventListener is absent", () => {
    // Safari <14 only has the deprecated form; the unsubscribe must match it.
    const listeners: Array<(e: any) => void> = [];
    let removed = 0;
    vi.stubGlobal("matchMedia", () => ({
      matches: false,
      addListener: (fn: (e: any) => void) => listeners.push(fn),
      removeListener: () => {
        removed += 1;
      },
    }));
    const seen: string[] = [];
    const stop = watchSystemTheme((t) => seen.push(t));
    expect(listeners).toHaveLength(1);

    listeners[0]({ matches: true });
    expect(seen).toEqual(["dark"]);

    stop();
    expect(removed).toBe(1);
  });

  it("unsubscribes cleanly", () => {
    const media = fakeMatchMedia({ "(prefers-color-scheme: dark)": false });
    const seen: string[] = [];
    watchSystemTheme((t) => seen.push(t))();
    media.fire(true);
    expect(seen).toEqual([]);
  });
});

// The expected colours are read from the generated tokens, not spelled here: the
// tokens are the stylesheet's own values (a cargo test regenerates and checks them),
// so these tests pin *which* stylesheet colour each host surface uses, and keep
// passing when a designer re-tints the .msl.
describe("ringGradient", () => {
  it("fills proportionally to the percent, clamped to 0..100", () => {
    const { accent, hairline } = MSL_TOKENS.light;
    expect(ringGradient("light", 0)).toBe(`conic-gradient(${accent} 0deg, ${hairline} 0deg)`);
    expect(ringGradient("light", 50)).toBe(`conic-gradient(${accent} 180deg, ${hairline} 180deg)`);
    expect(ringGradient("light", 100)).toBe(`conic-gradient(${accent} 360deg, ${hairline} 360deg)`);
    expect(ringGradient("light", 150)).toBe(ringGradient("light", 100));
    expect(ringGradient("light", -10)).toBe(ringGradient("light", 0));
  });

  it("uses the dark theme's own fill/track colours", () => {
    const { accent, hairline } = MSL_TOKENS.dark;
    expect(ringGradient("dark", 25)).toBe(`conic-gradient(${accent} 90deg, ${hairline} 90deg)`);
    expect(ringGradient("dark", 25)).not.toBe(ringGradient("light", 25));
  });
});

describe("startupChrome and applyThemeGround", () => {
  it("draw the startup states in app-shell's ground, ink and font, alerting in storage-warning's red", () => {
    for (const theme of ["light", "dark"] as const) {
      const t = MSL_TOKENS[theme];
      expect(startupChrome(theme)).toEqual({
        ground: t.ground,
        text: t.ink,
        alert: t.alert,
        fontFamily: t.fontFamily,
      });
      applyThemeGround(theme);
      // jsdom normalises a hex colour to rgb(), so compare through the same parser.
      const probe = document.createElement("div");
      probe.style.background = t.ground;
      expect(document.documentElement.style.background).toBe(probe.style.background);
    }
  });
});

describe("boardAccent", () => {
  it("reuses the stylesheet's muted, accent and on-track colours for next/doing/done", () => {
    for (const theme of ["light", "dark"] as const) {
      const t = MSL_TOKENS[theme];
      expect(boardAccent(theme, "next")).toBe(t.muted);
      expect(boardAccent(theme, "doing")).toBe(t.accent);
      expect(boardAccent(theme, "done")).toBe(t.ok);
    }
  });

  it("gives review a colour of its own, distinct from the other three, per theme", () => {
    for (const theme of ["light", "dark"] as const) {
      const review = boardAccent(theme, "review");
      expect(review).toMatch(/^#[0-9a-f]{6}$/);
      for (const other of ["next", "doing", "done"]) {
        expect(review).not.toBe(boardAccent(theme, other));
      }
    }
    expect(boardAccent("dark", "review")).not.toBe(boardAccent("light", "review"));
  });

  it("falls back to the 'next' colour for an unrecognised status id", () => {
    expect(boardAccent("light", "bogus")).toBe(boardAccent("light", "next"));
  });
});
