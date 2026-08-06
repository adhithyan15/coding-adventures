// Modality — which channel a lesson actually needs (spec: code/specs/HL08-*).
//
// The question this module answers is deliberately concrete: **can a learner do
// this lesson while driving?** A voice assistant reading the curriculum aloud can
// teach anything that lives in sound. It cannot show you the shape of a Devanagari
// letter, it cannot point at the third column of a paradigm table, and it certainly
// cannot hold your pen. So every lesson gets one of three labels:
//
//   voice  🚗  learnable by ear alone — safe in the car
//   sight  👁  needs eyes — letter shapes, figures, or a table you cannot hear
//   pen    ✍  needs a hand — handwriting formation and practice
//
// ---------------------------------------------------------------------------
// Why this is NOT derived from `skills:`
// ---------------------------------------------------------------------------
//
// Every schema-v2 lesson already carries a `skills:` list, and the obvious move is
// to read modality straight off it: "this lesson lists `reading`, so it needs eyes."
// That obvious move is wrong, and wrong in a way that would have quietly ruined the
// whole feature.
//
// `skills` records what a lesson **develops**, not what it **requires**. 501 of the
// 531 schema-v2 lessons declare `[listening, speaking, reading]` — because a learner
// who works through them ends up able to read the word, not because they must be
// looking at anything to learn it. You can learn *hola* perfectly well by ear. Had
// modality been derived from `skills`, roughly 95% of the corpus would have been
// stamped "needs eyes" and the drivable course would have been an empty promise.
//
// Requirement is a property of the lesson's STRUCTURE, not of its ambitions. So the
// derivation below looks at the lesson's type and at its parsed body blocks:
//
//   1. `type: writing`                        -> pen   (an orthography drill)
//   2. a `script` block                       -> sight (letter shapes on the page)
//   2. a sight cue in the prose               -> sight ("look at the chart above")
//   2. a table wider than we can read aloud   -> sight (a paradigm grid)
//   3. otherwise                              -> voice
//
// Measured over all 1,096 lessons that yields 51 `pen`, and among the rest the
// single biggest obstacle is the **table**, not the script — which is a tractable
// problem, because a two-column word→gloss table linearises into speech fine
// ("*día* means day") while a five-column verb paradigm does not.
//
// ---------------------------------------------------------------------------
// One trap worth naming, because it nearly landed
// ---------------------------------------------------------------------------
//
// A parsed block keeps its display Markdown on `markdown`. There is no `content`
// field. Scanning `block.content` therefore reads `undefined` for every block, finds
// no tables and no cues anywhere, and reports a beautiful 100% drivable corpus. The
// detector looks like it works. It is measuring nothing. Every text accessor in this
// file goes through `lessonText()` below for exactly that reason.
//
// ---------------------------------------------------------------------------
// Amendment: one lesson, two modalities (HL08 §"Block-level modality")
// ---------------------------------------------------------------------------
//
// The original design gave each lesson exactly one modality. That is the right
// answer for the BOOK, which prints every block and needs one honest sign at the
// top of the chapter. It is the wrong answer for a lesson that is voice all the
// way through except for one short section teaching the hand how to form a letter
// — the interspersed writing pattern the project owner asked for, where a letter
// met earlier comes back for two minutes inside an ordinary lesson.
//
// Under one-modality-per-lesson those two minutes cost the whole lesson: it is
// stamped `pen`, and a listener loses all five minutes of it. That is a worse
// outcome than the truth, which is that four of those five minutes are perfectly
// listenable.
//
// So modality is now derived at TWO scales:
//
//   fullModality  what the lesson needs when every block is delivered — what the
//                 BOOK prints a sign for. Unchanged; still on `modality`.
//   coreModality  what the lesson needs once its DETACHABLE blocks are set aside.
//
// A `writing` block is the only detachable block type (see `LessonBlockType`).
// Detachable is a structural claim, not a value judgement: nothing later in the
// lesson reads back from the writing section, so a renderer that cannot use a
// hand may skip it and still deliver a coherent lesson. The book skips nothing.
//
// Two consequences worth stating plainly, because they are easy to get backwards:
//
//   • This is NOT a way to keep books drivable. The book is a standalone artifact
//     and keeps all writing content in full. A dictation-friendly edition is a
//     separate output view over the same canonical source, exactly like the
//     narration export; `coreModality` is the metadata that view will read.
//   • The drivable numbers are therefore about that future view's REACH, not
//     about book quality. If honest writing instruction lands in a track and its
//     full-modality `pen` count rises, that is the correct result, and no gate
//     here may push back on it.

