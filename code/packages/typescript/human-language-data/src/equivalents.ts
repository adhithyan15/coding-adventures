// HL41 -- family and neighbour equivalents.
//
// A word sticks when it hooks onto something the reader already holds. For a
// Tamil word those hooks are its sisters (Kannada, Telugu, Malayalam), one big
// neighbour (Hindi) and English. Some of those share the Tamil word's root and
// some do not, and seeing which is which is itself the lesson:
//
//     போ  pō   "go"
//       Kannada    ಹೋಗು   hōgu     same root (an old p softened to h)
//       Telugu     వెళ్ళు  veḷḷu
//       Malayalam  പോകുക  pōkuka   same root
//       Hindi      जाना   jānā
//       English    go
//
// This module owns that panel end to end:
//
//   comparison sets   which languages a track compares against (core/)
//   owner files       one `<track>/equivalents.d/<LESSON-ID>.json` per lesson
//   validation        every form in its own language's script, every language
//                     in the track's set, every lesson real and a word lesson
//   the book panel    a book-only copy of the lesson with the table appended
//                     to its first teaching block
//
// WHY DATA AND NOT LESSON PROSE. A panel is reference material the reader
// glances at. Written into the lesson it would count toward the lesson's
// measured duration (and many lessons sit near the 300s budget), be read aloud
// by narration, and put other scripts' letters into the lesson's own teaching
// text. As data it does none of those: only the book's view of the lesson
// carries it, exactly as the stroke-order filmstrips are added (HL-C443).

import { existsSync } from "node:fs";
import { join } from "node:path";
import type { ParsedLesson } from "./parse.js";
import { readLedgerFile, readShards } from "./shard.js";
import type { Issue, LanguageRegistry } from "./types.js";

/** One sister or neighbour word. */
export interface Equivalent {
  /** A language id from the registry, e.g. `kannada`. */
  language: string;
  /** The word in its own script. One form, never a paradigm. */
  form: string;
  /** Required unless the language is written in Latin script. */
  romanization?: string;
  /**
   * `true`: cognate with the headword, or one borrowed the other.
   * `false`: a reviewer judged them unrelated.
   * absent: nobody has judged the pair yet. The panel marks only `true`, so an
   * unjudged pair is never presented as related.
   */
  sameRoot?: boolean;
}

/** The owner file for one lesson's panel. */
export interface LessonEquivalents {
  lesson: string;
  /** The English word, shown last. */
  english: string;
  equivalents: Equivalent[];
  /** Where the data came from: matched lesson ids, or `hand-filled`. */
  source: string;
}

/** A track's comparison languages, in display order. */
export interface ComparisonSet {
  family: string[];
  neighbours: string[];
}

export interface ComparisonSets {
  version: number;
  sets: Record<string, ComparisonSet>;
}

export const COMPARISON_SETS_PATH = join("core", "comparison-sets.json");

/**
 * The Unicode scripts each registry script id is written in.
 *
 * A form must use only letters of its language's scripts. Japanese is the one
 * track written in three scripts at once.
 */
export const UNICODE_SCRIPTS_BY_REGISTRY_SCRIPT: Readonly<Record<string, readonly string[]>> = {
  latin: ["Latin"],
  arabic: ["Arabic"],
  "perso-arabic": ["Arabic"],
  "urdu-nastaliq": ["Arabic"],
  devanagari: ["Devanagari"],
  tamil: ["Tamil"],
  kannada: ["Kannada"],
  telugu: ["Telugu"],
  malayalam: ["Malayalam"],
  gurmukhi: ["Gurmukhi"],
  bengali: ["Bengali"],
  gujarati: ["Gujarati"],
  cyrillic: ["Cyrillic"],
  chinese: ["Han"],
  japanese: ["Han", "Hiragana", "Katakana"],
};

/** The longest form a panel row may carry, in space-separated words. */
export const MAX_FORM_WORDS = 4;

/** Lesson types that teach a word or phrase, and so can carry a panel. */
const VOCABULARY_TYPES = new Set(["word", "phrase"]);

/** Read the comparison sets, or `undefined` for a curriculum with none. */
export function loadComparisonSets(root: string): ComparisonSets | undefined {
  const path = join(root, COMPARISON_SETS_PATH);
  return existsSync(path) ? readLedgerFile<ComparisonSets>(path) : undefined;
}

/**
 * Every owner file under `<track>/equivalents.d/`, for the given tracks.
 *
 * Read through `readShards`, so the directory gets the same symlink refusal,
 * dangerous-key rejection and parse-error scrubbing as every other ledger. A
 * track with no directory simply has no panels.
 */
export function loadEquivalents(root: string, tracks: readonly string[]): LessonEquivalents[] {
  const out: LessonEquivalents[] = [];
  for (const track of tracks) {
    // Track ids come from the checked-in registry, but they become a path here,
    // so hold them to the same shape every other per-track loader does.
    if (!/^[a-z][a-z0-9-]*$/.test(track)) throw new Error(`equivalents: unsafe track id '${track}'`);
    const shards = readShards<LessonEquivalents>(join(root, track, "equivalents.json"));
    for (const shard of shards ?? []) out.push(shard.value);
  }
  return out;
}

