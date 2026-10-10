// Narration cues — what one bracketed cue SOUNDS like, and where a paragraph's
// cues are (spec: code/specs/HL08-*).
//
// Split out of `narration.ts` so that `modality.ts` can ask the narration's own
// question — "will the narrator say this sentence plainly, or defer it?" — without
// importing the narrator. `narration.ts` imports `modality.ts` (every narrated
// lesson opens with its modality notice), so a modality rule that needed
// `splitNarrationCues` from `narration.ts` would close an import cycle. The cue
// vocabulary and splitter depend on nothing but the shared cue grammar
// (`delivery-cue.ts`) and the speech helpers (`speech.ts`), so they live here and
// both modules import them. `narration.ts` re-exports every name, so no consumer
// of the old import path moves.
//
// The deciding reader is `src/drivable-instructions.ts`, which reads each lesson
// section through `splitNarrationCues` to find the steps a driver would hear
// unhedged, and `modality.ts` turns what it finds into the `eyes-or-hands-step`
// reason.

import { closingBracket, joinWrappedLines, parseDeliveryCue } from "./delivery-cue.js";
import { speakableInline } from "./speech.js";

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

/**
 * How long to leave for a learner to answer an unscored rehearsal cue.
 *
 * Eight seconds, matching `RESPONSE_SECONDS_PER_PROMPT` in `report.ts` — the same
 * number the duration estimator already uses to budget a lesson's runtime. Using two
 * different numbers for "how long does a `[YOU SAY: …]` take" would make the report's
 * timing estimate a fiction the narration then contradicts.
 *
 * Scored activities never use this: they carry their own authored `response_seconds`.
 */
export const PROMPT_RESPONSE_SECONDS = 8;

