// Narration — the script an AI voice assistant reads while the learner drives.
//
// This is the "audio script" output HL04's one-source pipeline diagram has named
// since the day it was written, and which nothing had ever built. HL08 specifies it.
// The goal is stated best in the project owner's own words: *"I want to be able to
// have one of the AI chatbots with voice capabilities read through and teach me while
// I am driving."*
//
// ---------------------------------------------------------------------------
// What this module is NOT
// ---------------------------------------------------------------------------
//
// It produces **no audio**. No text-to-speech, no voice selection, no recordings.
// HL04 keeps all of that out of scope and HL08 repeats it. What comes out of here is
// a *script*: words to say, places to stop, questions to ask, and answers to score.
// Something else — a voice agent, a TTS engine, a person — performs it.
//
// ---------------------------------------------------------------------------
// Two outputs, and why both
// ---------------------------------------------------------------------------
//
// **Plain text.** One continuous script, start to finish. Hand it to any voice
// assistant with "read this to me" and it works. Directives appear as bracketed
// stage directions (`[pause 2 seconds]`) exactly because that is a form every model
// already understands as *an instruction to the reader*, not words to pronounce.
//
// **Structured JSON.** The same script with its joints intact: which span is prose,
// which is a two-second silence, which is a question the learner must answer aloud,
// and — crucially — which questions can be *scored*. A voice agent driving the JSON
// can wait for a spoken answer and mark it right or wrong. A voice agent reading the
// plain text can only talk.
//
// ---------------------------------------------------------------------------
// Where a correct answer comes from (this is the important rule)
// ---------------------------------------------------------------------------
//
// `activity.ts` opens with a rule this module obeys without exception:
//
//     Runtime consumers use only the typed AST. They never recover prompts or
//     answers from learner-facing Markdown, so prose edits cannot silently change
//     what counts as a correct response.
//
// Lessons contain two superficially similar things, and conflating them would break
// that rule:
//
//   `[YOU SAY: "hola" — OH-la, silent h]`   a *rehearsal cue*. The learner speaks.
//                                           There is no answer key. Nothing is
//                                           scored. The agent waits, then moves on.
//
//   `<!-- hl-activity: {...} -->`           a *contract*. Typed, validated, with an
//                                           `answer`, `accepted` variants, feedback,
//                                           and a response window. THIS is what a
//                                           spoken answer is scored against.
//
// So this module never guesses that a `[YOU SAY: …]` cue "is really" the block's
// activity. Cues become `prompt` segments with `scored: false`; compiled activities
// become `activity` segments with `scored: true` and the compiled `acceptedResponses`
// straight off `compileLessonActivities`. If an author wants a spoken answer graded,
// they author an activity — the same as everywhere else in this package.
//
// ---------------------------------------------------------------------------
// The safety rule
// ---------------------------------------------------------------------------
//
// **Nothing is ever silently dropped.** A table this export cannot read aloud does
// not disappear; it becomes a spoken notice naming its size, its column headings, and
// why it was refused, and it forces its lesson to `sight` so the learner is told up
// front. The failure this design fears most is not an awkward sentence. It is a
// learner who finishes a lesson without knowing that part of it never reached them.

import { compileLessonActivity } from "./activity.js";
import { canonicalChapterHash } from "./hash.js";
import {
  deriveLessonModality,
  orderChapterLessons,
  type LessonModality,
  type Modality,
  type ModalityReasonCode,
} from "./modality.js";
import type { ParsedLesson } from "./parse.js";
import {
  collapseSpaces,
  DEFAULT_LINEARISABLE_TABLE_COLUMNS,
  endSentence,
  isTableRowLine,
  linariseTable,
  findMarkdownTables,
  speakableInline,
  TABLE_REFUSAL_MESSAGES,
  type TableRefusalReason,
} from "./speech.js";
import type { CompiledLessonActivity, LessonBlockType } from "./types.js";

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
 * a driver to pick up a pen. Everything not listed here is treated as speakable,
 * which is the safe default: the corpus's long tail (`BUILD`, `CONTRAST`, `SEGMENT`,
 * `PARAPHRASE`, …) is all sayable, and a new verb that is genuinely manual will be
 * caught by the lesson's `type: writing` or its script block long before it gets
 * here.
 */
export const MANUAL_CUE_ACTIONS: ReadonlySet<string> = new Set([
  "WRITE",
  "TRACE",
  "POINT",
  "GESTURE",
  "LABEL",
  "FEEL",
]);

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
  /** What to do, already stripped of Markdown. */
  instruction: string;
  /** True when the learner can do this with their mouth alone. */
  spoken: boolean;
  scored: false;
  /** Silence to leave afterwards, in seconds. */
  responseSeconds: number;
  source: string;
}

/** Ordinary prose, ready to say. */
export interface NarrationSpeech {
  kind: "speech";
  text: string;
}

