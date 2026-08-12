// ---------------------------------------------------------------------------
// voicescript.ts — turning authored narration into something a voice can run.
//
// HL10 §10.2 says voice is the PRIMARY mode, not an accessibility feature: the
// drivable-course design assumes a learner with their hands on a wheel. The
// loop it asks for is narration → prompt → the learner speaks → score → next.
//
// Almost all of the hard work is already done, and not by this file. The
// narration generator has been emitting typed segments for a while — `pause`
// with seconds, `speech` with text, `prompt` with an instruction and a response
// budget, `activity` with its accepted answers, `table` pre-flattened into
// utterances, `repeat` with a count. So this module does not parse anything.
// It walks a structure the corpus already guarantees and turns it into a flat
// list of instructions a player can execute without thinking.
//
// WHY FLATTEN AT ALL. A player that walks a tree has to hold the tree, and
// every feature (skip back, resume, repeat) becomes tree surgery. A flat list
// with an index is a cursor, and every one of those features is arithmetic.
//
// WHAT THIS DELIBERATELY LEAVES OUT. Scoring. A `respond` step carries the
// accepted answers when the corpus has them, and stops there — matching speech
// against them needs recognition this module has no business knowing about.
// The script says what to do and when; whether the learner got it right is
// somebody else's decision.
// ---------------------------------------------------------------------------

/** One thing a player does, in order. */
export type VoiceStep =
  /** Say this aloud. */
  | { kind: "speak"; text: string }
  /** Say nothing for this long — the authored thinking gap. */
  | { kind: "wait"; seconds: number }
  /**
   * Ask the learner to say something, then leave them `seconds` to do it.
   * `accepted` is present only when the corpus authored a scored activity.
   */
  | { kind: "respond"; instruction: string; seconds: number; accepted?: string[] };

/** The narration shape this module consumes, as the generator emits it. */
export interface NarrationSegment {
  kind: string;
  text?: string;
  seconds?: number;
  instruction?: string;
  responseSeconds?: number;
  prompt?: string;
  accepted?: string[];
  answer?: string;
  times?: number;
  utterances?: string[];
}

export interface NarrationBlock {
  title?: string;
  segments: NarrationSegment[];
}

export interface NarrationLesson {
  id: string;
  title?: string;
  headword?: string;
  blocks: NarrationBlock[];
}

/** Default seconds to leave for a spoken answer the corpus did not budget. */
export const DEFAULT_RESPONSE_SECONDS = 8;

/** Seconds of silence between blocks, so a lesson does not run together. */
export const BLOCK_GAP_SECONDS = 1;

/**
 * Flatten one lesson's narration into an executable script.
 *
 * Block titles are spoken. That is not decoration: a listener with no screen
 * has no other way to know that the etymology is over and the practice has
 * started, and the authored titles are already written to be said aloud
 * ("Grammar Lens: the family that costs nothing").
 */
export function buildVoiceScript(lesson: NarrationLesson): VoiceStep[] {
  const steps: VoiceStep[] = [];
  if (lesson.title) steps.push({ kind: "speak", text: lesson.title });

  for (const block of lesson.blocks ?? []) {
    if (steps.length > 0) steps.push({ kind: "wait", seconds: BLOCK_GAP_SECONDS });
    if (block.title) steps.push({ kind: "speak", text: block.title });

    // `repeat` applies to the segments already emitted for THIS block, which is
    // what "[REPEAT x2]" means where it sits in the source: do that again.
    const blockStart = steps.length;
    for (const segment of block.segments ?? []) {
      appendSegment(steps, segment, blockStart);
    }
  }
  return steps;
}

function appendSegment(steps: VoiceStep[], segment: NarrationSegment, blockStart: number): void {
  switch (segment.kind) {
    case "speech": {
      const text = (segment.text ?? "").trim();
      if (text !== "") steps.push({ kind: "speak", text });
      return;
    }
    case "pause": {
      const seconds = positive(segment.seconds);
      if (seconds > 0) steps.push({ kind: "wait", seconds });
      return;
    }
    case "prompt": {
      const instruction = (segment.instruction ?? "").trim();
      if (instruction === "") return;
      steps.push({
        kind: "respond",
        instruction,
        seconds: positive(segment.responseSeconds) || DEFAULT_RESPONSE_SECONDS,
      });
      return;
    }
    case "activity": {
      const instruction = (segment.prompt ?? "").trim();
      if (instruction === "") return;
      const accepted = acceptedOf(segment);
      steps.push({
        kind: "respond",
        instruction,
        seconds: positive(segment.responseSeconds) || DEFAULT_RESPONSE_SECONDS,
        ...(accepted.length > 0 ? { accepted } : {}),
      });
      return;
    }
    case "table": {
      // Already flattened by the generator into sayable rows, precisely because
      // a table cannot be read aloud as a table.
      for (const utterance of segment.utterances ?? []) {
        const text = utterance.trim();
        if (text !== "") steps.push({ kind: "speak", text });
      }
      return;
    }
    case "repeat": {
      const times = Math.max(0, Math.trunc(segment.times ?? 0) - 1);
      if (times === 0) return;
      // Copy what this block has produced so far, `times` more times. Slicing
      // the accumulated steps is why the flat list earns its keep.
      const body = steps.slice(blockStart);
      if (body.length === 0) return;
      for (let i = 0; i < times; i += 1) steps.push(...body.map((step) => ({ ...step })));
      return;
    }
    default:
      // An unknown segment kind is skipped rather than guessed at. A new kind
      // added by the generator should be silent here until someone teaches this
      // module what to do with it — not spoken as JSON.
      return;
  }
}

function acceptedOf(segment: NarrationSegment): string[] {
  const values = [segment.answer, ...(segment.accepted ?? [])];
  return values
    .filter((value): value is string => typeof value === "string" && value.trim() !== "")
    .map((value) => value.trim());
}

function positive(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : 0;
}

/** Total wall-clock seconds a script will take, excluding speech itself. */
export function scriptSilence(steps: readonly VoiceStep[]): number {
  return steps.reduce((total, step) => {
    if (step.kind === "wait") return total + step.seconds;
    if (step.kind === "respond") return total + step.seconds;
    return total;
  }, 0);
}

/** How many places the learner is asked to speak. */
export function respondCount(steps: readonly VoiceStep[]): number {
  return steps.filter((step) => step.kind === "respond").length;
}
