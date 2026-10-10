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
import { joinQualifier } from "./delivery-cue.js";
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
import {
  splitNarrationCues,
  type NarrationPause,
  type NarrationPrompt,
  type NarrationRepeat,
} from "./narration-cues.js";

// The cue vocabulary and the paragraph splitter moved to `narration-cues.ts` so the
// modality rules can read a lesson exactly as this narrator does without importing
// it (see that file's header). Re-exported here so every existing import keeps
// working: this module is still where a consumer asks what a cue sounds like.
export {
  isManualCueAction,
  MANUAL_CUE_ACTIONS,
  parseNarrationCue,
  PROMPT_RESPONSE_SECONDS,
  SPOKEN_CUE_ACTIONS,
  splitNarrationCues,
} from "./narration-cues.js";
export type {
  CueSplit,
  NarrationCue,
  NarrationPause,
  NarrationPrompt,
  NarrationRepeat,
} from "./narration-cues.js";

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
  /**
   * What the lesson needs once its detachable sections (writing, inline letters) are
   * set aside — HL08's `coreModality`. A lesson whose core is `voice` can be done in
   * the car even when `modality` is `pen` or `sight`: its hands-on part is read in
   * place, announced in the notice, and left until the driver has stopped.
   */
  coreModality: Modality;
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
  /**
   * How many lessons, from the front, are drivable — HL08's number for a commuter.
   *
   * Counted on each lesson's CORE modality, the same definition `modality.ts`
   * (`drivablePrefix`) and the book's "Hands-free start" line use. A lesson that is
   * `voice` apart from a detachable writing or letters section counts: the narration
   * reads that section in place and its notice tells the driver to leave it until
   * they have stopped, so the rest of the lesson is a car lesson.
   */
  drivablePrefix: number;
  /**
   * How many of those {@link drivablePrefix} lessons carry such a part to come back
   * to — full modality `pen` or `sight`, core `voice`. Zero when every counted lesson
   * is voice through and through. Kept so the chapter header can say so honestly.
   */
  prefixWithPartsSetAside: number;
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