/** A table that was successfully turned into sentences. */
export interface NarrationTable {
  kind: "table";
  headers: string[];
  columns: number;
  rowCount: number;
  utterances: string[];
}

/**
 * A table that could not be. Never a silent omission — this segment is *spoken*.
 *
 * The learner hears how big it is, what its columns are called, and why it needs
 * eyes, so they know precisely what to come back to.
 */
export interface NarrationTableSkipped {
  kind: "table-skipped";
  reason: TableRefusalReason;
  columns: number;
  rowCount: number;
  headers: string[];
  /** The spoken sentence a voice reads in place of the table. */
  text: string;
}

/** A scored retrieval contract from `hl-activity`, ready for a voice agent to grade. */
export interface NarrationActivity {
  kind: "activity";
  scored: true;
  id: string;
  /** Authored prompt, Markdown stripped. */
  prompt: string;
  assesses: string[];
  responseSeconds: number;
  /** Normalized answer set from `compileLessonActivities` — never re-derived from prose. */
  acceptedResponses: string[];
  feedback: { correct: string; incorrect: string };
}

export type NarrationSegment =
  | NarrationSpeech
  | NarrationPause
  | NarrationRepeat
  | NarrationPrompt
  | NarrationTable
  | NarrationTableSkipped
  | NarrationActivity;

/** Any directive a bracketed cue can turn into. */
export type NarrationCue = NarrationPause | NarrationRepeat | NarrationPrompt;

/** One `## …` section of a lesson, narrated. */
export interface NarrationBlock {
  index: number;
  type: LessonBlockType;
  /** The heading, spoken. Empty for the opening material before the first heading. */
  title: string;
  segments: NarrationSegment[];
}

/** The spoken warning that opens a lesson needing eyes or a hand. */
export interface NarrationNotice {
  modality: Modality;
  /** What the learner must have to hand. */
  needs: string[];
  /** Named sections to come back to once they have stopped. */
  waitUntilStopped: string[];
  /** The whole warning as one spoken paragraph. */
  text: string;
}

/** A problem worth reporting, collected rather than thrown. */
export interface NarrationFinding {
  code: "narration-block-unrenderable" | "narration-activity-invalid";
  lessonId: string;
  language: string;
  message: string;
}

export interface LessonNarration {
  lessonId: string;
  language: string;
  chapter: number | null;
  sequence: number | null;
  /** The `# …` line, spoken. */
  title: string;
  headword: string;
  romanization: string;
  gloss: string;
  script: string;
  modality: Modality;
  derivedModality: Modality;
  modalityReasons: ModalityReasonCode[];
  /** Fingerprint of the lesson AST this narration was generated from. */
  sourceHash: string;
  notice: NarrationNotice | null;
  blocks: NarrationBlock[];
  findings: NarrationFinding[];
}

export interface ChapterNarration {
  language: string;
  chapter: number;
  title: string;
  /** How many lessons, from the front, are drivable — HL08's number for a commuter. */
  drivablePrefix: number;
  lessonIds: string[];
  /** Combined fingerprint of every lesson AST in the chapter, as the book uses. */
  sourceHash: string;
  lessons: LessonNarration[];
  findings: NarrationFinding[];
}

export interface NarrationOptions {
  /** Widest table still read aloud. Defaults to the lineariser's measured default. */
  maxLinearisableTableColumns?: number;
  /**
   * Extra headword→romanization pairs to apply, normally every other lesson in the
   * same chapter. See {@link pairRomanization} for why a chapter-wide glossary beats
   * a lesson-local one.
   */
  glossary?: ReadonlyArray<RomanizationPair>;
  /** Display name for the track, e.g. "Telugu". Falls back to the slug. */
  languageName?: string;
  /** Chapter title from the HL05 ledger. Falls back to "Chapter N". */
  chapterTitle?: string;
}

/** One target-script word and how to say it. */
export interface RomanizationPair {
  headword: string;
  romanization: string;
}

// ---------------------------------------------------------------------------
// Cue parsing
// ---------------------------------------------------------------------------

function isDigit(character: string | undefined): boolean {
  return character !== undefined && character >= "0" && character <= "9";
}

/**
 * Read a decimal number starting at `index`; returns null when there is none.
 *
 * Hand-scanned, like everything else here. `[PAUSE 1.5s]` is not in the corpus today
 * but costs one line to support and would otherwise silently become literal prose.
 */
function readNumber(text: string, index: number): { value: number; next: number } | null {
  let cursor = index;
  let digits = "";
  while (isDigit(text[cursor])) {
    digits += text[cursor];
    cursor += 1;
  }
  if (text[cursor] === "." && isDigit(text[cursor + 1])) {
    digits += ".";
    cursor += 1;
    while (isDigit(text[cursor])) {
      digits += text[cursor];
      cursor += 1;
    }
  }
  if (digits === "") return null;
  return { value: Number(digits), next: cursor };
}

