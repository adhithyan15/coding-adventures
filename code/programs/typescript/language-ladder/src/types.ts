// The shape of the SCRIPT data we consume. This mirrors the `ScriptData` /
// `Letter` types published by `@coding-adventures/human-language-data` (HL01),
// but is redeclared locally: the app reads `data/scripts/*.json` directly, and
// duplicating a handful of field names is cheaper than routing the JSON through
// the package.
//
// (The app does now depend on that package — see `lessons.ts`, which uses its
// pure `parseLesson` to read the ~679 lesson files. It deep-imports
// `.../src/parse.ts` rather than the barrel, which would drag `node:fs` and
// `process` into the browser bundle. Only the LESSON types come from the
// package; the script types below stay local.)

/** One base letter (or character/radical) of a script. */
export interface Letter {
  /** The citation form as written. */
  glyph: string;
  /** Romanization / phonetic value, e.g. "v (as in 'van')". */
  sound: string;
  /** consonant | vowel | letter | logograph | … */
  role: string;
  /** Tonal scripts (Mandarin): the canonical tone. */
  tone?: string;
  /** Abugida: the vowel a bare consonant carries. */
  inherentVowel?: string;
  /** Cursive/abjad positional forms (Arabic). */
  forms?: Record<string, string>;
  /** The literal "pieces" a learner draws — the heart of "break it apart". */
  components: string[];
  /** A conventional stroke order for paper practice. */
  strokeOrder: string[];
  strokeOrderNote: string;
  /** Free-text notes; may flag a "FALSE FRIEND" of a Latin letter. */
  notes?: string;
}

/** A whole script: its metadata plus its inventory of letters. */
export interface ScriptData {
  script: string;
  name: string;
  font: string;
  direction: "ltr" | "rtl";
  system: string; // alphabet | abugida | abjad | logographic | …
  /** The one visual feature that gives this script away at a glance — for a
   *  "spot the script" identification mode. Verified by rendering the font. */
  signature?: string;
  letters: Letter[];
  /** For an abugida: the independent (word-initial) vowels — the letters a word
   *  writes when it BEGINS with a vowel (అ a, ఆ ā), as opposed to the vowel signs
   *  that ride on a consonant. Kept separate from `letters` so the consonant
   *  syllabary (and anything keyed on it being all-syllables) is untouched. */
  independentVowels?: Letter[];
  marks?: unknown[];
  combination?: string;
  complete?: boolean;
  notes?: string;
}
