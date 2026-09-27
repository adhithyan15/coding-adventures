// HL41, rollout step 2: the "In the family, and next door" panel, as the app
// shows it.
//
// The book already prints this panel (human-language-data's
// `equivalentsPanelMarkdown`). The app reads the SAME owner files,
// `<track>/equivalents.d/<LESSON>.json`, so the two can never disagree about a
// word. This module is the pure half: it turns one owner file into rows the
// DOM shell can print with `textContent`. The loading half lives in
// `equivalents-sources.ts`, behind a dynamic import, so the owner files never
// reach the first-paint bundle.
//
// A panel for Tamil கண் (kaṇ, "eye") reads:
//
//   Kannada               ಕಣ್ಣು   kaṇṇu     same root
//   Telugu                కన్ను   kannu     same root
//   Malayalam             കണ്ണ്   kaṇṇŭ     same root
//   Hindi (neighbour)     आँख     ā̃kh
//   English               eye
//
// The validator in human-language-data already refuses a malformed owner file
// at commit time. This module checks the shape again anyway, because what
// arrives here is a JSON value, and the app must never render half a panel or
// throw while a lesson is opening. Anything unexpected makes the whole panel
// absent (null), never partial.

/** One row of the panel: a language, its word, and how that word is said. */
export interface EquivalentRow {
  /** The language's display name, with "(neighbour)" for the next-door language. */
  label: string;
  /** The word in its own script. Empty only on the English row. */
  form: string;
  /** How the word is said, in Latin letters. Empty when the form already is. */
  romanization: string;
  /** True only where the author judged the two words to share a root. */
  sameRoot: boolean;
}

export interface EquivalentsView {
  rows: EquivalentRow[];
  /** True when any row is marked, so the "same root" column is worth showing. */
  marked: boolean;
}

const isString = (value: unknown): value is string => typeof value === "string";

/**
 * Build the panel rows from one owner file.
 *
 * `neighbours` is the lesson's track's neighbour list from
 * `core/comparison-sets.json`, and `nameOf` gives a language's display name.
 * Returns null for anything that is not a well-formed owner file with at least
 * one equivalent. The English row always comes last, as it does in the book.
 */
export function equivalentsView(
  raw: unknown,
  neighbours: readonly string[],
  nameOf: (languageId: string) => string,
): EquivalentsView | null {
  if (typeof raw !== "object" || raw === null) return null;
  const entry = raw as Record<string, unknown>;
  if (!isString(entry.english) || entry.english.trim() === "") return null;
  if (!Array.isArray(entry.equivalents) || entry.equivalents.length === 0) return null;

  const rows: EquivalentRow[] = [];
  for (const item of entry.equivalents as unknown[]) {
    if (typeof item !== "object" || item === null) return null;
    const equivalent = item as Record<string, unknown>;
    if (!isString(equivalent.language) || !isString(equivalent.form) || equivalent.form.trim() === "") {
      return null;
    }
    if (equivalent.romanization !== undefined && !isString(equivalent.romanization)) return null;
    if (equivalent.sameRoot !== undefined && typeof equivalent.sameRoot !== "boolean") return null;
    const name = nameOf(equivalent.language);
    rows.push({
      label: neighbours.includes(equivalent.language) ? `${name} (neighbour)` : name,
      form: equivalent.form,
      romanization: equivalent.romanization ?? "",
      sameRoot: equivalent.sameRoot === true,
    });
  }
  rows.push({ label: "English", form: entry.english, romanization: "", sameRoot: false });
  return { rows, marked: rows.some((row) => row.sameRoot) };
}
