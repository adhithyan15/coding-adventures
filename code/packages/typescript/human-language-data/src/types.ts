// Data model for the Human Languages data layer (spec: code/specs/HL01-*).
//
// The whole point of this package is to turn the curriculum's Markdown lessons
// into something a program can reason about *across* languages. The unit of that
// reasoning is the CONCEPT — a language-independent idea ("the greeting you say
// on meeting someone") — and each language's word for it is a REALIZATION. These
// types are that model.

/** A writing system. Every track uses exactly one; `latin` needs no script data. */
export type Script =
  | "latin"
  | "devanagari"
  | "bengali"
  | "gurmukhi"
  | "tamil"
  | "telugu"
  | "kannada"
  | "malayalam"
  | "arabic";

/** Grammatical gender tag on a noun. `null` when the language/word carries none. */
export type Gender = "masc" | "fem" | "neut" | null;

/** One entry in `concepts/taxonomy.json` — a canonical universal concept. */
export interface TaxonomyConcept {
  family: string;
  gloss: string;
  /** Expected in every track once it declares parity. */
  core: boolean;
  /** Pre-canonical tags this id replaced, for traceability. */
  retires?: string[];
  notes?: string;
}

export interface Taxonomy {
  version: number;
  concepts: Record<string, TaxonomyConcept>;
}

/**
 * One language's realization of a concept — the row the app turns into a card.
 * Derived from a single lesson's frontmatter (plus a couple of best-effort
 * heuristics where the field isn't authored yet).
 */
export interface Realization {
  concept: string; // canonical or namespaced concept id
  language: string; // track slug: "spanish", "telugu", …
  lessonId: string; // e.g. ES-C01-dia
  chapter: number;
  type: string; // word | phrase | practice | practice-mix | review
  headword: string; // "día" / "నమస్కారం"
  gloss: string; // "day (el día — masculine)"
  romanization: string; // "DEE-ah"; = headword for Latin script; "" if unknown
  script: Script;
  gender: Gender;
  sounds: string[]; // ids into pronunciation-reference.md
  roots: string[]; // etymological roots, for indexing
  etymologyHook: string; // ≤120-char memory anchor; "" if not authored yet
}

/** A concept plus every language that realizes it — the cross-language join. */
export interface Concept {
  id: string;
  family: string;
  gloss: string;
  core: boolean;
  /** True when language-local (namespaced id, not in the taxonomy). */
  namespaced: boolean;
  realizations: Realization[];
}

/** The whole parsed curriculum, ready for querying. */
export interface Dataset {
  taxonomy: Taxonomy;
  concepts: Concept[];
  byLanguage: Record<string, Realization[]>;
  languages: string[];
}

// ---- Script / character-breakdown data (data/scripts/<script>.json) ----

export interface Glyph {
  glyph: string;
  sound: string;
  type: "consonant" | "vowel" | "sign" | "conjunct" | "other";
  inherentVowel?: string;
  /** The literal "pieces" of the character, for paper practice. */
  components: string[];
  /** Typical handwriting order — conventional, not canonical. */
  strokeOrder: string[];
  strokeOrderNote: string;
  notes?: string;
}

export interface VowelSign {
  sign: string;
  sound: string;
  attachesAs: string;
  example?: { base: string; combined: string; sound: string };
}

export interface ScriptData {
  script: Script;
  font: string;
  abugida: boolean;
  glyphs: Glyph[];
  vowelSigns: VowelSign[];
  conjunctRule?: string;
}

// ---- Validation ----

export interface Issue {
  level: "error" | "warning" | "info";
  code: string;
  message: string;
  lessonId?: string;
}