/**
 * The structural rules. Everything here is checkable from the data alone; the
 * one judgement it cannot make -- whether two words really share a root -- is
 * why `sameRoot` is allowed to be absent.
 */
export function validateEquivalents(input: {
  registry: LanguageRegistry;
  sets: ComparisonSets | undefined;
  lessons: readonly ParsedLesson[];
  equivalents: readonly LessonEquivalents[];
}): Issue[] {
  const issues: Issue[] = [];
  const error = (lessonId: string, code: string, message: string): void => {
    issues.push({ level: "error", code, message: `${lessonId}: ${message}`, lessonId });
  };
  const byId = new Map(input.lessons.map((lesson) => [lesson.realization.lessonId, lesson]));
  const languages = new Map(input.registry.languages.map((language) => [language.id, language]));
  const seen = new Set<string>();

  for (const entry of input.equivalents) {
    if (typeof entry !== "object" || entry === null) {
      issues.push({ level: "error", code: "equivalents-shape", message: "an equivalents owner file must hold an object" });
      continue;
    }
    const id = String(entry.lesson ?? "");
    if (seen.has(id)) error(id, "equivalents-duplicate", "more than one equivalents owner file");
    seen.add(id);
    const lesson = byId.get(id);
    if (lesson === undefined) {
      error(id, "equivalents-unknown-lesson", "no such lesson");
      continue;
    }
    const track = lesson.realization.language;
    if (!VOCABULARY_TYPES.has(lesson.realization.type)) {
      error(id, "equivalents-not-vocabulary", `a panel belongs on a word or phrase lesson, not '${lesson.realization.type}'`);
    }
    if (typeof entry.english !== "string" || entry.english.trim() === "") {
      error(id, "equivalents-english", "english must be a non-empty string");
    }
    // A pipe would split the panel's table row, whatever escaping it carried.
    const texts = [entry.english, ...(Array.isArray(entry.equivalents) ? entry.equivalents : [])
      .flatMap((equivalent) => typeof equivalent === "object" && equivalent !== null
        ? [equivalent.form, equivalent.romanization] : [])];
    if (texts.some((text) => typeof text === "string" && text.includes("|"))) {
      error(id, "equivalents-pipe", "no panel text may contain '|'");
    }
    if (typeof entry.source !== "string" || entry.source.trim() === "") {
      error(id, "equivalents-source", "source must say where the data came from");
    }
    const set = input.sets?.sets[track];
    if (set === undefined) {
      error(id, "equivalents-no-set", `track '${track}' has no comparison set in ${COMPARISON_SETS_PATH}`);
      continue;
    }
    const order = [...set.family, ...set.neighbours];
    if (!Array.isArray(entry.equivalents)) {
      error(id, "equivalents-shape", "equivalents must be a list");
      continue;
    }
    let last = -1;
    for (const equivalent of entry.equivalents) {
      if (typeof equivalent !== "object" || equivalent === null) {
        error(id, "equivalents-shape", "each equivalent must be an object");
        continue;
      }
      const where = `${equivalent.language}`;
      const at = order.indexOf(equivalent.language);
      if (at === -1) {
        error(id, "equivalents-language", `${where} is not in ${track}'s comparison set`);
        continue;
      }
      if (at <= last) error(id, "equivalents-order", `${where} is repeated or out of comparison-set order`);
      last = Math.max(last, at);
      const language = languages.get(equivalent.language);
      const scripts = language ? UNICODE_SCRIPTS_BY_REGISTRY_SCRIPT[language.script] : undefined;
      if (scripts === undefined) {
        error(id, "equivalents-language", `${where} has no known script`);
        continue;
      }
      if (typeof equivalent.form !== "string" || equivalent.form.trim() === "") {
        error(id, "equivalents-form", `${where} form must be a non-empty string`);
        continue;
      }
      if (/\s\/\s|·/u.test(equivalent.form)) {
        error(id, "equivalents-form", `${where} form '${equivalent.form}' lists several forms; give one`);
      }
      // A panel row is a word or a short phrase. A twelve-month list set in one
      // table cell cannot break across a page, and a list's "equivalent" is
      // rarely the same list (Hindi's Gregorian months are not Tamil's solar
      // ones).
      if (equivalent.form.trim().split(/\s+/u).length > MAX_FORM_WORDS) {
        error(id, "equivalents-form", `${where} form '${equivalent.form}' is longer than ${MAX_FORM_WORDS} words`);
      }
      const letters = new RegExp(`^(?:${scripts.map((script) => `\\p{Script_Extensions=${script}}`).join("|")})$`, "u");
      const stray = [...equivalent.form].filter((ch) => /[\p{L}\p{M}]/u.test(ch) && !letters.test(ch));
      if (stray.length > 0) {
        error(id, "equivalents-script", `${where} form '${equivalent.form}' has letters outside ${scripts.join("/")}: ${[...new Set(stray)].join(" ")}`);
      }
      const latin = scripts.length === 1 && scripts[0] === "Latin";
      if (!latin && (typeof equivalent.romanization !== "string" || equivalent.romanization.trim() === "")) {
        error(id, "equivalents-romanization", `${where} needs a romanization`);
      }
      if (equivalent.sameRoot !== undefined && typeof equivalent.sameRoot !== "boolean") {
        error(id, "equivalents-same-root", `${where} sameRoot must be true, false or absent`);
      }
    }
  }
  return issues;
}