import type { LessonBodyBlock } from "./types.js";
import type { ParsedLesson } from "./parse.js";
import {
  DEFAULT_LINEARISABLE_TABLE_COLUMNS as SPEAKABLE_TABLE_COLUMNS,
  hasUnspeakableTable,
  splitTableRow,
} from "./speech.js";

// ---------------------------------------------------------------------------
// The vocabulary
// ---------------------------------------------------------------------------

/** The channel a lesson requires. See the table at the top of this file. */
export type Modality = "voice" | "sight" | "pen";

/**
 * Ordered weakest-to-strongest. The order IS the monotonicity rule: `pen` implies
 * `sight`, because you cannot form a letter you are not looking at. Nothing implies
 * `voice` downward — a `sight` lesson is not "also a voice lesson", it is a lesson
 * you cannot do in the car.
 */
export const MODALITIES: readonly Modality[] = ["voice", "sight", "pen"];

/** The signs the book prints beside a chapter opening (HL08). */
export const MODALITY_SIGNS: Readonly<Record<Modality, string>> = {
  voice: "🚗",
  sight: "👁",
  pen: "✍",
};

/**
 * Phrases that give away a lesson pointing at something on the page.
 *
 * Plain lowercase substrings, matched with `String.includes` — deliberately **not**
 * regular expressions. Cue matching runs over every block of all 1,096 lessons, and
 * an author-supplied phrase list compiled into alternation-heavy patterns is exactly
 * the shape that turns into catastrophic backtracking on a long lesson body. Literal
 * substring search is linear in the text and cannot backtrack at all.
 *
 * The list is a floor, not a proof: prose that needs eyes without saying so slips
 * through, which is why the table and script checks sit beside it rather than behind
 * it.
 */
export const SIGHT_CUES: readonly string[] = [
  "see the",
  "look at",
  "shown below",
  "the table",
  "the chart",
  "column",
  "written above",
];

/**
 * How many columns a table may have and still be read aloud.
 *
 * This was **0** when modality first landed, on purpose: HL08 sequences the work, and
 * until the lineariser existed, claiming a table was speakable would have claimed a
 * capability nothing implemented. The failure mode there is the worst one available —
 * a learner told a lesson is drivable who then silently misses whatever the table
 * carried.
 *
 * The lineariser now exists (`speech.ts`), so the number is its measured default: see
 * `DEFAULT_LINEARISABLE_TABLE_COLUMNS` there for why three columns is the honest line
 * and four is not. Modality no longer decides this on its own — it asks the same
 * lineariser the narration export uses, so "drivable" and "actually narratable" can
 * never disagree.
 */
export const DEFAULT_LINEARISABLE_TABLE_COLUMNS = SPEAKABLE_TABLE_COLUMNS;

/** Tunables, so the width above can be raised without editing a call site. */
export interface ModalityOptions {
  /** Widest table still considered speakable. Default {@link DEFAULT_LINEARISABLE_TABLE_COLUMNS}. */
  maxLinearisableTableColumns?: number;
}

/**
 * Why a lesson derived the modality it did — one entry per rule that fired.
 *
 * `wide-table` is named for the common case but means the general one: *a table the
 * narration lineariser refuses*, whether because it is too wide, has a blank heading,
 * or has ragged rows.
 */
export type ModalityReasonCode =
  | "writing-type"
  | "writing-block"
  | "script-block"
  | "sight-cue"
  | "wide-table"
  | "no-visual-dependency";

/**
 * Block types a non-visual renderer may set aside without breaking the lesson.
 *
 * Exactly one member today, and the set is deliberately small: every addition is
 * a promise that nothing downstream in the lesson depends on the block, which is
 * a claim about content that only an author can make honestly. A block type in
 * here is *skippable by a renderer*, never *optional for a reader* — the book
 * prints all of them.
 */
