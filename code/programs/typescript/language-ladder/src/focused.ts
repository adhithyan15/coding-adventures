// Pure focused-retrieval checks used before a lesson joins mixed review.

import type { Lesson } from "./lessons.ts";

const MEANING_CHECK_TYPES = new Set(["word", "phrase", "new"]);

export type FocusedCheckKind = "meaning" | "self-check";

export function focusedCheckKind(lesson: Pick<Lesson, "type" | "gloss">): FocusedCheckKind {
  return MEANING_CHECK_TYPES.has(lesson.type) && lesson.gloss.trim() !== ""
    ? "meaning"
    : "self-check";
}

export function normalizeFocusedAnswer(value: string): string {
  return value
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLocaleLowerCase("en")
    .replace(/[’']/g, "")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim()
    .replace(/\s+/g, " ");
}

/** One complete gloss or any top-level slash/semicolon/or alternative. */
export function acceptedMeanings(gloss: string): string[] {
  const withoutNotes = gloss.replace(/\s*\([^)]*\)\s*/g, " ").trim();
  const candidates = [withoutNotes, ...withoutNotes.split(/\s+(?:\/|;|or)\s+/i)];
  return [...new Set(candidates.map(normalizeFocusedAnswer).filter((value) => value !== ""))];
}

export function meaningAnswerIsCorrect(answer: string, gloss: string): boolean {
  const normalized = normalizeFocusedAnswer(answer);
  return normalized !== "" && acceptedMeanings(gloss).includes(normalized);
}
