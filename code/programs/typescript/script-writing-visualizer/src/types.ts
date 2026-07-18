// The shape of the script data we consume. This mirrors the `ScriptData` /
// `Letter` types published by `@coding-adventures/human-language-data` (HL01),
// but is redeclared locally so this MVP app carries no package dependency — it
// reads the JSON files directly. If the two ever need to share a type, promote
// this to an import; for now, duplication of a handful of field names is the
// cheaper trade than a cross-package build edge.

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
  letters: Letter[];
  marks?: unknown[];
  combination?: string;
  complete?: boolean;
  notes?: string;
}
