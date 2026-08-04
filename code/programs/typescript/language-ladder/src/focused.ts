// Pure focused-retrieval checks used before a lesson joins mixed review.

import type { Lesson } from "./lessons.ts";
import {
  activityAnswerIsCorrect,
} from "@coding-adventures/human-language-data/src/activity.ts";
import type { CompiledLessonActivity } from "@coding-adventures/human-language-data/src/types.ts";

export { activityAnswerIsCorrect };

const MEANING_CHECK_TYPES = new Set(["word", "phrase", "new"]);

export type FocusedCheckKind = "activity" | "meaning" | "self-check";

export function focusedCheckKind(
  lesson: Pick<Lesson, "type" | "gloss"> & Partial<Pick<Lesson, "activities">>,
): FocusedCheckKind {
  if ((lesson.activities?.length ?? 0) > 0) return "activity";
  return MEANING_CHECK_TYPES.has(lesson.type) && lesson.gloss.trim() !== ""
    ? "meaning"
    : "self-check";
}

/** Prefer the authored final recall; fall back to the last typed activity. */
export function focusedActivity(
  lesson: Partial<Pick<Lesson, "activities">>,
): CompiledLessonActivity | undefined {
  const activities = lesson.activities ?? [];
  const recalls = activities.filter((activity) => activity.blockType === "recall");
  return recalls[recalls.length - 1] ?? activities[activities.length - 1];
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
