// Fixed facts about the curriculum that the parser and validator lean on.
import type { Script } from "./types.js";

/**
 * Which script each track is written in. HL01 imagines each track eventually
 * declaring this itself (a `track.json`); until then this map is the single
 * source of truth, kept here where it's easy to find and easy to extend when a
 * new track (e.g. Gujarati) is added.
 */
export const LANGUAGE_SCRIPT: Record<string, Script> = {
  spanish: "latin",
  french: "latin",
  german: "latin",
  italian: "latin",
  portuguese: "latin",
  latin: "latin",
  hindi: "devanagari",
  marathi: "devanagari",
  sanskrit: "devanagari",
  punjabi: "gurmukhi",
  bengali: "bengali",
  // The three tracks below were missing, and the fallback silently resolved them to
  // `latin` — a wrong answer that looks like a right one.
  //
  // Gujarati was the worked example in the comment above and never actually got an
  // entry, so all 39 of its lessons resolved to `latin`: glyph-coverage validation
  // looked its headwords up in the Latin inventory, and `romanization` fell back to
  // the Gujarati headword itself, handing a voice assistant Gujarati script in the
  // field a speech engine reads as Latin. The script ramp surfaced it — a track whose
  // script is unknown reads as having no script to learn.
  //
  // Chinese and Japanese never hit that in production because both ship a `track.json`
  // that the loader prefers. But a fallback that is wrong for some tracks is worse than
  // no fallback: it fails only in the paths that skip the loader, which is exactly where
  // a unit test lives. Completing the map costs nothing and removes the trap.
  gujarati: "gujarati",
  chinese: "chinese",
  japanese: "japanese",
  tamil: "tamil",
  kannada: "kannada",
  telugu: "telugu",
  malayalam: "malayalam",
  arabic: "arabic",
  russian: "cyrillic",
  persian: "perso-arabic",
  urdu: "urdu-nastaliq",
};

/** Lesson types that teach a real concept and take part in the cross-language join. */
export const CONTENT_TYPES = new Set(["word", "phrase"]);

// Lesson types that can REALIZE a spine concept.
//
// Deliberately WIDER than CONTENT_TYPES, and the difference is the point.
// CONTENT_TYPES answers "is this a lesson that teaches an item of language" --
// it drives book generation, the level gate, and the rule that a content lesson
// must carry a concept_tag. Realization answers a different question: "can this
// lesson be the thing that makes a spine node true?"
//
// Half the spine's concepts ARE grammar -- TENSE-BACKSHIFT, RELATIVE-CLAUSE,
// VOICE-PASSIVE, CONNECTIVE-IF -- and no `word` lesson will ever teach them. With
// realization restricted to word/phrase, a node declaring them read as
// unrealized no matter how completely the corpus taught it:
// SPINE-EXPRESS-CONDITION had ELEVEN lessons behind it and reported 0 of 4
// (HL-C169). That is a category error, not a content gap, and it sent four
// separate attempts off to author lessons the corpus already had.
//
// Kept separate rather than widening CONTENT_TYPES because that set is read by
// eight call sites that mean the narrower thing; widening it raised 33
// validation errors across other tracks on lessons that legitimately carry no
// concept_tag.
export const REALIZING_TYPES = new Set(["word", "phrase", "grammar"]);

/**
 * Lesson types that carry a session/orthography label, not a cross-language
 * concept — exempt from the join.
 *
 * - `practice` / `practice-mix` / `review` — recap sessions that re-assemble
 *   already-taught words.
 * - `writing` — an orthography lesson that teaches a *writing-system* nuance
 *   (an accent mark, a diacritic, the direction of a stroke) rather than a
 *   vocabulary item. Its `headword` is the mark itself (e.g. "◌́" for the acute
 *   accent) and its `gloss` names it; it takes no `concept_tag`, because a mark
 *   is not a word that joins across languages. See HL00 "writing nuances" and
 *   HL02 (the app renders these for hand-writing practice).
 *
 * `grammar` and `etymology` are short support lessons whose progression lives
 * in knowledge atoms rather than the cross-language vocabulary join.
 * `pattern` teaches a productive frame whose typed slots reuse already-known
 * atoms; the frame itself, not a new lexical realization, is the new knowledge.
 *
 * `pronunciation` is the newest member, and it exists because of Mandarin. Every
 * earlier track's pronunciation facts are *segmental* — "the h is silent", "this
 * vowel is long" — and a segmental fact belongs to a letter, so it lives inside
 * the word lesson that first uses that letter (HL00, "Pronunciation & Script:
 * Inline, Never a Gate"). A tone does not belong to a letter. It rides on a whole
 * syllable, it is phonemic, and third-tone sandhi changes it because of the
 * NEXT syllable, which makes it a fact about a sequence rather than about any
 * glyph. There was no lesson type for that: `grammar` would misfile a sound rule
 * as morphology, and folding it into a word lesson pushed that lesson over the
 * five-minute budget. Adding the type is the smaller, more honest change.
 */
export const EXEMPT_TYPES = new Set([
  "practice",
  "practice-mix",
  "review",
  "writing",
  // Short support lessons are ordered by knowledge atoms, not concept joins.
  "grammar",
  "etymology",
  "pronunciation",
  "pattern",
]);

/** A language-local concept id: two-letter lang prefix + SCREAMING-KEBAB name. */
export const NAMESPACED_TAG = /^[A-Z]{2}-[A-Z0-9-]+$/;

/** Longest an authored etymology hook may be (HL01). */
export const MAX_ETYMOLOGY_HOOK = 120;

/**
 * Own-property check that ignores the prototype chain. Lesson content controls
 * concept ids, so a tag like `constructor` or `toString` must NOT resolve to an
 * inherited Object member and sneak past validation as "canonical".
 */
/**
 * Strip control characters from a corpus-derived string before it reaches a
 * report line.
 *
 * WHY THIS EXISTS
 *
 * Every gate in this package interpolates author-written ids and messages into
 * lines written to stdout: lesson ids, node ids, root slugs, finding text. A
 * lesson id carrying an ANSI escape rewrites its own line in a terminal, so a
 * crafted id can erase the very defect line a reviewer is reading to decide
 * whether the corpus is sound. These reports exist to make problems visible; a
 * report that can be edited by its own subject does not.
 *
 * Removed rather than escaped. The reports are read by humans, not parsed, so a
 * visible \u001b adds noise without adding information, and the case that
 * matters is a string that should never have carried a control character at
 * all. Tab and newline are kept: they are ordinary layout, and the render
 * helpers control their own line breaks.
 */
export function stripControlCharacters(value: string): string {
  return value.replace(
    /[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g,
    "",
  );
}

export function hasOwn(obj: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(obj, key);
}