/**
 * Cue verbs that ask for a hand or an eye rather than a voice.
 *
 * `[YOU SAY: …]` and `[YOU ANSWER: …]` are things a driver can do. `[YOU WRITE: …]`
 * and `[YOU TRACE: …]` are not, so the narration says so out loud instead of asking
 * a driver to pick up a pen:
 *
 *     authored                     narrated
 *     ---------------------------  -----------------------------------------------
 *     [YOU SAY: hola]              [your turn — say: hola]  [pause 8 seconds …]
 *     [YOU COPY: **ا** once]       [once you have stopped driving — copy: ا once]
 *
 * This set used to stop at six verbs, on the theory that a new verb that was
 * genuinely manual would be caught by the lesson's `type: writing` or its script
 * block long before it got here. It was not. `[YOU COPY: …]` sat in three drivable
 * lessons (MW-C01-raam-raam-saa, FA-C01-practice, TA-C01-practice) and
 * `[YOU CIRCLE: …]` in a fourth (TE-R152-jhari-recall), none of them a writing
 * lesson, and the narration read all four to a driver as an ordinary turn. Forty
 * drivable `[YOU LOOK: at **था** and put your finger on …]` cues and about a hundred
 * and thirty drivable `[YOU READ: **でぐち** …]` cues did the same to the eyes. So
 * the default is no longer trusted on its own: {@link SPOKEN_CUE_ACTIONS} lists the
 * verbs that are NOT here, and a corpus test (`cue-action-classification.test.ts`)
 * fails on any head verb in neither set. The next new verb is a decision somebody
 * makes, not a default nobody noticed.
 *
 * Each verb, and what it needs besides a voice:
 *
 *   verb       needs  why
 *   ---------  -----  --------------------------------------------------------------
 *   WRITE      hand   a pen on paper — the case this set was made for
 *   TRACE      hand   a finger or pen following a letter's outline
 *   COPY       hand   writing again from a visible model ("**வ** once beside the
 *                     visible model"); copying a SOUND is authored as SAY, not COPY
 *   CIRCLE     hand   a pen ring round a letter on the page
 *   COVER      hand   a hand over the page before writing from memory — every COVER
 *                     in the corpus is "cover, then write"
 *   DRAW       hand   no cue uses these four yet. They are the pen verbs the prose
 *   UNDERLINE  hand   detector in src/drivable-instructions.ts already
 *   MARK       hand   treats as writing, or their nearest siblings, so the first
 *   TICK       hand   cue to use one is deferred rather than read out
 *   TAP        hand   tapping out beats with a finger ("*yu | hold | ku | ri*") —
 *                     the same family as GESTURE
 *   GESTURE    hand   a movement that carries the meaning
 *   CLAP       hand   clapping out beats; TAP's louder sibling. No cue heads with it:
 *                     the 24 drivable "[YOU SAY: *denwa*, clapping three beats]" drills
 *                     now count the beats aloud instead
 *   SHOW       hand   holding up fingers or a thing. No cue heads with it either; four
 *                     drivable number drills nested it ("[YOU HEAR: *añcŭ*; YOU SHOW: 5]")
 *                     and now say the number. The gesture check in
 *                     src/drivable-instructions.ts reads a nested cue's verb with
 *                     {@link isManualCueAction}, so listing SHOW here is what it catches
 *   LABEL      hand   writing a label against something on the page
 *   FEEL       hand   a hand at the throat or the mouth, feeling the sound
 *   TEST       hand   the corpus's one TEST is FEEL by another name: "hand at the
 *                     mouth — **chār** still, **chhe** breathed"
 *   POINT      eye    a finger on a printed letter
 *   LOOK       eye    looking at a printed word ("at ಆರು and find the ರ")
 *   READ       eye    reading printed script, which is the eyes' job whatever the
 *                     mouth does next — READ ALOUD included (see below)
 *   FIND       eye    finding a sign inside a printed word
 *   CHECK      eye    checking a written shape ("compact left side; open right
 *                     side") — the corpus's one CHECK is a handwriting check
 *   STACK      eye    placing a component on the page ("point below 五, where 口
 *                     will go")
 *
 * A cue action can be several words — `READ ALOUD`, `COVER AND WRITE`,
 * `WRITE FROM THE HEARD OR ROMANIZED CUE` — and {@link isManualCueAction} counts the
 * cue as manual when ANY of its words is listed here, so a `[YOU SAY AND WRITE: …]`
 * cannot slip through on the strength of its first word.
 */
export const MANUAL_CUE_ACTIONS: ReadonlySet<string> = new Set([
  "WRITE",
  "TRACE",
  "COPY",
  "CIRCLE",
  "COVER",
  "DRAW",
  "UNDERLINE",
  "MARK",
  "TICK",
  "TAP",
  "GESTURE",
  "CLAP",
  "SHOW",
  "LABEL",
  "FEEL",
  "TEST",
  "POINT",
  "LOOK",
  "READ",
  "FIND",
  "CHECK",
  "STACK",
]);

/**
 * Cue verbs a driver can do with the voice and the ear alone.
 *
 * The narration never consults this set — anything outside {@link MANUAL_CUE_ACTIONS}
 * is already spoken — so it exists for the corpus test that demands every cue's head
 * verb (the first word of its action) be classified one way or the other. A verb is
 * here only when its uses in the corpus can be done by ear: `HEAR` and `LISTEN` are
 * the ears; `NOTICE`, `CONTRAST`, `SPLIT` and their siblings ask for a thought said
 * aloud; `COUNT` counts beats and endings as well as dots, and its dot-counting cues
 * sit in lessons that already need eyes.
 *
 * Words that only ever follow a head verb (`ALOUD`, `AND`, `THE`, `BY`, …) are not
 * listed. They are not verbs, and {@link isManualCueAction} only asks whether they
 * are manual, which they are not.
 */
