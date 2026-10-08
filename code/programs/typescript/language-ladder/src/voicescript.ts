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
//
// HANDS AND EYES ARE NOT A VOICE. Some cues ask for a pen or a look —
// `[YOU WRITE: …]`, `[YOU TRACE: …]` — and a learner at the wheel can do
// neither. The narration generator already knows this: it marks such a prompt
// `spoken: false`, opens the lesson with a spoken notice ("this one needs your
// hands…, I will say so again when we reach it"), and in its plain-text script
// says the cue as a deferral, "once you have stopped driving — write: …", with
// no answer gap after it. HL10 §10.2 asks the same of voice mode: every sight
// or pen segment "is separately marked so voice mode can skip it and queue it".
// So this module does exactly that, and nothing cleverer:
//
//     cue                     voice step(s)
//     ----------------------  ------------------------------------------------
//     [YOU SAY: hola]         respond  "hola"                 + 8 s to answer
//     [YOU WRITE: hola]       speak    "Once you have stopped driving —
//                                       write: hola."         no gap at all
//     (end of the lesson)     speak    "Saved for when you have stopped
//                                       driving, one thing. Write: hola."
//                                                             the queue
//
// Leaving eight silent seconds after "write hola" would be worse than useless:
// it tells a driver the course expects them to do it NOW.
// And the lesson's notice, when it has one, is spoken straight after the
// title, which is where the narration's plain text puts it too.
//
// WHO DECIDES WHAT IS MANUAL. Not this file. The generator's own verdict — the
// `spoken` flag it stamps on every prompt from `MANUAL_CUE_ACTIONS` — wins
// whenever it is present, which is always for the committed corpus (CI checks
// the narration export byte-for-byte, so the flag cannot go stale). Only a
// segment that carries no verdict at all falls back to asking that same set
// directly. Either way the list of manual verbs lives in one place,
// human-language-data's `narration.ts`; when it grows, voice mode follows on
// the next regeneration without a line changing here.
// ---------------------------------------------------------------------------

import { joinQualifier } from "@coding-adventures/human-language-data/src/delivery-cue.ts";
import { MANUAL_CUE_ACTIONS } from "@coding-adventures/human-language-data/src/narration.ts";

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
  /** A prompt's cue verb, uppercased as the generator emits it: `SAY`, `WRITE`. */
  action?: string;
  /** Words between the verb and the colon, `(m.)` in `[YOU SAY (m.): …]`. */
  qualifier?: string;
  /** The generator's verdict: false when the cue needs a hand or an eye. */
  spoken?: boolean;
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
  /**
   * The spoken warning a `sight` or `pen` lesson opens with, null for a lesson
   * that is drivable all the way through.
   */
  notice?: { text?: string } | null;
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
 *
 * A lesson that needs eyes or hands opens with the generator's notice, in the
 * same place the narration's plain text puts it — straight after the title,
 * before anything the learner might start doing. And it closes with the queue
 * of hands-on cues it skipped, so the driver hears once more, all together,
 * what to come back to.
 */
export function buildVoiceScript(lesson: NarrationLesson): VoiceStep[] {
  const steps: VoiceStep[] = [];
  const deferred: string[] = [];
  if (lesson.title) steps.push({ kind: "speak", text: lesson.title });
  const notice = (lesson.notice?.text ?? "").trim();
  if (notice !== "") steps.push({ kind: "speak", text: notice });

  for (const block of lesson.blocks ?? []) {
    if (steps.length > 0) steps.push({ kind: "wait", seconds: BLOCK_GAP_SECONDS });
    if (block.title) steps.push({ kind: "speak", text: block.title });

    // `repeat` applies to the segments already emitted for THIS block, which is
    // what "[REPEAT x2]" means where it sits in the source: do that again.
    const blockStart = steps.length;
    for (const segment of block.segments ?? []) {
      appendSegment(steps, segment, blockStart, deferred);
    }
  }
  if (deferred.length > 0) {
    steps.push({ kind: "wait", seconds: BLOCK_GAP_SECONDS });
    steps.push({ kind: "speak", text: deferredRecap(deferred) });
  }
  return steps;
}

/**
 * True when a prompt asks for a hand or an eye rather than a voice.
 *
 * The generator's `spoken` flag is the answer whenever it is there — see the
 * header for why it cannot be stale. Without it (narration from before the
 * flag existed, or hand-built in a test), ask the generator's own set. A
 * multi-word verb counts as manual if ANY of its words is: `WRITE OUT` and
 * `TRACE OVER` need a pen exactly as much as `WRITE` does, and for a driver
 * the two possible mistakes are not equal — deferring a sayable cue costs one
 * rehearsal, while asking for a pen at the wheel is the failure this whole
 * branch exists to prevent.
 */
export function isHandsOnCue(segment: NarrationSegment): boolean {
  if (typeof segment.spoken === "boolean") return !segment.spoken;
  const words = (segment.action ?? "").trim().toUpperCase().split(/\s+/);
  return words.some((word) => MANUAL_CUE_ACTIONS.has(word));
}

/**
 * "write: the letter ja" — the verb and qualifier, lowercased, then the task.
 *
 * Built with the generator's own `joinQualifier`, so `[YOU WRITE (m.): …]`
 * comes out "write (m.): …" here exactly as it does in the plain-text script.
 */
function cueTask(segment: NarrationSegment, instruction: string): string {
  const action = (segment.action ?? "").trim().toLowerCase();
  if (action === "") return instruction;
  const verb = joinQualifier(action, (segment.qualifier ?? "").trim());
  return `${verb}: ${instruction}`;
}

/** Close a sentence for speech without doubling punctuation the author wrote. */
function sentence(text: string): string {
  return /[.!?…]$/.test(text) ? text : `${text}.`;
}

/**
 * The queue, said once at the end.
 *
 *   "Saved for when you have stopped driving, 2 things. Write: hola. Trace: ja."
 *
 * Each task is capitalised so a TTS engine reads it as the start of a sentence
 * rather than running it into the one before.
 */
function deferredRecap(tasks: readonly string[]): string {
  const count = tasks.length === 1 ? "one thing" : `${tasks.length} things`;
  const items = tasks.map((task) => sentence(task.charAt(0).toUpperCase() + task.slice(1)));
  return [`Saved for when you have stopped driving, ${count}.`, ...items].join(" ");
}

function appendSegment(
  steps: VoiceStep[],
  segment: NarrationSegment,
  blockStart: number,
  deferred: string[],
): void {
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
      if (isHandsOnCue(segment)) {
        // Said, not asked: the narration's own deferral wording, and no answer
        // gap after it, because there is nothing to answer at the wheel. It is
        // also queued for the recap at the end of the lesson.
        const task = cueTask(segment, instruction);
        steps.push({ kind: "speak", text: sentence(`Once you have stopped driving — ${task}`) });
        deferred.push(task);
        return;
      }
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
