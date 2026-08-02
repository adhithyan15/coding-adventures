// Data model for the Human Languages data layer (spec: code/specs/HL01-*).
//
// The whole point of this package is to turn the curriculum's Markdown lessons
// into something a program can reason about *across* languages. The unit of that
// reasoning is the CONCEPT — a language-independent idea ("the greeting you say
// on meeting someone") — and each language's word for it is a REALIZATION. These
// types are that model.

/**
 * A writing-system identifier. **Open by design** — this is a plain string, not a
 * closed union, so teaching a new script (Gujarati, Hebrew, Greek, …) never
 * requires a code change: you drop in a `data/scripts/<script>.json` and point a
 * track at it. Values in use today: `latin`, `devanagari`, `bengali`, `gurmukhi`,
 * `tamil`, `telugu`, `kannada`, `malayalam`, `arabic`.
 */
export type Script = string;

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
  type: string; // word | phrase | practice | practice-mix | review | writing
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

// ---- Shared curriculum spine (HL04) ---------------------------------------

export type CurriculumStage = "pre-A1" | "A1" | "A2" | "B1" | string;

export interface LanguageDefinition {
  id: string;
  name: string;
  family: string;
  script: Script;
  status: "active" | "planned" | string;
  /** Languages whose history, vocabulary, or structure gives this track a bridge. */
  bridges: string[];
}

export interface LanguageRegistry {
  version: number;
  /** Authored order used as the default cross-language walk. */
  languages: LanguageDefinition[];
}

export interface SpineNode {
  id: string;
  stage: CurriculumStage;
  canDo: string;
  prerequisites: string[];
  core: boolean;
  /** Canonical concept ids taught inside this communicative ability. */
  concepts: string[];
}

export interface CurriculumSpine {
  version: number;
  stages: CurriculumStage[];
  nodes: SpineNode[];
}

/** One authored chapter from an existing LaTeX book. */
export interface BookChapter {
  language: string;
  chapter: number;
  slug: string;
  title: string;
  /** Curriculum-root-relative path, suitable for provenance and diagnostics. */
  source: string;
  /** Original LaTeX. Kept losslessly so future renderers do not scrape PDFs. */
  tex: string;
}

/** The authored LaTeX layer that surrounds and sequences the short lessons. */
export interface LanguageBook {
  language: string;
  entrypoint: string;
  chapters: BookChapter[];
}

export interface BookCorpus {
  books: LanguageBook[];
}

// ---- Typed lesson body (HL04 schema v2) ----------------------------------

export type LessonBlockType =
  | "warmup"
  | "input"
  | "notice"
  | "pronunciation"
  | "script"
  | "etymology"
  | "grammar"
  | "culture-pragmatics"
  | "guided-production"
  | "comprehension"
  | "fluency"
  | "recall"
  | "unknown";

export interface LessonBodyBlock {
  type: LessonBlockType;
  /** Original level-two heading, kept for book/app presentation. */
  title: string;
  /** Lossless Markdown between this heading and the next typed block. */
  markdown: string;
}

// ---- Script / character-breakdown data (data/scripts/<script>.json) ----
//
// Deliberately GENERAL, so one schema can teach any writing system. It has to
// describe three structurally different families without special-casing them:
//
//   • alphabet  (latin, greek, cyrillic)   — letters spell vowels + consonants.
//   • abugida   (devanagari, telugu, …)     — consonants carry an inherent vowel;
//                                             a vowel *mark* changes it; consonants
//                                             stack into conjuncts.
//   • abjad     (arabic, hebrew)            — consonants only; vowels are optional
//                                             diacritic *marks*; written right-to-
//                                             left; letters take contextual FORMS.
//
// The shared vocabulary: a script has DIRECTION, a SYSTEM, a set of LETTERS (each
// optionally with positional FORMS), and a set of MARKS (vowel signs OR harakat/
// niqqud). Every letter/mark carries the component decomposition + typical stroke
// order that is the whole point — "learn to write it, piece by piece."
//
// A fourth family, LOGOGRAPHIC (Chinese Hanzi), fits the same model without any
// special-casing: a "letter" is a character or a radical, its `components` are the
// radicals/strokes it's built from, `strokeOrder` is well-defined (and for Chinese
// actually authoritative), and its `sound` carries tone-marked pinyin (with an
// optional structured `tone`). No forms, no marks — those fields are simply omitted.

/** Which way the script runs. Abjads (Arabic, Hebrew) are `rtl`. */
export type Direction = "ltr" | "rtl";

/**
 * The structural family. Open string — the union members are hints, not a limit,
 * so a system we haven't named yet still validates.
 */
export type WritingSystem =
  | "alphabet"
  | "abugida"
  | "abjad"
  | "logographic"
  | "syllabary"
  | string;

/** A letter's role within its system. Open string (logographs, radicals, …). */
export type LetterRole =
  | "consonant"
  | "vowel"
  | "independent-vowel" // abugida word-initial vowels
  | "letter" // alphabet letters with no vowel/consonant split needed
  | "logograph" // a whole-word/morpheme character (Chinese)
  | "radical" // a recurring component of logographs
  | "other"
  | string;

/** Contextual letter forms, for cursive/abjad scripts (Arabic). */
export interface LetterForms {
  isolated?: string;
  initial?: string;
  medial?: string;
  final?: string;
}

/** One base letter of the script (a letter, a character, or a radical). */
export interface Letter {
  glyph: string; // the citation form
  sound: string; // romanization / phonetic value (tone-marked pinyin for Chinese)
  role: LetterRole;
  /** Tonal languages (Mandarin, Vietnamese, …): the canonical tone, e.g. "1".."4" | "neutral". */
  tone?: string;
  /** Abugida: the vowel a bare consonant carries (e.g. Devanagari "a"). */
  inherentVowel?: string;
  /** Cursive/abjad scripts: how the letter looks by position in a word. */
  forms?: LetterForms;
  /** The literal "pieces" of the character, for paper practice. */
  components: string[];
  /** Typical handwriting order — conventional, not canonical. */
  strokeOrder: string[];
  strokeOrderNote: string;
  notes?: string;
}

/**
 * A vowel sign / diacritic that attaches to a letter — an abugida mātrā, or the
 * harakat/niqqud of an abjad. `null`-free: alphabets simply have no marks.
 */
export interface Mark {
  mark: string;
  sound: string;
  role: "vowel-sign" | "diacritic" | "nasal" | "virama" | "other";
  attachesAs: string;
  example?: { base: string; combined: string; sound: string };
}

export interface ScriptData {
  script: Script; // the id, matches the filename
  name: string; // human name, e.g. "Devanagari"
  font: string; // vendored Noto path
  direction: Direction;
  system: WritingSystem;
  letters: Letter[];
  marks?: Mark[];
  /** How letters combine: abugida conjuncts, Arabic joining, ligatures, etc. */
  combination?: string;
  /** Set true when the inventory is complete enough to enforce glyph coverage. */
  complete?: boolean;
  notes?: string;
}

// ---- Validation ----

export interface Issue {
  level: "error" | "warning" | "info";
  code: string;
  message: string;
  lessonId?: string;
}