export const SPOKEN_CUE_ACTIONS: ReadonlySet<string> = new Set([
  // Speaking, asking and answering.
  "SAY", "ANSWER", "ASK", "REPLY", "REQUEST", "GREET", "OFFER", "GRANT", "CLOSE",
  "PUSH", "ADMIT", "HEDGE", "QUALIFY", "REJECT", "STATE", "EXPLAIN", "NAME", "SPELL",
  "LIST", "PRODUCE", "TRANSLATE", "PARAPHRASE", "NOTE", "REPAIR", "USE", "KEEP",
  "ADD", "FRAME", "JOIN", "COMBINE", "SWAP", "SWITCH", "TURN", "FLIP", "CONVERT",
  // Sounds made with the mouth alone.
  "HUM", "NASALIZE",
  // Remembering.
  "RECALL", "RETRIEVE", "RETURN",
  // Listening.
  "HEAR", "LISTEN",
  // Working on words in the head, then saying the result.
  "BUILD", "REBUILD", "RUN", "SEGMENT", "SPLIT", "STRIP", "SEPARATE", "DERIVE",
  "APPLY", "PAIR", "MATCH", "CONNECT", "CONTRAST", "COMPARE", "CLASSIFY", "SORT",
  "CHOOSE", "DECIDE", "IDENTIFY", "NOTICE", "COUNT",
]);

/**
 * True when a cue action needs a hand or an eye: when ANY word of it is one of the
 * {@link MANUAL_CUE_ACTIONS}.
 *
 *     WRITE             manual
 *     READ ALOUD        manual  (READ)
 *     COVER AND BUILD   manual  (COVER)
 *     SAY WHY           spoken
 *     CHOOSE BY CONTEXT spoken
 *
 * Before this was a function the narration looked only at the first word, which for
 * every action in today's corpus gives the same answer. Reading every word costs
 * nothing and means a hand-on verb later in a compound action is still heard.
 *
 * `parseDeliveryCue` always hands over upper-case words joined by one space, but this
 * is exported, and a caller passing `"write"` or `"SAY\tWRITE"` must not be told
 * "spoken" — that is the unsafe answer for a driver. So the action is upper-cased and
 * split on any whitespace run first (one linear pass each).
 */
export function isManualCueAction(action: string): boolean {
  return action
    .toUpperCase()
    .split(/\s+/)
    .some((word) => MANUAL_CUE_ACTIONS.has(word));
}

/** A silence the lesson asked for. `perItem` marks `[PAUSE 1s each]` over a list. */
export interface NarrationPause {
  kind: "pause";
  seconds: number;
  perItem: boolean;
  /** The cue exactly as authored, so a reader can trace any segment to its source. */
  source: string;
}

/** `[REPEAT x2]` — say the previous utterance again, this many times. */
export interface NarrationRepeat {
  kind: "repeat";
  times: number;
  source: string;
}

/**
 * `[YOU SAY: …]` and its siblings: the learner does something.
 *
 * `scored` is permanently `false` — see the header. A prompt is a rehearsal, not an
 * assessment; the field exists so a consumer can tell prompts and activities apart
 * without knowing this module's rules.
 */
export interface NarrationPrompt {
  kind: "prompt";
  /** The cue verb, uppercased: `SAY`, `WRITE`, `BUILD`, … */
  action: string;
  /**
   * The author's words between the verb and the colon, spoken with the verb:
   * `(m.)` in `[YOU SAY (m.): …]`, `the pattern` in `[YOU RUN the pattern: …]`.
   * Absent when the cue has none, which is almost every cue.
   */
  qualifier?: string;
  /** What to do, already stripped of Markdown. */
  instruction: string;
  /** True when the learner can do this with their mouth alone. */
  spoken: boolean;
  scored: false;
  /** Silence to leave afterwards, in seconds. */
  responseSeconds: number;
  source: string;
}

/** Any directive a bracketed cue can turn into. */
export type NarrationCue = NarrationPause | NarrationRepeat | NarrationPrompt;

