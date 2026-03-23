/**
 * Tests for the i18n (internationalization) system.
 *
 * The i18n module provides a lightweight translation layer:
 *
 *   - initI18n() loads locale data (flat key-value maps)
 *   - translate() looks up keys, falling back to English, then to the key itself
 *   - setLocale() switches the active language
 *   - getAvailableLocales() lists what's loaded
 *
 * These are pure function tests — no React rendering needed.
 * The useTranslation hook is tested separately since it requires React.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  initI18n,
  translate,
  setLocale,
  getAvailableLocales,
} from "./index.js";

/* ── Test locale data ─────────────────────────────────────────────── */

const enLocale = {
  "app.title": "My App",
  "app.greeting": "Hello",
  "shared.ok": "OK",
};

const frLocale = {
  "app.title": "Mon App",
  "app.greeting": "Bonjour",
  // "shared.ok" intentionally missing — should fall back to English
};

const jaLocale = {
  "app.title": "My App (ja)",
};

/* ── Tests ────────────────────────────────────────────────────────── */

describe("i18n", () => {
  /**
   * Each test starts with a fresh i18n state to avoid cross-contamination.
   * initI18n clears previously loaded locales before loading new ones.
   */
  beforeEach(() => {
    initI18n({ en: enLocale, fr: frLocale });
  });

  /* ── initI18n ──────────────────────────────────────────────────── */

  it("loads locales so they are available", () => {
    const locales = getAvailableLocales();
    expect(locales).toContain("en");
    expect(locales).toContain("fr");
  });

  it("multiple initI18n calls reset state properly", () => {
    /**
     * After re-initializing with only Japanese and English,
     * French should no longer be available.
     */
    initI18n({ en: enLocale, ja: jaLocale });
    const locales = getAvailableLocales();
    expect(locales).toContain("en");
    expect(locales).toContain("ja");
    expect(locales).not.toContain("fr");
  });

  /* ── translate ─────────────────────────────────────────────────── */

  it("returns the correct value for a known key", () => {
    expect(translate("app.title")).toBe("My App");
  });

  it("returns the value from the active locale", () => {
    setLocale("fr");
    expect(translate("app.title")).toBe("Mon App");
  });

  it("falls back to English for a key missing in the active locale", () => {
    /**
     * "shared.ok" exists in English but not in French.
     * When French is active, translate should fall back to "OK".
     */
    setLocale("fr");
    expect(translate("shared.ok")).toBe("OK");
  });

  it("returns the key itself when missing in all locales", () => {
    /**
     * If a key exists in neither the active locale nor English,
     * the raw key is returned. This makes missing translations
     * visually obvious in the UI.
     */
    expect(translate("nonexistent.key")).toBe("nonexistent.key");
  });

  /* ── setLocale ─────────────────────────────────────────────────── */

  it("changes the active locale", () => {
    setLocale("fr");
    expect(translate("app.greeting")).toBe("Bonjour");
  });

  it("does not change locale when given an unknown code", () => {
    /**
     * Attempting to switch to a locale that hasn't been loaded
     * should be a no-op — the current locale stays the same.
     */
    setLocale("de");
    expect(translate("app.title")).toBe("My App");
  });

  /* ── getAvailableLocales ───────────────────────────────────────── */

  it("returns all loaded locale codes", () => {
    const locales = getAvailableLocales();
    expect(locales).toEqual(["en", "fr"]);
  });

  it("reflects changes after re-initialization", () => {
    initI18n({ en: enLocale, ja: jaLocale });
    expect(getAvailableLocales()).toEqual(["en", "ja"]);
  });
});
