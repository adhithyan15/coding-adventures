/**
 * Lightweight i18n (internationalization) system.
 *
 * === Design ===
 *
 * All user-facing text in the application — UI chrome, educational content,
 * gate descriptions, hardware explanations — lives in JSON locale files.
 * No strings are hardcoded in components.
 *
 * The system is intentionally simple:
 *   - Flat dot-notation keys: "alu.title", "calculator.key.add"
 *   - One JSON file per language: en.json, ja.json, de.json, etc.
 *   - Fallback to English for missing keys
 *   - Language picker shown when 2+ locale files exist
 *
 * === Adding a new language ===
 *
 * 1. Copy `locales/en.json` to `locales/<code>.json` (e.g., `ja.json`)
 * 2. Translate all values (keys stay the same)
 * 3. The language picker appears automatically — no code changes needed
 *
 * === Why not i18next? ===
 *
 * For a single-page educational app with ~100 strings, a 3-function module
 * is simpler and smaller than pulling in i18next + react-i18next (~50KB).
 * If the app grows to need pluralization, interpolation, or ICU message
 * format, we can swap in a library later.
 */

import { useState, useCallback, useEffect, useMemo } from "react";

// Import English locale statically — it's always available as the fallback.
import enLocale from "./locales/en.json";

/** A locale is a flat map of dot-notation keys to translated strings. */
type LocaleMap = Record<string, string>;

/** Currently loaded locale data. English is always pre-loaded. */
const loadedLocales: Record<string, LocaleMap> = {
  en: enLocale as LocaleMap,
};

/** Current active locale code. */
let currentLocale = "en";

/** Listeners notified when the locale changes. */
const listeners = new Set<() => void>();

/**
 * Look up a translation key in the current locale.
 *
 * Falls back to English if the key is missing in the active locale.
 * If the key is missing in English too, returns the key itself
 * (so missing translations are obvious in the UI).
 *
 * @param key - Dot-notation key, e.g. "alu.title"
 * @returns The translated string, or the key if not found
 */
function translate(key: string): string {
  const locale = loadedLocales[currentLocale];
  if (locale && key in locale) {
    return locale[key]!;
  }
  // Fallback to English
  const en = loadedLocales["en"]!;
  if (key in en) {
    return en[key]!;
  }
  // Key not found anywhere — return the key itself so it's visible
  return key;
}

/**
 * Get the list of available locale codes.
 *
 * In the future, this will scan the locales directory dynamically.
 * For now, it returns whatever has been loaded.
 */
function getAvailableLocales(): string[] {
  return Object.keys(loadedLocales);
}

/**
 * Switch to a different locale.
 *
 * @param code - Locale code (e.g., "en", "ja")
 */
function setLocale(code: string): void {
  if (code !== currentLocale && code in loadedLocales) {
    currentLocale = code;
    // Notify all listeners (React components re-render)
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