/**
 * Turn the inside of one `[…]` into a directive, or return null if it is not a cue.
 *
 * Returning null matters as much as returning a cue: lessons are full of ordinary
 * brackets — Markdown links, parenthetical asides, `[bonjour]` glosses — and treating
 * one of those as a directive would delete real teaching content from the script.
 * Only the three authored shapes count:
 *
 *   `PAUSE 2s`        `PAUSE 1s each`        `REPEAT x2`        `YOU SAY: …`
 */
export function parseNarrationCue(inner: string): NarrationCue | null {
  const text = inner.trim();
  const source = `[${text}]`;

  if (text.startsWith("PAUSE ")) {
    const number = readNumber(text, "PAUSE ".length);
    if (!number) return null;
    let cursor = number.next;
    if (text[cursor] === "s") cursor += 1;
    const rest = text.slice(cursor).trim().toLowerCase();
    if (rest !== "" && rest !== "each") return null;
    return { kind: "pause", seconds: number.value, perItem: rest === "each", source };
  }

  if (text.startsWith("REPEAT ")) {
    let cursor = "REPEAT ".length;
    if (text[cursor] === "x" || text[cursor] === "X") cursor += 1;
    const number = readNumber(text, cursor);
    if (!number || text.slice(number.next).trim() !== "") return null;
    return { kind: "repeat", times: number.value, source };
  }

  if (text.startsWith("YOU ")) {
    const colon = text.indexOf(":");
    if (colon === -1) return null;
    const action = text.slice("YOU ".length, colon).trim();
    // A cue verb is capitals and spaces — `YOU SAY`, `YOU CHOOSE BY CONTEXT`. Anything
    // else is prose that happens to open with the word "you", e.g. `[you: see below]`.
    if (action === "" || !isCueAction(action)) return null;
    const instruction = speakableInline(text.slice(colon + 1));
    return {
      kind: "prompt",
      action,
      instruction,
      spoken: !MANUAL_CUE_ACTIONS.has(action.split(" ")[0] ?? action),
      scored: false,
      responseSeconds: PROMPT_RESPONSE_SECONDS,
      source,
    };
  }

  return null;
}

function isCueAction(action: string): boolean {
  for (const character of action) {
    const isUpper = character >= "A" && character <= "Z";
    if (!isUpper && character !== " ") return false;
  }
  return true;
}

/** A span of a paragraph: either words to say or a directive to obey. */
type CueSplit = { text: string } | { cue: NarrationCue };

/**
 * How far ahead {@link splitNarrationCues} will look for a cue's closing bracket.
 *
 * 4,096 characters, against a corpus whose longest authored cue is a couple of
 * hundred. It exists to bound the scan, not to reject anything real — see the comment
 * at the loop.
 */
const MAX_CUE_LENGTH = 4096;

/**
 * Split a paragraph into prose spans and cues.
 *
 * The bracket scan tracks depth, because the corpus nests brackets inside cues for
 * real — `[YOU SAY: the pattern — "[nā] [pēru]"]` — and stopping at the first `]`
 * would cut the cue in half and leave the tail as garbled prose.
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
    let depth = 0;
    let cursor = index;
    let close = -1;
    // Bounded lookahead. An unbalanced `[` makes this scan run to the end of the
    // paragraph and then advance by one character, so a line of nothing but `[[[[[…`
    // costs O(n²). The longest real cue in the corpus is a couple of hundred
    // characters, so capping the search keeps every genuine cue intact while making
    // the total work linear in the text with a fixed constant.
    const limit = Math.min(text.length, index + MAX_CUE_LENGTH);
    while (cursor < limit) {
      const scanned = text[cursor];
      if (scanned === "\\") {
        cursor += 2;
        continue;
      }
      if (scanned === "[") depth += 1;
      else if (scanned === "]") {
        depth -= 1;
        if (depth === 0) {
          close = cursor;
          break;
        }
      }
      cursor += 1;
    }
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

// ---------------------------------------------------------------------------
// Romanization pairing
// ---------------------------------------------------------------------------

/**
 * Follow target-script text with how to say it.
 *
 * HL08: *"Target-language text carries its `romanization` alongside, so a voice
 * engine reading a Latin-script transcription is never guessing at the script."* A
 * TTS engine handed `خداحافظ` with an English voice will produce silence or noise;
 * handed `خداحافظ (khodâ hâfez)` it produces something a learner can repeat.
 *
 * Two deliberate restraints:
 *
 * 1. **Chapter-wide, not lesson-local.** A Persian lesson on خداحافظ freely mentions
 *    خدا and حافظ from the two lessons before it. Those words' romanizations live in
 *    *their* frontmatter, so `narrateChapter` passes every lesson's pair to every
 *    other lesson. Longest headword first, so the compound is paired before its parts
 *    and we never produce `خدا(khodâ)حافظ`.
 *
 * 2. **Never pair twice.** Authors already write `**خداحافظ** — *khodâ hâfez*` by
 *    hand in most lessons. If the romanization is already somewhere in this span, the
 *    pairing is skipped: hearing "khodâ hâfez khodâ hâfez" teaches nothing and sounds
 *    broken. The cost is that a later bare mention in the same paragraph stays bare,
 *    which is the right trade — a listener who just heard the pronunciation does not
 *    need it again two clauses later.
 *
 * 3. **Whole words only.** This one was a real bug before it was a rule. The Arabic
 *    track teaches the single letter ا (*alif*) as its own lesson, and a plain
 *    substring replace turned the word سلام into `سلا (alif)م` — the pronunciation
 *    guide spliced into the middle of the word it was supposed to help with. Arabic,
 *    Telugu and Devanagari do not put spaces between letters, so "is this occurrence a
 *    word?" has to be asked explicitly: a match counts only when the characters on
 *    either side of it are not letters or combining marks.
 */
