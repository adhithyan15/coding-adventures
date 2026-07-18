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

/** Lesson types that are session labels (review/recap) — exempt from the join. */
export const EXEMPT_TYPES = new Set(["practice", "practice-mix", "review"]);

/** A language-local concept id: two-letter lang prefix + SCREAMING-KEBAB name. */
export const NAMESPACED_TAG = /^[A-Z]{2}-[A-Z0-9-]+$/;

/** Longest an authored etymology hook may be (HL01). */
export const MAX_ETYMOLOGY_HOOK = 120;