export const DETACHABLE_BLOCK_TYPES: ReadonlySet<LessonBodyBlock["type"]> = new Set([
  "writing",
]);

/** Whether one parsed block may be set aside by a hands-free renderer. */
export function isDetachableBlock(block: LessonBodyBlock): boolean {
  return DETACHABLE_BLOCK_TYPES.has(block.type);
}

/** One block's own requirement, with the rules that produced it. */
export interface BlockModality {
  /** Position in `lesson.blocks`, so a report can point at the right section. */
  index: number;
  type: LessonBodyBlock["type"];
  title: string;
  modality: Modality;
  reasons: ModalityReasonCode[];
  /** True when a hands-free renderer may skip this block whole. */
  detachable: boolean;
}

/**
 * A reported problem with an authored `modality:` override.
 *
 * Findings, not exceptions. This module follows the multi-pass habit the rest of the
 * package already has: walk the whole corpus, collect everything, report once. A
 * validator that throws on the first bad override tells an author about one mistake
 * and hides the other forty.
 */
export interface ModalityFinding {
  code:
    | "modality-unknown-value"
    | "modality-unexplained-override"
    | "modality-writing-segment-not-separable";
  lessonId: string;
  language: string;
  message: string;
}

/** One lesson's modality, with the full derivation kept for the report. */
export interface LessonModality {
  lessonId: string;
  language: string;
  chapter: number | null;
  /** Authored `sequence`, or null on legacy lessons that carry none. */
  sequence: number | null;
  /** What the structure says the lesson needs, with every block delivered. */
  derived: Modality;
  /** What the lesson actually claims — the override when one is accepted. */
  modality: Modality;
  /**
   * What the lesson needs once its detachable writing segments are set aside.
   *
   * Never stronger than {@link modality}: setting a block aside can only lower a
   * requirement. Equal to `modality` for every lesson that carries no detachable
   * block, which today is almost all of them — so adding this field moved no
   * existing number.
   */
  coreModality: Modality;
  /** The structural derivation of {@link coreModality}, before any override. */
  coreDerived: Modality;
  /** Rules that fired for the core, in derivation order. */
  coreReasons: ModalityReasonCode[];
  /** Per-block requirements, in body order. */
  blocks: BlockModality[];
  /** Titles of the detachable writing segments, in body order. */
  writingSegments: string[];
  /** Rules that fired, in derivation order. */
  reasons: ModalityReasonCode[];
  /** Raw authored `modality:` value; null when the author left it derived. */
  authored: string | null;
  /** Authored `modality_reason:`; empty when absent. */
  authoredReason: string;
  /** True when an accepted authored value disagrees with the derivation. */
  overridden: boolean;
  /** Widest table found, in columns. 0 when the lesson has no table. */
  widestTableColumns: number;
  /** Which cue phrases matched, for the author fixing the lesson. */
  sightCues: string[];
  /** Monotone closure: `pen` also requires `sight`. */
  requires: Modality[];
}

/** One chapter's modality picture, including the number a commuter cares about. */
export interface ChapterModality {
  language: string;
  chapter: number;
  lessonCount: number;
  voice: number;
  sight: number;
  pen: number;
  /**
   * Lessons whose CORE is `voice` — listenable once detachable writing segments
   * are set aside. Never smaller than {@link voice}; the difference is exactly the
   * lessons a hands-free renderer rescues by skipping a section instead of a lesson.
   */
  coreVoice: number;
  /** Union of its lessons' requirements, weakest first. */
  modalities: Modality[];
  /**
   * How many lessons, **in authored order**, have a `voice` CORE before the first
   * that does not. "You can do the first six of this chapter's nine lessons in the
   * car." Counted on the core because that is what the hands-free view can deliver.
   */
  drivablePrefix: number;
  /** The lesson that ends the prefix, or null when the whole chapter is drivable. */
  firstNonVoiceLesson: string | null;
}

