/**
 * Lightweight i18n (internationalization) system.
 *
 * === Design ===
 *
 * All user-facing text lives in JSON locale files. No strings are
 * hardcoded in components. The system is intentionally simple:
 *
 *   - Flat dot-notation keys: "era1.title", "tabs.vacuumTube"
 *   - One JSON file per language: en.json, ja.json, de.json, etc.
 *   - Fallback to English for missing keys
 *   - Language picker shown when 2+ locale files exist
 *
 * === Setup ===
 *
 * Each app provides its own locale data at startup:
 *
 * ```typescript
 * import { initI18n } from "@coding-adventures/ui-components";
 * import en from "./i18n/locales/en.json";
 *
 * initI18n({ en });
 * ```
 *
 * === Adding a new language ===
 *
 * 1. Copy `en.json` to `ja.json` (for example)
 * 2. Translate all values (keys stay the same)
 * 3. Import and pass to initI18n: `initI18n({ en, ja })`
 * 4. The language picker appears automatically — minimal code changes
 */

import { useState, useCallback, useEffect, useMemo } from "react";

/** A locale is a flat map of dot-notation keys to translated strings. */
export type LocaleMap = Record<string, string>;

/** Currently loaded locale data. */
const loadedLocales: Record<string, LocaleMap> = {};

/** Current active locale code. */
let currentLocale = "en";

/** Listeners notified when the locale changes. */
const listeners = new Set<() => void>();

/**
 * Initialize the i18n system with locale data.
 *
 * Call this once at app startup before rendering any components.
 * English ("en") should always be included as it serves as the fallback.
 *
 * @param locales - Map of locale code to locale data
 * @param defaultLocale - Initial locale (defaults to "en")
 */
export function initI18n(
  locales: Record<string, LocaleMap>,
  defaultLocale = "en",
): void {
  // Clear and reload
  for (const key of Object.keys(loadedLocales)) {
    delete loadedLocales[key];
  }
  for (const [code, data] of Object.entries(locales)) {
    loadedLocales[code] = data;
  }
  currentLocale = defaultLocale;
}

/**
 * Look up a translation key in the current locale.
 *
 * Falls back to English if the key is missing in the active locale.
 * If the key is missing in English too, returns the key itself
 * (so missing translations are obvious in the UI).
 */
export function translate(key: string): string {
  const locale = loadedLocales[currentLocale];
  if (locale && key in locale) {
    return locale[key]!;
  }
  // Fallback to English
  const en = loadedLocales["en"];
  if (en && key in en) {
    return en[key]!;
  }
  // Key not found — return the key itself so it's visible
  return key;
}

/**
 * Get the list of available locale codes.
 */
export function getAvailableLocales(): string[] {
  return Object.keys(loadedLocales);
}

/**
 * Switch to a different locale.
 */
export function setLocale(code: string): void {
  if (code !== currentLocale && code in loadedLocales) {
    currentLocale = code;
    for (const listener of listeners) {
      listener();
    }
  }
}

/**
 * React hook for translations.
 *
 * Returns:
 *   - t(key): translate a key
 *   - locale: current locale code
 *   - setLocale: switch locale
 *   - availableLocales: list of loaded locale codes
 *
 * Components using this hook automatically re-render when the locale changes.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { t } = useTranslation();
 *   return <h1>{t("app.title")}</h1>;
 * }
 * ```
 */
export function useTranslation() {
  const [, forceUpdate] = useState(0);

  useEffect(() => {
    const listener = () => forceUpdate((n) => n + 1);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);

  const t = useCallback((key: string) => translate(key), []);

  return useMemo(
    () => ({
      t,
      locale: currentLocale,
      setLocale,
      availableLocales: getAvailableLocales(),
    }),
    [t],
  );
}