/** One table cell: whitespace collapsed, so a value cannot start a new block. */
function cell(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

/**
 * True when an entry has the shape the panel needs. Validation reports every
 * malformed entry; book generation, which does not run validation first, skips
 * them rather than crashing on a `null` or a number where a string belongs.
 */
function wellFormed(entry: unknown): entry is LessonEquivalents {
  if (typeof entry !== "object" || entry === null) return false;
  const candidate = entry as Partial<LessonEquivalents>;
  return typeof candidate.lesson === "string" &&
    typeof candidate.english === "string" &&
    !candidate.english.includes("|") &&
    Array.isArray(candidate.equivalents) &&
    candidate.equivalents.every((equivalent) =>
      typeof equivalent === "object" && equivalent !== null &&
      typeof equivalent.language === "string" &&
      typeof equivalent.form === "string" && !equivalent.form.includes("|") &&
      (equivalent.romanization === undefined ||
        (typeof equivalent.romanization === "string" && !equivalent.romanization.includes("|"))));
}

/** One table row; an empty cell is written `| |`. */
function row(cells: readonly string[]): string {
  return `|${cells.map((text) => (text === "" ? " " : ` ${text} `)).join("|")}|`;
}

/**
 * The panel as Markdown: a caption and one table row per language, English
 * last. The "same root" column appears only when some row is marked, so a
 * panel nobody has judged yet does not carry a column of blanks.
 */
export function equivalentsPanelMarkdown(
  entry: LessonEquivalents,
  set: ComparisonSet,
  registry: LanguageRegistry,
): string {
  const names = new Map(registry.languages.map((language) => [language.id, language.name]));
  const marked = entry.equivalents.some((equivalent) => equivalent.sameRoot === true);
  // The word and its romanization share one cell: a long word such as
  // Malayalam എഴുന്നേൽക്കുക cannot break, and a third of the line holds it
  // where a quarter did not.
  const header = marked ? "| | word | same root |\n|---|---|---|" : "| | word |\n|---|---|";
  const rows = entry.equivalents.map((equivalent) => {
    const name = names.get(equivalent.language) ?? equivalent.language;
    const label = set.neighbours.includes(equivalent.language) ? `${name} (neighbour)` : name;
    const said = equivalent.romanization ? ` (*${cell(equivalent.romanization)}*)` : "";
    const word = `${cell(equivalent.form)}${said}`;
    const root = equivalent.sameRoot === true ? "yes" : "";
    return row(marked ? [label, word, root] : [label, word]);
  });
  rows.push(row(marked ? ["English", cell(entry.english), ""] : ["English", cell(entry.english)]));
  return `**In the family, and next door**\n\n${header}\n${rows.join("\n")}`;
}

/**
 * Lessons as the BOOK sees them: each panel appended to the lesson's first
 * teaching block (the first block that is not the warm-up).
 *
 * Only the book gets this view. Narration and the app keep the authored lesson,
 * and the lesson's `sourceHash` is left alone, as `withFilmstripImages` leaves
 * it: the chapter ledgers hash authored lessons, and the `check:books` drift
 * gate still catches a stale .tex.
 */
export function withEquivalentsPanels(
  lessons: readonly ParsedLesson[],
  equivalents: readonly LessonEquivalents[],
  sets: ComparisonSets | undefined,
  registry: LanguageRegistry,
): ParsedLesson[] {
  if (sets === undefined || equivalents.length === 0) return [...lessons];
  const byLesson = new Map(equivalents.filter(wellFormed).map((entry) => [entry.lesson, entry]));
  return lessons.map((lesson) => {
    const entry = byLesson.get(lesson.realization.lessonId);
    const set = sets.sets[lesson.realization.language];
    if (entry === undefined || set === undefined || !Array.isArray(entry.equivalents) || entry.equivalents.length === 0) {
      return lesson;
    }
    const found = lesson.blocks.findIndex((block) => block.type !== "warmup");
    const at = found === -1 ? 0 : found;
    if (lesson.blocks[at] === undefined) return lesson;
    const panel = equivalentsPanelMarkdown(entry, set, registry);
    const blocks = lesson.blocks.map((block, index) =>
      index === at ? { ...block, markdown: `${block.markdown.replace(/\s+$/, "")}\n\n${panel}\n` } : block,
    );
    return { ...lesson, blocks };
  });
}