// ---------------------------------------------------------------------------
// Cue parsing
// ---------------------------------------------------------------------------
//
// The grammar of a cue — where its bracket closes and what its words mean — lives
// in `delivery-cue.ts`, shared with the printed book. This section only decides
// what each cue *sounds like*. It used to own a private copy of the grammar, and
// the two copies disagreed: a `[YOU SAY (m.): …]` was a cue to neither, so the
// voice read the brackets out as prose, and a cue wrapped across two source lines
// was a cue to this file but raw text to the book. One grammar, two renderings.



/**
 * Turn the inside of one `[…]` into a directive, or return null if it is not a cue.
 *
 * Returning null matters as much as returning a cue: lessons are full of ordinary
 * brackets — Markdown links, parenthetical asides, `[bonjour]` glosses — and treating
 * one of those as a directive would delete real teaching content from the script.
 * Only the three authored shapes count:
 *
 *   `PAUSE 2s`        `PAUSE 1s each`        `REPEAT x2`        `YOU SAY: …`
 *
 * and the prompt shape may carry a qualifier between its verb and its colon —
 * `YOU SAY (m.): …`, `YOU RUN the pattern: …` — which is spoken with the verb.
 */
export function parseNarrationCue(inner: string): NarrationCue | null {
  const cue = parseDeliveryCue(inner);
  if (!cue) return null;
  const source = `[${joinWrappedLines(inner)}]`;
  switch (cue.kind) {
    case "pause":
      return { kind: "pause", seconds: cue.seconds, perItem: cue.perItem, source };
    case "repeat":
      return { kind: "repeat", times: cue.times, source };
    case "prompt": {
      const prompt: NarrationPrompt = {
        kind: "prompt",
        action: cue.action,
        instruction: speakableInline(cue.content),
        spoken: !isManualCueAction(cue.action),
        scored: false,
        responseSeconds: PROMPT_RESPONSE_SECONDS,
        source,
      };
      // Present only when authored, so every unqualified prompt's JSON is
      // byte-for-byte what it was before qualifiers were understood.
      if (cue.qualifier !== "") prompt.qualifier = speakableInline(cue.qualifier);
      return prompt;
    }
  }
}

/** A span of a paragraph: either words to say or a directive to obey. */
export type CueSplit = { text: string } | { cue: NarrationCue };

/**
 * Split a paragraph into prose spans and cues.
 *
 * The bracket scan ({@link closingBracket}) tracks depth, because the corpus nests
 * brackets inside cues for real — `[YOU SAY: the pattern — "[nā] [pēru]"]` — and
 * stopping at the first `]` would cut the cue in half and leave the tail as garbled
 * prose. Its lookahead is bounded, so the walk stays linear in the paragraph.
 *
 * A bracket run that is not a cue is handed back **including its brackets**, and with
 * any following `(…)` attached, so that Markdown links survive intact for
 * {@link speakableInline} to resolve.
 */
export function splitNarrationCues(text: string): CueSplit[] {
  const parts: CueSplit[] = [];
  let buffer = "";
  let index = 0;
  const flush = (): void => {
    if (buffer.trim() !== "") parts.push({ text: buffer });
    buffer = "";
  };
  while (index < text.length) {
    const character = text[index];
    if (character === "\\" && index + 1 < text.length) {
      buffer += text.slice(index, index + 2);
      index += 2;
      continue;
    }
    if (character !== "[") {
      buffer += character;
      index += 1;
      continue;
    }
    const close = closingBracket(text, index);
    if (close === -1) {
      // An unbalanced `[`. Keep it as prose rather than swallowing the rest of the
      // paragraph — losing content is the one outcome this module refuses.
      buffer += character;
      index += 1;
      continue;
    }
    const cue = parseNarrationCue(text.slice(index + 1, close));
    if (cue) {
      flush();
      parts.push({ cue });
      index = close + 1;
      continue;
    }
    let end = close + 1;
    if (text[end] === "(") {
      const paren = text.indexOf(")", end);
      if (paren !== -1) end = paren + 1;
    }
    buffer += text.slice(index, end);
    index = end;
  }
  flush();
  return parts;
}
