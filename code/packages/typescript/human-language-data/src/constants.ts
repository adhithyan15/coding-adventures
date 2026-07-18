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
  tamil: "tamil",
  kannada: "kannada",
  telugu: "telugu",
  malayalam: "malayalam",
  arabic: "arabic",
};

/** Lesson types that teach a real concept and take part in the cross-language join. */
export const CONTENT_TYPES = new Set(["word", "phrase"]);

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
 */
export const EXEMPT_TYPES = new Set([
  "practice",
  "practice-mix",
  "review",
  "writing",
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
export function hasOwn(obj: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(obj, key);
}