export function pairRomanization(text: string, pairs: readonly RomanizationPair[]): string {
  let out = text;
  const ordered = [...pairs].sort((left, right) => right.headword.length - left.headword.length);
  for (const { headword, romanization } of ordered) {
    if (headword === "" || romanization === "" || headword === romanization) continue;
    if (!out.includes(headword)) continue;
    if (out.includes(romanization)) continue;
    out = replaceWholeWord(out, headword, `${headword} (${romanization})`);
  }
  return out;
}

/** Single-character class test. Not a pattern over the text — no backtracking to have. */
const LETTER_OR_MARK = /[\p{L}\p{M}\p{N}]/u;

function isWordCharacter(character: string | undefined): boolean {
  return character !== undefined && LETTER_OR_MARK.test(character);
}

/**
 * Replace every whole-word occurrence of `needle`, left to right.
 *
 * `indexOf` from a moving cursor, so the walk is linear and the replacement text is
 * never rescanned — which also means a headword that happens to appear inside its own
 * romanization cannot send this into a loop.
 */
function replaceWholeWord(text: string, needle: string, replacement: string): string {
  let out = "";
  let cursor = 0;
  while (cursor <= text.length) {
    const found = text.indexOf(needle, cursor);
    if (found === -1) break;
    const before = found > 0 ? text[found - 1] : undefined;
    const after = text[found + needle.length];
    if (isWordCharacter(before) || isWordCharacter(after)) {
      out += text.slice(cursor, found + needle.length);
      cursor = found + needle.length;
      continue;
    }
    out += text.slice(cursor, found) + replacement;
    cursor = found + needle.length;
  }
  return out + text.slice(cursor);
}

// ---------------------------------------------------------------------------
// Narrating one lesson
// ---------------------------------------------------------------------------

function stringValue(value: ParsedLesson["frontmatter"][string] | undefined): string {
  return typeof value === "string" ? value : "";
}