/** One track's rollup. */
export interface TrackModality {
  language: string;
  lessonCount: number;
  voice: number;
  sight: number;
  pen: number;
  /** Lessons whose CORE is `voice`. See {@link ChapterModality.coreVoice}. */
  coreVoice: number;
  /**
   * Share of the track a hands-free view can deliver, rounded to a whole percent.
   *
   * Computed from {@link coreVoice}, not {@link voice}: a lesson whose only
   * non-listenable part is a detachable writing segment is reachable, because the
   * view skips the segment rather than the lesson. The two numerators coincide
   * for every track with no detachable blocks.
   */
  drivablePercent: number;
  /** Sum of every chapter's drivable prefix — total lessons reachable in the car. */
  drivablePrefixTotal: number;
  chapters: ChapterModality[];
}

/** The whole corpus, ready to drop into the gap report. */
export interface ModalitySummary {
  maxLinearisableTableColumns: number;
  totalLessons: number;
  voice: number;
  sight: number;
  pen: number;
  /** Lessons whose CORE is `voice`. See {@link ChapterModality.coreVoice}. */
  coreVoice: number;
  /** Lessons carrying at least one detachable writing segment. */
  lessonsWithWritingSegments: number;
  /** Computed from {@link coreVoice}. See {@link TrackModality.drivablePercent}. */
  drivablePercent: number;
  tracks: TrackModality[];
  findings: ModalityFinding[];
}

// ---------------------------------------------------------------------------
// Small readers over frontmatter
// ---------------------------------------------------------------------------

function stringValue(value: ParsedLesson["frontmatter"][string] | undefined): string {
  return typeof value === "string" ? value : "";
}

function isModality(value: string): value is Modality {
  return (MODALITIES as readonly string[]).includes(value);
}

/** Rank in {@link MODALITIES}; -1 for anything unknown. */
export function modalityRank(modality: Modality): number {
  return MODALITIES.indexOf(modality);
}

/**
 * The monotone closure of one modality.
 *
 * `pen` implies `sight` — forming a letter means looking at it — so a pen lesson
 * requires both. Nothing implies `voice`: needing eyes does not make a lesson
 * ear-only, and reading the implication the other way would count sight lessons as
 * drivable, which is the exact lie this module exists to prevent.
 */
export function requiredChannels(modality: Modality): Modality[] {
  return modality === "pen" ? ["sight", "pen"] : [modality];
}

/** Union of several lessons' requirements, ordered weakest-first for printing. */
export function unionModalities(modalities: Iterable<Modality>): Modality[] {
  const seen = new Set<Modality>();
  for (const modality of modalities) {
    for (const channel of requiredChannels(modality)) seen.add(channel);
  }
  return MODALITIES.filter((candidate) => seen.has(candidate));
}

// ---------------------------------------------------------------------------
// Reading the lesson's own text
// ---------------------------------------------------------------------------

/**
 * Every scrap of learner-visible Markdown in a lesson, as one string.
 *
 * The canonical AST puts a block's display Markdown on `markdown` — see the trap
 * note at the top of this file. The preamble (normally the `# title` line) is
 * included because a table or a cue there counts exactly as much as one lower down.
 */
export function lessonText(lesson: ParsedLesson): string {
  const parts = [lesson.preamble];
  for (const block of lesson.blocks) parts.push(block.markdown);
  return parts.join("\n");
}

/**
 * The same text with every detachable block removed — what a hands-free renderer
 * would actually read out.
 *
 * The preamble stays, because a renderer that skips a writing section still reads
 * the title. When a lesson has no detachable block this returns exactly what
 * {@link lessonText} returns, which is why the amendment moved no existing number.
 */
export function lessonCoreText(lesson: ParsedLesson): string {
  const parts = [lesson.preamble];
  for (const block of lesson.blocks) {
    if (isDetachableBlock(block)) continue;
    parts.push(block.markdown);
  }
  return parts.join("\n");
}

/**
 * Count the columns in a Markdown table row.
 *
 * A GFM row is cells fenced by pipes — `| a | b | c |` — so the cell count is the
 * number of pipe-separated fields with the empty leading/trailing fields dropped.
 * The splitting itself lives in `speech.ts`, because the narration exporter needs the
 * *cells* and not merely how many there are; sharing the one scan keeps the count a
 * lesson is judged on identical to the cells it is later narrated from.
 *
 *   | word | gloss |          -> 2 columns   (speakable: "word means gloss")
 *   | yo | tú | él | ella |   -> 4 columns   (a paradigm grid; eyes required)
 */
