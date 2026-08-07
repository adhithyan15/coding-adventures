// Pure HL-V03 activity parsing, compilation, and answer matching.
//
// Runtime consumers use only the typed AST. They never recover prompts or
// answers from learner-facing Markdown, so prose edits cannot silently change
// what counts as a correct response.

import type {
  CompiledLessonActivity,
  LessonActivity,
  LessonActivityFeedback,
  LessonBodyBlock,
} from "./types.js";

const ACTIVITY_ID = /^[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*$/;
const ACTIVITY_KEYS = new Set([
  "id",
  "kind",
  "assesses",
  "prompt",
  "answer",
  "accepted",
  "feedback",
  "response_seconds",
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringArray(value: unknown): string[] | undefined {
  return Array.isArray(value) && value.every((item) => typeof item === "string")
    ? value
    : undefined;
}

function feedback(value: unknown): LessonActivityFeedback | undefined {
  if (!isRecord(value)) return undefined;
  const keys = Object.keys(value);
  if (keys.some((key) => key !== "correct" && key !== "incorrect")) return undefined;
  return typeof value.correct === "string" && typeof value.incorrect === "string"
    ? { correct: value.correct, incorrect: value.incorrect }
    : undefined;
}

export interface ParsedActivityValue {
  activity?: LessonActivity;
  error?: string;
}

/** Convert one JSON value from an `hl-activity` comment into the typed AST. */
export function parseLessonActivityValue(value: unknown): ParsedActivityValue {
  if (!isRecord(value)) return { error: "must contain one JSON object" };
  const unknownKeys = Object.keys(value).filter((key) => !ACTIVITY_KEYS.has(key));
  if (unknownKeys.length > 0) {
    return { error: `contains unknown field(s): ${unknownKeys.sort().join(", ")}` };
  }
  const assesses = stringArray(value.assesses);
  const accepted = stringArray(value.accepted);
  const authoredFeedback = feedback(value.feedback);
  if (
    typeof value.id !== "string" ||
    value.kind !== "text" ||
    assesses === undefined ||
    typeof value.prompt !== "string" ||
    typeof value.answer !== "string" ||
    accepted === undefined ||
    authoredFeedback === undefined ||
    typeof value.response_seconds !== "number"
  ) {
    return {
      error:
        "requires id, kind='text', assesses[], prompt, answer, accepted[], " +
        "feedback.correct, feedback.incorrect, and numeric response_seconds",
    };
  }
  return {
    activity: {
      id: value.id,
      kind: value.kind,
      assesses,
      prompt: value.prompt,
      answer: value.answer,
      accepted,
      feedback: authoredFeedback,
      responseSeconds: value.response_seconds,
    },
  };
}

/** Stable, deliberately conservative normalization for authored text answers. */
export function normalizeActivityResponse(value: string): string {
  return value
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[‘’]/g, "'")
    .replace(/[“”]/g, '"')
    .replace(/\s+/g, " ")
    .trim();
}

/** Shape and answer-resolution errors independent of a containing block. */
export function activityContractErrors(activity: LessonActivity): string[] {
  const errors: string[] = [];
  if (!ACTIVITY_ID.test(activity.id)) errors.push("id must be a stable hyphenated token");
  if (activity.assesses.length === 0) errors.push("assesses must not be empty");
  if (new Set(activity.assesses).size !== activity.assesses.length) {
    errors.push("assesses must not contain duplicates");
  }
  for (const field of ["prompt", "answer"] as const) {
    if (activity[field].trim() === "") errors.push(`${field} must not be empty`);
  }
  for (const field of ["correct", "incorrect"] as const) {
    if (activity.feedback[field].trim() === "") errors.push(`feedback.${field} must not be empty`);
  }
  if (!Number.isInteger(activity.responseSeconds) || activity.responseSeconds < 1 || activity.responseSeconds >= 300) {
    errors.push("response_seconds must be an integer from 1 through 299");
  }

  const authoredResponses = [activity.answer, ...activity.accepted];
  const normalized = authoredResponses.map(normalizeActivityResponse);
  if (normalized.some((response) => response === "")) {
    errors.push("answer and accepted variants must normalize to non-empty responses");
  }
  if (new Set(normalized).size !== normalized.length) {
    errors.push("answer and accepted variants must resolve to unique responses");
  }
  return errors;
}

/** Compile one already-validated activity into its runtime answer set. */
export function compileLessonActivity(
  activity: LessonActivity,
  block: Pick<LessonBodyBlock, "type" | "title">,
  blockIndex: number,
): CompiledLessonActivity {
  const errors = activityContractErrors(activity);
  if (errors.length > 0) throw new Error(`${activity.id}: ${errors.join("; ")}`);
  return {
    ...activity,
    blockIndex,
    blockType: block.type,
    blockTitle: block.title,
    acceptedResponses: [activity.answer, ...activity.accepted].map(normalizeActivityResponse),
  };
}

/** Flatten block-bound contracts in authored order for books and applications. */
export function compileLessonActivities(
  blocks: readonly LessonBodyBlock[],
): CompiledLessonActivity[] {
  return blocks.flatMap((block, blockIndex) =>
    (block.activities ?? []).map((activity) => compileLessonActivity(activity, block, blockIndex)),
  );
}

export function activityAnswerIsCorrect(
  answer: string,
  activity: Pick<CompiledLessonActivity, "acceptedResponses">,
): boolean {
  const normalized = normalizeActivityResponse(answer);
  return normalized !== "" && activity.acceptedResponses.includes(normalized);
}