function numberOrNull(value: string): number | null {
  if (value.trim() === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

/** The `# …` heading, or the headword when a lesson has none. */
export function narrationTitle(lesson: ParsedLesson): string {
  const firstLine = lesson.preamble.split(/\r?\n/).find((line) => line.trim() !== "") ?? "";
  const raw = firstLine.startsWith("# ") ? firstLine.slice(2) : lesson.realization.headword;
  return speakableInline(raw);
}

/** Peel `>` blockquote markers off the front of a line, leaving the content. */
function stripQuoteMarkers(line: string): string {
  let text = line.trimStart();
  while (text.startsWith(">")) text = text.slice(1).trimStart();
  return text;
}

/** Strip a line's block-level Markdown furniture: bullets, quote marks, headings. */
function stripLinePrefix(line: string): string {
  let text = line.trim();
  while (text.startsWith(">")) text = text.slice(1).trim();
  if (text.startsWith("- ") || text.startsWith("* ") || text.startsWith("+ ")) {
    return text.slice(2).trim();
  }
  let hashes = 0;
  while (text[hashes] === "#") hashes += 1;
  if (hashes > 0 && text[hashes] === " ") return text.slice(hashes + 1).trim();
  // Ordered list markers: `1. `, `12) `.
  let digits = 0;
  while (isDigit(text[digits])) digits += 1;
  if (digits > 0 && (text[digits] === "." || text[digits] === ")") && text[digits + 1] === " ") {
    return text.slice(digits + 2).trim();
  }
  return text;
}

/** True when a line starts a new spoken unit rather than continuing the last one. */
function startsNewUnit(line: string): boolean {
  const text = line.trim();
  if (text.startsWith("- ") || text.startsWith("* ") || text.startsWith("+ ")) return true;
  if (text.startsWith(">")) return true;
  if (text.startsWith("#")) return true;
  let digits = 0;
  while (isDigit(text[digits])) digits += 1;
  return digits > 0 && (text[digits] === "." || text[digits] === ")") && text[digits + 1] === " ";
}

/**
 * Turn one block's Markdown into segments.
 *
 * The walk is line-based and does exactly three things: peel table runs off into the
 * lineariser, group the remaining lines into spoken units (a paragraph, a bullet, a
 * blockquote), and split each unit into prose and cues. Anything it does not
 * recognise falls through as prose, which is the failure direction that loses nothing.
 */
function narrateMarkdown(
  markdown: string,
  options: { maxColumns: number; pairs: readonly RomanizationPair[] },
): NarrationSegment[] {
  const segments: NarrationSegment[] = [];
  const lines = markdown.split(/\r?\n/);
  let unit: string[] = [];

  const flushUnit = (): void => {
    if (unit.length === 0) return;
    const text = unit.join(" ");
    unit = [];
    for (const part of splitNarrationCues(text)) {
      if ("cue" in part) {
        if (part.cue.kind === "prompt") {
          segments.push({
            ...part.cue,
            instruction: pairRomanization(part.cue.instruction, options.pairs),
          });
        } else {
          segments.push(part.cue);
        }
        continue;
      }
      const spoken = endSentence(pairRomanization(speakableInline(part.text), options.pairs));
      if (spoken !== "") segments.push({ kind: "speech", text: spoken });
    }
  };

  // A blockquote runs over several `>` lines and is ONE utterance. Without this flag
  // each line became its own "sentence", so `> Why flag this so hard? Because the
  // whole method of this book is building` came out as "…is building." followed by
  // "on real family resemblances…" — a paragraph chopped at the page's line breaks
  // rather than at its own clauses, which is unlistenable.
  let inQuote = false;
  let index = 0;
  while (index < lines.length) {
    const line = lines[index] ?? "";

    // Tables are looked for *inside* blockquotes too. The German chapter-1 sound-shift
    // aside is a real table nested in a `>` block, and testing the raw line meant it
    // never looked like a table at all — the rows joined the surrounding paragraph and
    // the learner heard "pipe English pipe German pipe dash dash dash". Peeling the
    // quote marker first costs nothing and makes the two cases identical.
    if (isTableRowLine(stripQuoteMarkers(line))) {
      flushUnit();
      inQuote = false;
      const start = index;
      while (index < lines.length && isTableRowLine(stripQuoteMarkers(lines[index] ?? ""))) {
        index += 1;
      }
      const chunk = lines
        .slice(start, index)
        .map(stripQuoteMarkers)
        .join("\n");
      const table = findMarkdownTables(chunk)[0];
      if (table) {
        segments.push(narrateTable(table, options));
      }
      continue;
    }

    if (line.trim() === "") {
      flushUnit();
      inQuote = false;
      index += 1;
      continue;
    }

    const isQuoteLine = line.trimStart().startsWith(">");
    if (startsNewUnit(line) && !(isQuoteLine && inQuote)) flushUnit();
    inQuote = isQuoteLine;
    const stripped = stripLinePrefix(line);
    if (stripped !== "") unit.push(stripped);
    index += 1;
  }
  flushUnit();
  return segments;
}

function narrateTable(
  table: ReturnType<typeof findMarkdownTables>[number],
  options: { maxColumns: number; pairs: readonly RomanizationPair[] },
): NarrationTable | NarrationTableSkipped {
  const result = linariseTable(table, { maxColumns: options.maxColumns });
  if (result.ok) {
    return {
      kind: "table",
      headers: result.headers,
      columns: result.columns,
      rowCount: result.rowCount,
      utterances: result.utterances.map((line) => pairRomanization(line, options.pairs)),
    };
  }
  // Naming the columns is the whole point of this segment: it is what turns "you
  // missed something" into "you missed the Spanish twin of each Italian verb". An
  // unlabelled column still gets said, as "one with no heading", because silence
  // there would undercount what the learner has to come back to.
  const named = result.headers.map((heading) =>
    heading.trim() === "" ? "one with no heading" : heading,
  );
  const headings = named.length > 0 ? ` Its columns are: ${named.join(", ")}.` : "";
  const size =
    result.rowCount > 0
      ? `${result.columns} columns and ${result.rowCount} rows`
      : `${result.columns} columns`;
  return {
    kind: "table-skipped",
    reason: result.reason,
    columns: result.columns,
    rowCount: result.rowCount,
    headers: result.headers,
    text:
      `There is a table here I cannot read to you — ${size}, and ` +
      `${TABLE_REFUSAL_MESSAGES[result.reason]}.${headings} ` +
      `Come back and look at it when you have stopped.`,
  };
}

/** The needs sentence fragments, one per rule that made this lesson non-drivable. */
function noticeNeeds(entry: LessonModality, skipped: NarrationTableSkipped[]): string[] {
  const needs: string[] = [];
  for (const reason of entry.reasons) {
    if (reason === "writing-type") needs.push("a pen and something to write on");
    if (reason === "script-block") needs.push("your eyes, for letter shapes on the page");
    if (reason === "sight-cue") {
      needs.push("your eyes, because the lesson points at something written down");
    }
    if (reason === "wide-table") {
      needs.push(
        skipped.length === 1
          ? "your eyes for one table that cannot be read aloud"
          : `your eyes for ${skipped.length || "some"} tables that cannot be read aloud`,
      );
    }
  }
  return needs;
}

function buildNotice(
  entry: LessonModality,
  blocks: NarrationBlock[],
): NarrationNotice | null {
  if (entry.modality === "voice") return null;
  const skipped = blocks.flatMap((block) =>
    block.segments.filter((segment): segment is NarrationTableSkipped =>
      segment.kind === "table-skipped",
    ),
  );
  const waitUntilStopped = blocks
    .filter((block) =>
      block.type === "script" ||
      block.segments.some(
        (segment) =>
          segment.kind === "table-skipped" ||
          (segment.kind === "prompt" && !segment.spoken),
      ),
    )
    .map((block) => block.title)
    .filter((title) => title !== "");
  const needs = noticeNeeds(entry, skipped);

  const opening =
    entry.modality === "pen"
      ? "Before we start: this one needs your hands, so it is not a driving lesson."
      : "Before we start: this one needs your eyes, so it is not fully a driving lesson.";
  const needsSentence =
    needs.length > 0 ? ` You will want ${joinList(needs)}.` : "";
  const sections = waitUntilStopped.length === 1 ? "the section" : "the sections";
  const skipSentence =
    waitUntilStopped.length > 0
      ? ` You can listen to everything else now — leave ${sections} called ${joinList(waitUntilStopped)} until you have stopped, and I will say so again when we reach it.`
      : " You can listen to all of it now and come back to the parts that need looking at once you have stopped.";

  return {
    modality: entry.modality,
    needs,
    waitUntilStopped,
    text: collapseSpaces(`${opening}${needsSentence}${skipSentence}`),
  };
}

/** `"a, b and c"` — an Oxford-free list, because it is being spoken, not printed. */
function joinList(items: readonly string[]): string {
  if (items.length === 0) return "";
  if (items.length === 1) return items[0] as string;
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/**
 * Narrate one lesson.
 *
 * Pure: filesystem, configuration, and output paths all live in `narration-cli.ts`.
 */
export function narrateLesson(
  lesson: ParsedLesson,
  options: NarrationOptions = {},
): LessonNarration {
  const maxColumns = options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  const entry = deriveLessonModality(lesson, { maxLinearisableTableColumns: maxColumns });
  const pairs: RomanizationPair[] = [
    ...(options.glossary ?? []),
    { headword: lesson.realization.headword, romanization: lesson.realization.romanization },
  ].filter(
    (pair) =>
      pair.headword.trim() !== "" &&
      pair.romanization.trim() !== "" &&
      pair.headword !== pair.romanization,
  );
  const markdownOptions = { maxColumns, pairs };

  const findings: NarrationFinding[] = [];
  const blocks: NarrationBlock[] = [];

  // Anything before the first `## ` heading that is not the title line is still
  // teaching material, so it becomes an untitled opening block rather than vanishing.
  const preambleBody = lesson.preamble
    .split(/\r?\n/)
    .filter((line) => !line.trimStart().startsWith("# "))
    .join("\n");
  const preambleSegments = narrateMarkdown(preambleBody, markdownOptions);
  if (preambleSegments.length > 0) {
    blocks.push({ index: -1, type: "unknown", title: "", segments: preambleSegments });
  }

  lesson.blocks.forEach((block, index) => {
    const segments = narrateMarkdown(block.markdown, markdownOptions);
    for (const activity of block.activities ?? []) {
      try {
        const compiled: CompiledLessonActivity = compileLessonActivity(activity, block, index);
        segments.push({
          kind: "activity",
          scored: true,
          id: compiled.id,
          prompt: pairRomanization(speakableInline(compiled.prompt), pairs),
          assesses: compiled.assesses,
          responseSeconds: compiled.responseSeconds,
          acceptedResponses: compiled.acceptedResponses,
          feedback: {
            correct: speakableInline(compiled.feedback.correct),
            incorrect: speakableInline(compiled.feedback.incorrect),
          },
        });
      } catch (error) {
        // An invalid contract is the validator's problem, not the narrator's. Report
        // it and keep going: refusing to narrate 1,096 lessons because one author
        // typo'd a `response_seconds` would be a worse outcome than a missing question.
        findings.push({
          code: "narration-activity-invalid",
          lessonId: lesson.realization.lessonId,
          language: lesson.language,
          message: `${lesson.realization.lessonId}: ${
            error instanceof Error ? error.message : String(error)
          }`,
        });
      }
    }
    blocks.push({
      index,
      type: block.type,
      title: speakableInline(block.title),
      segments,
    });
  });

  const skipped = blocks.flatMap((block) =>
    block.segments.filter((segment) => segment.kind === "table-skipped"),
  );
  if (skipped.length > 0 && entry.modality === "voice") {
    findings.push({
      code: "narration-block-unrenderable",
      lessonId: lesson.realization.lessonId,
      language: lesson.language,
      message:
        `${lesson.realization.lessonId}: ${skipped.length} table(s) cannot be spoken ` +
        `but the lesson is marked '${entry.modality}'`,
    });
  }

  return {
    lessonId: lesson.realization.lessonId,
    language: lesson.language,
    chapter: entry.chapter,
    sequence: numberOrNull(stringValue(lesson.frontmatter.sequence)),
    title: pairRomanization(narrationTitle(lesson), pairs),
    headword: lesson.realization.headword,
    romanization: lesson.realization.romanization,
    gloss: speakableInline(lesson.realization.gloss),
    script: lesson.script,
    modality: entry.modality,
    derivedModality: entry.derived,
    modalityReasons: entry.reasons,
    sourceHash: lesson.sourceHash,
    notice: buildNotice(entry, blocks),
    blocks,
    findings,
  };
}

// ---------------------------------------------------------------------------
// Narrating a chapter
// ---------------------------------------------------------------------------

/**
 * Narrate one chapter's lessons in authored order.
 *
 * The glossary is assembled first, from every lesson in the chapter, so a lesson may
 * pair a word that a *neighbouring* lesson introduced — see {@link pairRomanization}.
 */
export function narrateChapter(
  language: string,
  chapter: number,
  lessons: readonly ParsedLesson[],
  options: NarrationOptions = {},
): ChapterNarration {
  const glossary: RomanizationPair[] = [
    ...(options.glossary ?? []),
    ...lessons.map((lesson) => ({
      headword: lesson.realization.headword,
      romanization: lesson.realization.romanization,
    })),
  ];
  const ordered = orderLessons(lessons, options);
  const narrated = ordered.map((lesson) => narrateLesson(lesson, { ...options, glossary }));
  let drivablePrefix = 0;
  for (const lesson of narrated) {
    if (lesson.modality !== "voice") break;
    drivablePrefix += 1;
  }
  return {
    language,
    chapter,
    title: options.chapterTitle ?? `Chapter ${chapter}`,
    drivablePrefix,
    lessonIds: narrated.map((lesson) => lesson.lessonId),
    sourceHash: canonicalChapterHash([...ordered]),
    lessons: narrated,
    findings: narrated.flatMap((lesson) => lesson.findings),
  };
}

/**
 * Authored order, borrowed from `modality.ts` so the drivable prefix a learner is
 * told and the order they actually hear are computed the same way.
 */
function orderLessons(
  lessons: readonly ParsedLesson[],
  options: NarrationOptions,
): ParsedLesson[] {
  const width = options.maxLinearisableTableColumns;
  const byId = new Map<string, ParsedLesson>();
  const entries: LessonModality[] = [];
  for (const lesson of lessons) {
    byId.set(lesson.realization.lessonId, lesson);
    entries.push(
      deriveLessonModality(lesson, width === undefined ? {} : { maxLinearisableTableColumns: width }),
    );
  }
  return orderChapterLessons(entries)
    .map((entry) => byId.get(entry.lessonId))
    .filter((lesson): lesson is ParsedLesson => lesson !== undefined);
}

// ---------------------------------------------------------------------------
// The plain-text script
// ---------------------------------------------------------------------------

function plural(count: number, singular: string, many = `${singular}s`): string {
  return count === 1 ? `1 ${singular}` : `${count} ${many}`;
}

function pauseLine(segment: NarrationPause): string {
  const seconds = plural(segment.seconds, "second");
  return segment.perItem ? `[pause ${seconds} after each]` : `[pause ${seconds}]`;
}

function repeatLine(segment: NarrationRepeat): string {
  return segment.times === 2 ? "[repeat that twice]" : `[repeat that ${segment.times} times]`;
}

function promptLine(segment: NarrationPrompt): string[] {
  const verb = segment.action.toLowerCase();
  if (!segment.spoken) {
    return [`[once you have stopped driving — ${verb}: ${segment.instruction}]`];
  }
  return [
    `[your turn — ${verb}: ${segment.instruction}]`,
    `[pause ${plural(segment.responseSeconds, "second")} for the answer]`,
  ];
}

function activityLines(segment: NarrationActivity): string[] {
  return [
    `[question — say your answer, then pause ${plural(segment.responseSeconds, "second")}]`,
    segment.prompt,
  ];
}

function segmentLines(segment: NarrationSegment): string[] {
  switch (segment.kind) {
    case "speech":
      return [segment.text];
    case "pause":
      return [pauseLine(segment)];
    case "repeat":
      return [repeatLine(segment)];
    case "prompt":
      return promptLine(segment);
    case "table":
      return [
        `[a table of ${plural(segment.rowCount, "row")}, read aloud]`,
        ...segment.utterances,
      ];
    case "table-skipped":
      return [segment.text];
    case "activity":
      return activityLines(segment);
  }
}

/** One lesson as a continuous script. */
export function renderLessonNarrationText(lesson: LessonNarration): string {
  // The `# …` line is already written as "headword — gloss", and by the time it gets
  // here `narrateLesson` has paired its romanization. So it is the whole opening: a
  // second "headword — gloss" line underneath it said the same thing twice, which
  // sounds like a stutter rather than emphasis. The gloss is only added when the
  // title carries no dash and therefore is not already a definition.
  const heading = lesson.title === "" ? lesson.headword : lesson.title;
  const opening =
    heading.includes("—") || heading.includes(" - ") || lesson.gloss === ""
      ? heading
      : `${heading} — ${lesson.gloss}`;
  const lines: string[] = [];
  if (opening.trim() !== "") lines.push(endSentence(opening));
  if (lesson.notice) lines.push("", lesson.notice.text);
  for (const block of lesson.blocks) {
    if (block.segments.length === 0) continue;
    lines.push("");
    if (block.title !== "") lines.push(endSentence(block.title));
    for (const segment of block.segments) lines.push(...segmentLines(segment));
  }
  return lines.join("\n");
}

/** A whole chapter as a continuous script, ready to hand to a voice assistant. */
export function renderChapterNarrationText(
  chapter: ChapterNarration,
  languageName?: string,
): string {
  const track = languageName ?? chapter.language;
  const count = chapter.lessons.length;
  const drivable =
    chapter.drivablePrefix === count
      ? `All ${count} can be done entirely by ear.`
      : chapter.drivablePrefix === 0
        ? "The first lesson already needs your eyes or your hands, so save this one for when you have stopped."
        : `You can do the first ${chapter.drivablePrefix} of them in the car; after that you will want to have stopped.`;
  const lines: string[] = [
    `${titleCase(track)}, chapter ${chapter.chapter}: ${chapter.title}.`,
    `${plural(count, "lesson")}. ${drivable}`,
  ];
  chapter.lessons.forEach((lesson, index) => {
    lines.push("", "", `Lesson ${index + 1} of ${count}.`, renderLessonNarrationText(lesson));
  });
  return `${lines.join("\n")}\n`;
}

function titleCase(value: string): string {
  return value.length === 0 ? value : value[0]?.toUpperCase() + value.slice(1);
}

// ---------------------------------------------------------------------------
// Grouping a corpus
// ---------------------------------------------------------------------------

/**
 * Group a whole corpus into narratable chapters.
 *
 * Lessons whose `chapter` did not parse are dropped from chapter grouping the same
 * way `modality.ts` drops them — but unlike modality, nothing here can silently lose
 * a lesson, because a lesson with no chapter has no book chapter either and is
 * already visible as debt in the gap report.
 */
export function narrationChapters(
  lessons: readonly ParsedLesson[],
  options: NarrationOptions & {
    /** Chapter titles, keyed `"<language>/<chapter>"`. */
    titles?: ReadonlyMap<string, string>;
  } = {},
): ChapterNarration[] {
  const groups = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    const chapter = lesson.realization.chapter;
    if (!Number.isFinite(chapter)) continue;
    const key = `${lesson.language}/${chapter}`;
    const bucket = groups.get(key);
    if (bucket) bucket.push(lesson);
    else groups.set(key, [lesson]);
  }
  const chapters: ChapterNarration[] = [];
  for (const key of [...groups.keys()].sort(compareChapterKeys)) {
    const bucket = groups.get(key) ?? [];
    const first = bucket[0];
    if (!first) continue;
    const chapterNumber = first.realization.chapter;
    const title = options.titles?.get(key);
    chapters.push(
      narrateChapter(first.language, chapterNumber, bucket, {
        ...options,
        ...(title === undefined ? {} : { chapterTitle: title }),
      }),
    );
  }
  return chapters;
}

function compareChapterKeys(left: string, right: string): number {
  const [leftLanguage = "", leftChapter = ""] = left.split("/");
  const [rightLanguage = "", rightChapter = ""] = right.split("/");
  return (
    leftLanguage.localeCompare(rightLanguage) || Number(leftChapter) - Number(rightChapter)
  );
}