function isDigit(character: string | undefined): boolean {
  return character !== undefined && character >= "0" && character <= "9";
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

/**
 * The needs sentence fragments, one per rule that made this lesson non-drivable.
 *
 * One exception to "one per rule": a reading lesson names its eyes ONCE. Each eye
 * rule used to contribute its own "your eyes, …" fragment, and a reading lesson that
 * also tripped the cue detector came out as "your eyes, to read the printed text
 * yourself and your eyes, because the lesson points at something written down" — the
 * same organ twice, the second time for a reason the first already gave (the thing
 * the cue points at IS the printed text). So for a reading lesson:
 *
 *   reason          fragment
 *   --------------  -----------------------------------------------------------
 *   reading-type    "your eyes, to read the printed text yourself"   (the head)
 *   sight-cue       dropped — the cue points at the text being read
 *   script-block    folded in: "… and for letter shapes on the page"
 *   wide-table      folded in: "… and for one table that cannot be read aloud"
 *
 * Every other lesson is unchanged, so this moves no notice outside the reading type.
 */
function noticeNeeds(entry: LessonModality, skipped: NarrationTableSkipped[]): string[] {
  const tables =
    skipped.length === 1
      ? "one table that cannot be read aloud"
      : `${skipped.length || "some"} tables that cannot be read aloud`;
  if (entry.reasons.includes("reading-type")) {
    const also: string[] = [];
    if (entry.reasons.includes("script-block")) also.push("for letter shapes on the page");
    if (entry.reasons.includes("wide-table")) also.push(`for ${tables}`);
    // No `writing-type` to add: a lesson has one type, and this one is `reading`.
    return [`your eyes, ${joinList(["to read the printed text yourself", ...also])}`];
  }
  const needs: string[] = [];
  for (const reason of entry.reasons) {
    if (reason === "writing-type") needs.push("a pen and something to write on");
    if (reason === "script-block") needs.push("your eyes, for letter shapes on the page");
    if (reason === "sight-cue") {
      needs.push("your eyes, because the lesson points at something written down");
    }
    if (reason === "wide-table") needs.push(`your eyes for ${tables}`);
    // Said in the listener's terms rather than the rule's: what the step asks them to
    // do. The sections that carry such a step are named in the notice's
    // `waitUntilStopped` list, and the stop guard is spoken again when each begins.
    if (reason === "eyes-or-hands-step") {
      needs.push("your eyes or your hands for the steps that ask you to read, point, write or gesture");
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
  // A lesson whose CORE is voice is a car lesson with a part to come back to: the
  // chapter header counts it as drivable, so its notice must not call it "not a
  // driving lesson". It names every section that was set aside to reach that core
  // (a detachable block that is not itself voice), so the driver hears exactly which
  // part to leave — a `## Writing — …` block with no bracketed cue was otherwise
  // never named, and "come back to the parts that need looking at" pointed nowhere.
  const coreDrivable = entry.coreModality === "voice";
  const setAside = new Set(
    entry.blocks
      .filter((block) => block.detachable && block.modality !== "voice")
      .map((block) => block.index),
  );
  // A section with an eyes-or-hands step (see `eyesOrHandsSteps` in
  // drivable-instructions.ts) is named here too: its step is spoken plainly, so the
  // stop guard at its top is the only warning a driver hears at the moment it matters.
  // Only kept sections carry such a step (a detachable one is set aside whole), so a
  // lesson with one is never core voice; it is the section that made the lesson
  // undrivable, and the listener is told which.
  const askingForEyesOrHands = new Set(
    entry.blocks.filter((block) => block.eyesOrHandsSteps.length > 0).map((block) => block.index),
  );
  const waitUntilStopped = blocks
    .filter((block) =>
      block.type === "script" ||
      (coreDrivable && setAside.has(block.index)) ||
      askingForEyesOrHands.has(block.index) ||
      block.segments.some(
        (segment) =>
          segment.kind === "table-skipped" ||
          (segment.kind === "prompt" && !segment.spoken),
      ),
    )
    .map((block) => block.title)
    .filter((title) => title !== "");
  const needs = noticeNeeds(entry, skipped);

  // A reading lesson cannot be split into "listen now" and "come back later": the
  // modality rule that made it `sight` says so (the text is the whole lesson, and the
  // questions are about what was read), so its notice must not offer a split the rule
  // denies. It says what the chapter header says of a chapter that starts with one —
  // save it for when you have stopped — and drops the "not fully" hedge to match.
  //
  //   lesson                      opening
  //   --------------------------  -----------------------------------------------------
  //   core voice, full pen        "… you can do this one in the car, but part of it
  //                                needs your hands."
  //   core voice, full sight      "… you can do this one in the car, but part of it
  //                                needs your eyes."
  //   pen                         "… this one needs your hands, so it is not a driving
  //                                lesson."
  //   reading (sight)             "… this one needs your eyes, so it is not a driving
  //                                lesson."
  //   other sight                 "… this one needs your eyes, so it is not fully a
  //                                driving lesson."
  //
  // A reading lesson's core is never voice (the text is the lesson), so the first two
  // rows and the reading row cannot meet.
  const isReading = entry.reasons.includes("reading-type");
  const organ = entry.modality === "pen" ? "your hands" : "your eyes";
  const opening = coreDrivable
    ? `Before we start: you can do this one in the car, but part of it needs ${organ}.`
    : entry.modality === "pen"
      ? "Before we start: this one needs your hands, so it is not a driving lesson."
      : isReading
        ? "Before we start: this one needs your eyes, so it is not a driving lesson."
        : "Before we start: this one needs your eyes, so it is not fully a driving lesson.";
  const needsSentence =
    needs.length > 0
      ? coreDrivable
        ? ` For that part you will want ${joinList(needs)}.`
        : ` You will want ${joinList(needs)}.`
      : "";
  // The section titles go LAST, after a colon. Titles carry their own dashes, colons
  // and commas ("Writing — let your finger meet one shape", "The ten, in three
  // families"), and spoken in the middle of a sentence ("leave the section called
  // Writing — let your finger meet one shape until you have stopped") the listener
  // cannot hear where the title ends and the instruction resumes. At the end of the
  // sentence nothing follows them but a full stop.
  const plural = waitUntilStopped.length !== 1;
  const sections = plural ? "these sections" : "this section";
  const skipSentence = isReading
    ? " The questions are about what you read, so save the whole lesson for when you have stopped."
    : waitUntilStopped.length > 0
      ? ` You can listen to everything else now. I will say so again when we reach ${plural ? "them" : "it"}, but leave ${sections} until you have stopped: ${joinList(waitUntilStopped.map((title) => title.replace(/[.!?]+$/u, "")))}.`
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
    coreModality: entry.coreModality,
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
  // The core, not the full modality, gates the prefix — see `drivablePrefix` in
  // `modality.ts`, which this mirrors so the header, the modality report and the
  // book's "Hands-free start" line can never disagree about the same chapter.
  let drivablePrefix = 0;
  for (const lesson of narrated) {
    if (lesson.coreModality !== "voice") break;
    drivablePrefix += 1;
  }
  const prefixWithPartsSetAside = narrated
    .slice(0, drivablePrefix)
    .filter((lesson) => lesson.modality !== "voice").length;
  return {
    language,
    chapter,
    title: options.chapterTitle ?? `Chapter ${chapter}`,
    drivablePrefix,
    prefixWithPartsSetAside,
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
  const verb = joinQualifier(segment.action.toLowerCase(), segment.qualifier ?? "");
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

/**
 * Spoken at the top of every section a lesson's notice set aside for later. It is
 * bracketed like the deferred hands-on cues ("[once you have stopped driving — …]")
 * so a voice assistant and a reader hear the same instruction in the same shape.
 */
export const STOP_GUARD =
  "[once you have stopped driving — this part needs your eyes or your hands; if you are driving, skip ahead to the next part]";

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
  // The notice promises "I will say so again when we reach it" for every section it
  // names in `waitUntilStopped`. This keeps that promise: the moment such a section
  // begins, before a word of its content, the script says to leave it while driving.
  // Without it the hands-on instructions of a lesson the header now calls drivable
  // (a `## Writing — …` trace in lesson one) played straight through at the wheel.
  // Both lists carry the same `speakableInline` titles, so a plain match is exact.
  const setAside = new Set(lesson.notice?.waitUntilStopped ?? []);
  for (const block of lesson.blocks) {
    if (block.segments.length === 0) continue;
    lines.push("");
    if (block.title !== "") lines.push(endSentence(block.title));
    if (block.title !== "" && setAside.has(block.title)) lines.push(STOP_GUARD);
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
  // The second line counts the lessons and then says how far a driver gets. Every
  // branch is worded for the count it can see, because a sentence built for "many"
  // reads wrong at one and two. `k` is the drivable prefix, counted on each lesson's
  // CORE (see `narrateChapter`), and "entirely" is only said when none of those k
  // lessons has a part set aside (`prefixWithPartsSetAside`, d below, is 0):
  //
  //   lessons  drivable  says
  //   -------  --------  ------------------------------------------------------------
  //   1        1         "It can be done entirely by ear."      (not "All 1 can …")
  //   2        2         "Both can be done entirely by ear."    (not "All 2 can …")
  //   n > 2    n         "All n can be done entirely by ear."
  //   1        0         "It needs your eyes or your hands, so save it for …"
  //                       (there is no "first lesson" when there is only one)
  //   n > 1    0         "The first lesson already needs your eyes or your hands, …"
  //   n > 1    1         "You can do the first one in the car; …"
  //   n > 2    k > 1     "You can do the first k of them in the car; …"
  //
  // A lesson whose core is voice but which carries a writing or letters section
  // (`## Writing — observe and trace`) still counts: the narration reads that section
  // in place, and the lesson's own notice opens by saying the rest can be done in the
  // car and names the section to leave until the driver has stopped. The header
  // makes the same promise, in one more sentence, worded for d and k:
  //
  //   d      k       adds
  //   -----  ------  ------------------------------------------------------------------
  //   0      any     nothing — and the sentence above keeps "entirely"
  //   1      1, n=1  "It has a part that needs your eyes or your hands; that part …"
  //   1      1, n>1  "That lesson has a part that needs your eyes or your hands; …"
  //   1      2       "One of the two has a part that needs …; that part waits …"
  //   1      k > 2   "Of those k, one has a part that needs …; that part waits …"
  //   k      k > 1   "Each of those has a part that needs …; those parts wait …"
  //   1<d<k  k > 2   "Of those k, d have a part that needs …; those parts wait …"
  //
  // So Punjabi chapter 1, whose first lesson is a greeting plus a one-line
  // finger-trace of ਸ, now says "You can do the first 3 of them in the car; … Of
  // those 3, one has a part that needs your eyes or your hands; that part waits until
  // you have stopped." It used to tell a driver to stop before lesson one.
  const k = chapter.drivablePrefix;
  const d = chapter.prefixWithPartsSetAside;
  const byEar = d === 0 ? "entirely by ear" : "by ear";
  const drivable =
    k === count
      ? count === 1
        ? `It can be done ${byEar}.`
        : count === 2
          ? `Both can be done ${byEar}.`
          : `All ${count} can be done ${byEar}.`
      : k === 0
        ? count === 1
          ? "It needs your eyes or your hands, so save it for when you have stopped."
          : "The first lesson already needs your eyes or your hands, so save this one for when you have stopped."
        : k === 1
          ? "You can do the first one in the car; after that you will want to have stopped."
          : `You can do the first ${k} of them in the car; after that you will want to have stopped.`;
  const part = "a part that needs your eyes or your hands";
  const setAside =
    d === 0
      ? ""
      : k === 1
        ? count === 1
          ? ` It has ${part}; that part waits until you have stopped.`
          : ` That lesson has ${part}; that part waits until you have stopped.`
        : d === 1
          ? k === 2
            ? ` One of the two has ${part}; that part waits until you have stopped.`
            : ` Of those ${k}, one has ${part}; that part waits until you have stopped.`
          : d === k
            ? ` Each of those has ${part}; those parts wait until you have stopped.`
            : ` Of those ${k}, ${d} have ${part}; those parts wait until you have stopped.`;
  const lines: string[] = [
    `${titleCase(track)}, chapter ${chapter.chapter}: ${chapter.title}.`,
    `${plural(count, "lesson")}. ${drivable}${setAside}`,
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