export function tableRowColumns(line: string): number {
  return splitTableRow(line).length;
}

/** A line that opens a Markdown table row: up to three spaces, then a pipe. */
const TABLE_ROW = /^[ \t]{0,3}\|/;

/**
 * The widest table row in a lesson, in columns; 0 when there is no table.
 *
 * "Widest row" rather than "widest table" on purpose — a table is only as readable
 * aloud as its worst row, and a two-column table with one four-column header still
 * needs eyes.
 */
export function widestTableColumns(text: string): number {
  let widest = 0;
  for (const line of text.split(/\r?\n/)) {
    if (!TABLE_ROW.test(line)) continue;
    const columns = tableRowColumns(line);
    if (columns > widest) widest = columns;
  }
  return widest;
}

/** Which {@link SIGHT_CUES} appear in the text, in list order. */
export function matchedSightCues(text: string): string[] {
  const haystack = text.toLowerCase();
  return SIGHT_CUES.filter((cue) => haystack.includes(cue));
}

// ---------------------------------------------------------------------------
// The derivation
// ---------------------------------------------------------------------------

function numberOrNull(value: string): number | null {
  // `Number("")` is 0, not NaN — so an ABSENT `sequence` would otherwise read as
  // sequence 0 and sort ahead of every authored lesson. Check for empty first.
  if (value.trim() === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

/** The stronger of two requirements, by {@link MODALITIES} rank. */
export function strongerModality(left: Modality, right: Modality): Modality {
  return modalityRank(right) > modalityRank(left) ? right : left;
}

/** The weaker of two requirements — used to keep the core at or below the full. */
export function weakerModality(left: Modality, right: Modality): Modality {
  return modalityRank(right) < modalityRank(left) ? right : left;
}

/**
 * Derive ONE block's requirement.
 *
 * The same three sight rules the lesson uses, applied to this block's own title
 * and Markdown, plus one rule the lesson level cannot express: a `writing` block
 * teaches the hand, so it is `pen` whatever its prose looks like.
 *
 * The title is scanned as well as the body because a heading is learner-visible
 * copy — "Writing: మ — look at the tick" points at the page exactly as firmly
 * from a heading as from a paragraph.
 */
export function deriveBlockModality(
  block: LessonBodyBlock,
  index: number,
  options: ModalityOptions = {},
): BlockModality {
  const maxColumns = options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  const text = `${block.title}\n${block.markdown}`;
  const reasons: ModalityReasonCode[] = [];

  const isWritingBlock = block.type === "writing";
  if (isWritingBlock) reasons.push("writing-block");
  if (block.type === "script") reasons.push("script-block");
  const cues = matchedSightCues(text);
  if (cues.length > 0) reasons.push("sight-cue");
  const wideTable = widestTableColumns(text) > maxColumns;
  if (wideTable) reasons.push("wide-table");

  const modality: Modality = isWritingBlock
    ? "pen"
    : block.type === "script" || cues.length > 0 || wideTable
      ? "sight"
      : "voice";
  if (reasons.length === 0) reasons.push("no-visual-dependency");

  return {
    index,
    type: block.type,
    title: block.title,
    modality,
    reasons,
    detachable: isDetachableBlock(block),
  };
}

/**
 * Derive one lesson's modality from its type and block structure — at both scales.
 *
 * Pure, and deliberately ignorant of `skills:` — see the header. The rules are
 * applied in the order HL08 states them, and every rule that fires is recorded, so
 * an author fixing a `sight` lesson learns whether the culprit was the table, the
 * cue, or both.
 *
 * Two derivations run over the same rules and differ only in what they read:
 *
 *   full  the whole lesson — `lessonText`, every block. What the book signs.
 *   core  the lesson minus its detachable writing segments — `lessonCoreText`.
 *
 * A lesson with no detachable block feeds both derivations identical text and
 * gets identical answers, so this amendment is a no-op on the corpus as it stands.
 */
export function deriveLessonModality(
  lesson: ParsedLesson,
  options: ModalityOptions = {},
): LessonModality {
  const maxColumns = options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  const text = lessonText(lesson);
  const reasons: ModalityReasonCode[] = [];

  const blocks = lesson.blocks.map((block, index) => deriveBlockModality(block, index, options));
  const writingSegments = blocks.filter((block) => block.detachable).map((block) => block.title);

  // Rule 1 — a writing lesson teaches the hand. Nothing else can override that
  // downward, so we do not even look at the body. Note this is the lesson TYPE:
  // the whole lesson is the writing, so there is nothing separable to set aside,
  // and the core is `pen` too.
  const isWriting = lesson.realization.type === "writing";
  if (isWriting) reasons.push("writing-type");

  // Rule 1b — a writing BLOCK inside an ordinary lesson. Same requirement, but
  // confined to that block, which is what makes the core/full split meaningful.
  const hasWritingBlock = writingSegments.length > 0;
  if (hasWritingBlock) reasons.push("writing-block");

  // Rule 2 — the three ways a lesson can need eyes. All three are evaluated even
  // once one has fired, because the report is more useful when it names every cause.
  const hasScriptBlock = lesson.blocks.some((block) => block.type === "script");
  if (hasScriptBlock) reasons.push("script-block");
  const cues = matchedSightCues(text);
  if (cues.length > 0) reasons.push("sight-cue");
  const tableColumns = widestTableColumns(text);
  // Not "is it wider than the limit" but "would the lineariser refuse it" — a
  // strictly larger question. A three-column table sitting inside the limit is still
  // unspeakable when its rows are ragged or a heading is blank, and asking the
  // narration exporter's own judgement is the only way `voice` can be a promise the
  // export is able to keep.
  const wideTable = hasUnspeakableTable(text, { maxColumns });
  if (wideTable) reasons.push("wide-table");

  const derived: Modality =
    isWriting || hasWritingBlock
      ? "pen"
      : hasScriptBlock || cues.length > 0 || wideTable
        ? "sight"
        : "voice";
  // Rule 3 — nothing needed eyes or a hand, so it plays in the car.
  if (reasons.length === 0) reasons.push("no-visual-dependency");

  // The core: the same rules, over the text a hands-free renderer would read and
  // over the blocks it would keep.
  const coreReasons: ModalityReasonCode[] = [];
  if (isWriting) coreReasons.push("writing-type");
  const coreText = lessonCoreText(lesson);
  const coreScriptBlock = lesson.blocks.some(
    (block) => block.type === "script" && !isDetachableBlock(block),
  );
  if (coreScriptBlock) coreReasons.push("script-block");
  const coreCues = matchedSightCues(coreText);
  if (coreCues.length > 0) coreReasons.push("sight-cue");
  const coreWideTable = widestTableColumns(coreText) > maxColumns;
  if (coreWideTable) coreReasons.push("wide-table");
  const coreDerived: Modality = isWriting
    ? "pen"
    : coreScriptBlock || coreCues.length > 0 || coreWideTable
      ? "sight"
      : "voice";
  if (coreReasons.length === 0) coreReasons.push("no-visual-dependency");

  const authoredRaw = stringValue(lesson.frontmatter.modality).trim();
  const authored = authoredRaw === "" ? null : authoredRaw;
  const authoredReason = stringValue(lesson.frontmatter.modality_reason).trim();
  // An unusable authored value falls back to the derivation rather than poisoning the
  // corpus with an unknown channel; the finding below is how the author hears about it.
  const accepted = authored !== null && isModality(authored) ? authored : derived;
  // An override speaks for the lesson as a whole, so it also caps the core: an
  // author who says "this lesson is really only voice" cannot leave a core sitting
  // above the value they just accepted responsibility for. The invariant that
  // falls out — core is never stronger than full — is what lets a hands-free view
  // trust `coreModality` on its own.
  const coreAccepted = weakerModality(coreDerived, accepted);

  return {
    lessonId: lesson.realization.lessonId,
    language: lesson.language,
    chapter: Number.isFinite(lesson.realization.chapter) ? lesson.realization.chapter : null,
    sequence: numberOrNull(stringValue(lesson.frontmatter.sequence)),
    derived,
    modality: accepted,
    coreModality: coreAccepted,
    coreDerived,
    coreReasons,
    blocks,
    writingSegments,
    reasons,
    authored,
    authoredReason,
    overridden: accepted !== derived,
    widestTableColumns: tableColumns,
    sightCues: cues,
    requires: requiredChannels(accepted),
  };
}

/**
 * Findings for one lesson's authored override.
 *
 * The rule HL08 asks for: overriding is free, but an override that CONTRADICTS the
 * derivation has to say why. That keeps the common case (no annotation at all) cheap
 * while making the exceptional case auditable — an author who marks a table-bearing
 * lesson `voice` is asserting the table is decorative, and that assertion should be
 * written down where the next reader can check it.
 *
 * An override that merely agrees with the derivation needs no reason: it is a no-op.
 */
export function modalityFindings(entry: LessonModality): ModalityFinding[] {
  const findings: ModalityFinding[] = [];
  if (entry.authored !== null && !isModality(entry.authored)) {
    findings.push({
      code: "modality-unknown-value",
      lessonId: entry.lessonId,
      language: entry.language,
      message:
        `${entry.lessonId}: modality '${entry.authored}' is not one of ` +
        `${MODALITIES.join("/")}; using the derived '${entry.derived}'`,
    });
    return findings;
  }
  if (entry.overridden && entry.authoredReason === "") {
    findings.push({
      code: "modality-unexplained-override",
      lessonId: entry.lessonId,
      language: entry.language,
      message:
        `${entry.lessonId}: modality '${entry.modality}' contradicts the derived ` +
        `'${entry.derived}' (${entry.reasons.join(", ")}) and has no modality_reason`,
    });
  }
  // The separability contract. The interspersed pattern is "one ordinary lesson,
  // one short writing segment"; a lesson sprouting several writing sections has
  // stopped being an ordinary lesson with an aside and should be split, or
  // declared `type: writing` outright. Report-only, per the HL-V01 precedent.
  const isWritingLesson = entry.reasons.includes("writing-type");
  if (!isWritingLesson && entry.writingSegments.length > 1) {
    findings.push({
      code: "modality-writing-segment-not-separable",
      lessonId: entry.lessonId,
      language: entry.language,
      message:
        `${entry.lessonId}: carries ${entry.writingSegments.length} writing segments ` +
        `(${entry.writingSegments.join(" | ")}); an interspersed lesson may carry one, ` +
        "otherwise make it a type: writing lesson",
    });
  }
  return findings;
}

/** Derive modality for a whole corpus. Pure; ordering follows the input. */
export function lessonModalities(
  lessons: readonly ParsedLesson[],
  options: ModalityOptions = {},
): LessonModality[] {
  return lessons.map((lesson) => deriveLessonModality(lesson, options));
}

// ---------------------------------------------------------------------------
// Chapters and tracks
// ---------------------------------------------------------------------------

/**
 * Authored order within a chapter.
 *
 * Schema-v2 lessons carry an explicit `sequence`; legacy ones do not, and their
 * `sequence` is null. Sorting null-last by id keeps the walk deterministic without
 * inventing an order the author never wrote. This mirrors the comparator the book
 * generator already uses, so the drivable prefix counts lessons in the same order
 * the reader meets them on the page.
 */
export function orderChapterLessons(entries: readonly LessonModality[]): LessonModality[] {
  return [...entries].sort((left, right) => {
    const leftSequence = left.sequence ?? Number.POSITIVE_INFINITY;
    const rightSequence = right.sequence ?? Number.POSITIVE_INFINITY;
    if (leftSequence !== rightSequence) return leftSequence - rightSequence;
    return left.lessonId.localeCompare(right.lessonId);
  });
}

/**
 * How far into a chapter a commuting learner gets.
 *
 * Counted from the front and stopped at the first lesson whose CORE is not `voice`.
 * It is deliberately NOT "how many voice lessons does this chapter contain" — the
 * lessons are prerequisite-ordered, so a `voice` lesson sitting behind a `sight` one
 * is not reachable in the car no matter how ear-friendly it is on its own.
 *
 * The core, not the full modality, is the right gate here: the hands-free view sets
 * detachable writing segments aside and keeps going, so a lesson that is voice apart
 * from a two-minute writing aside does not end a commuter's run. The book, which
 * prints everything, is signed from the full modality instead.
 */
export function drivablePrefix(entries: readonly LessonModality[]): number {
  const ordered = orderChapterLessons(entries);
  let count = 0;
  for (const entry of ordered) {
    if (entry.coreModality !== "voice") break;
    count += 1;
  }
  return count;
}

function chapterModality(
  language: string,
  chapter: number,
  entries: readonly LessonModality[],
): ChapterModality {
  const ordered = orderChapterLessons(entries);
  const prefix = drivablePrefix(ordered);
  return {
    language,
    chapter,
    lessonCount: ordered.length,
    voice: ordered.filter((entry) => entry.modality === "voice").length,
    sight: ordered.filter((entry) => entry.modality === "sight").length,
    pen: ordered.filter((entry) => entry.modality === "pen").length,
    coreVoice: ordered.filter((entry) => entry.coreModality === "voice").length,
    modalities: unionModalities(ordered.map((entry) => entry.modality)),
    drivablePrefix: prefix,
    firstNonVoiceLesson: prefix < ordered.length ? (ordered[prefix]?.lessonId ?? null) : null,
  };
}

function percent(part: number, whole: number): number {
  return whole === 0 ? 0 : Math.round((part / whole) * 100);
}

/**
 * Roll derived lesson modalities up into tracks and chapters.
 *
 * Lessons whose `chapter` did not parse are still counted in their track's totals —
 * they exist and a learner still meets them — but they cannot belong to a chapter's
 * ordered prefix, so they are left out of the chapter list rather than silently
 * bucketed into a chapter 0 that no book prints.
 */
export function summarizeModality(
  lessons: readonly ParsedLesson[],
  options: ModalityOptions = {},
): ModalitySummary {
  const entries = lessonModalities(lessons, options);

  const findings: ModalityFinding[] = [];
  for (const entry of entries) findings.push(...modalityFindings(entry));
  findings.sort(
    (left, right) =>
      left.language.localeCompare(right.language) ||
      left.lessonId.localeCompare(right.lessonId) ||
      left.code.localeCompare(right.code),
  );

  const byLanguage = new Map<string, LessonModality[]>();
  for (const entry of entries) {
    const bucket = byLanguage.get(entry.language);
    if (bucket) bucket.push(entry);
    else byLanguage.set(entry.language, [entry]);
  }

  const tracks: TrackModality[] = [];
  for (const language of [...byLanguage.keys()].sort()) {
    const trackEntries = byLanguage.get(language) ?? [];
    const byChapter = new Map<number, LessonModality[]>();
    for (const entry of trackEntries) {
      if (entry.chapter === null) continue;
      const bucket = byChapter.get(entry.chapter);
      if (bucket) bucket.push(entry);
      else byChapter.set(entry.chapter, [entry]);
    }
    const chapters = [...byChapter.keys()]
      .sort((left, right) => left - right)
      .map((chapter) => chapterModality(language, chapter, byChapter.get(chapter) ?? []));
    const voice = trackEntries.filter((entry) => entry.modality === "voice").length;
    const coreVoice = trackEntries.filter((entry) => entry.coreModality === "voice").length;
    tracks.push({
      language,
      lessonCount: trackEntries.length,
      voice,
      sight: trackEntries.filter((entry) => entry.modality === "sight").length,
      pen: trackEntries.filter((entry) => entry.modality === "pen").length,
      coreVoice,
      drivablePercent: percent(coreVoice, trackEntries.length),
      drivablePrefixTotal: chapters.reduce((sum, chapter) => sum + chapter.drivablePrefix, 0),
      chapters,
    });
  }

  const voice = entries.filter((entry) => entry.modality === "voice").length;
  const coreVoice = entries.filter((entry) => entry.coreModality === "voice").length;
  return {
    maxLinearisableTableColumns:
      options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS,
    totalLessons: entries.length,
    voice,
    sight: entries.filter((entry) => entry.modality === "sight").length,
    pen: entries.filter((entry) => entry.modality === "pen").length,
    coreVoice,
    lessonsWithWritingSegments: entries.filter((entry) => entry.writingSegments.length > 0).length,
    drivablePercent: percent(coreVoice, entries.length),
    tracks,
    findings,
  };
}
