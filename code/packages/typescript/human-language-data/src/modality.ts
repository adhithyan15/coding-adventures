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
//   2. an ANCHORED sight cue in the prose     -> sight ("look at the chart above")
//   2. a table wider than we can read aloud   -> sight (a paradigm grid)
//   3. otherwise                              -> voice
//
// "Anchored" is doing real work in rule 2. A cue phrase is a pointing expression, and
// pointing expressions are only about the page when there is something on the page to
// point at; "Look at what English built on that jar" is an invitation to notice an
// etymology and "put a fact on the table" is an idiom. Both used to be counted, and
// authors were rewriting correct prose to get out from under the detector. See
// `SIGHT_CUE_RULES` for what each phrase has to clear before it counts.
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
 * How a cue phrase earns the right to be read as a reference to the page.
 *
 * A cue phrase is not evidence on its own. "Look at" is a **pointing expression**, and
 * a pointing expression only means "use your eyes" when there is something on the page
 * for it to point AT. The corpus is full of the same words aimed at nothing visible —
 * *"Look at what English built on that jar"*, *"put a fact on the table"*, *"la mesa —
 * 'the table'"* — and a detector that cannot tell those from *"look at the chart above"*
 * is measuring the vocabulary of the prose rather than the demands of the lesson.
 *
 * So each phrase declares what would have to be true for it to be a real reference:
 *
 *   `self-anchored`  the phrase carries its own deixis. "shown below" says *below*; there
 *                    is no figurative reading. It always counts.
 *
 *   `artifact`       the phrase is a DEFINITE reference to a whole named thing — *the
 *                    table*, *the chart*. Whether that thing exists is decidable: either
 *                    the document contains a table (or an image) or it does not. A
 *                    definite reference to an artifact the page does not contain is not a
 *                    reference to the page — it is a dining table, a gloss, or an idiom.
 *                    This direction is SAFE BY CONSTRUCTION: when a lesson holds no
 *                    table at all, no prose in it can send a reader to one.
 *
 *   `instruction`    the phrase is an instruction whose object decides the matter —
 *                    *look at*, *see the*, *column*. These keep firing by default and are
 *                    dropped only where the occurrence is demonstrably NOT an instruction
 *                    about the page (see {@link matchedSightCues}).
 *
 * `column` is deliberately an `instruction` and not an `artifact`, though it names part of
 * a table. "The table" is a whole artifact this module can look for; a *column* is a
 * part-noun that an author may reasonably use of any aligned display — a run of arrows, a
 * two-part list — that leaves no Markdown table behind for the structural check to find.
 * `ES-C56-cion` does exactly that ("Read those without the English column" over a
 * blockquote). Structure cannot adjudicate `column`, so it is never dropped on structural
 * grounds.
 *
 * The list is still a floor, not a proof: prose that needs eyes without saying so slips
 * through, which is why the table and script checks sit beside it rather than behind it.
 */
export type SightCueAnchor = "self-anchored" | "artifact" | "instruction";

/** One cue phrase and the condition under which it counts. */
export interface SightCueRule {
  /** Plain lowercase letters and spaces, so it compiles into a pattern unescaped. */
  readonly phrase: string;
  readonly anchor: SightCueAnchor;
}

export const SIGHT_CUE_RULES: readonly SightCueRule[] = [
  { phrase: "see the", anchor: "instruction" },
  { phrase: "look at", anchor: "instruction" },
  { phrase: "shown below", anchor: "self-anchored" },
  { phrase: "the table", anchor: "artifact" },
  { phrase: "the chart", anchor: "artifact" },
  { phrase: "column", anchor: "instruction" },
  { phrase: "written above", anchor: "self-anchored" },
];

/**
 * The cue phrases alone, in declaration order.
 *
 * Kept as a flat list because it is the package's published vocabulary and because the
 * tests pin its alphabet — every cue is plain lowercase letters and spaces, so it can go
 * straight into a pattern with no escaping, and a cue containing a regex metacharacter
 * would be a silent behaviour change.
 */
export const SIGHT_CUES: readonly string[] = SIGHT_CUE_RULES.map((rule) => rule.phrase);

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
  // The inline-letters section. HL00 makes it optional scaffolding by design — "a reader
  // who already knows the script skims that section" — and nothing later in the lesson
  // depends on having read it, so a hands-free renderer may set it aside and still teach
  // the word. The book prints it in full; only the driving edition skips it.
  "script",
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
  /**
   * Authored `delivery`, or null when the lesson does not declare one.
   *
   * This is the strand a lesson belongs to, declared rather than inferred. `type:
   * writing` already implies "script", but a consumer building a spoken-only edition
   * should not have to know that `type` doubles as a strand marker, nor re-parse the
   * Markdown to find out. Carried here so it reaches the manifest, which is what book
   * targets read.
   */
  delivery: string | null;
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
  /** Titles of the sections that teach the hand, in body order. Drive `pen`. */
  writingSegments: string[];
  /**
   * Titles of every section a hands-free renderer may set aside, in body order.
   *
   * A superset of {@link writingSegments}: an inline-letters `script` section is
   * detachable (HL00 calls it optional scaffolding a fluent reader skims) but teaches
   * no writing, so it belongs here and NOT there.
   */
  detachableSegments: string[];
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

/** A Markdown image — the only page artifact that is not a table. */
const MARKDOWN_IMAGE = /!\[[^\]]*\]\([^)\s]*/;

/**
 * Whether a document contains anything a cue phrase could be pointing AT.
 *
 * Exactly two things qualify, because exactly two things survive into the rendered page
 * as something a listener cannot receive: a Markdown table, and an image. (Corpus-wide
 * that is 633 lessons with a table and 1 with an image, so tables carry this rule almost
 * alone — but an image is the clearer case of the two, and leaving it out would make the
 * check wrong in the dangerous direction the day a figure lands.)
 */
export function hasPageArtifact(text: string): boolean {
  return widestTableColumns(text) > 0 || MARKDOWN_IMAGE.test(text);
}

/**
 * What the surrounding document offers a cue to point at.
 *
 * Passed in rather than recomputed so that a BLOCK is judged against the whole lesson.
 * A paragraph saying "look at the table" is pointing at a real table even when the table
 * itself lives in the next block, and a block-local check would call that figurative —
 * a false negative, which is the direction this module must never fail in.
 */
export interface SightCueContext {
  /** {@link hasPageArtifact} for the enclosing lesson. */
  hasPageArtifact: boolean;
}

/**
 * A wh-clause complement: the pointing expression's object is a PROPOSITION, not a thing.
 *
 * "Look at **what** English built on that jar", "Look at **what** is missing", "Look at
 * **how** the stress moves" — the reader is being asked to notice a fact, and the fact is
 * carried entirely by the sentences around it. Nothing is being indicated on the page,
 * so nothing is lost by hearing it instead of seeing it. Leading emphasis markers and an
 * opening quote are skipped because Markdown puts them between the verb and its object.
 */
const PROPOSITIONAL_COMPLEMENT = /^[\s*_“"'(]*(what|how|why|whether|where|when|who)\b/i;

/**
 * A short quoted run — a gloss or a citation rather than an instruction.
 *
 * Use versus mention. When a lesson writes the Indo-European root \*spek'- as “to look
 * at, to observe”, or glosses *la mesa* as "the table", the cue words are being QUOTED,
 * not addressed to the reader. The length cap is what keeps this from swallowing running
 * prose that merely happens to sit inside quotation marks: a gloss is a word or a phrase,
 * never a paragraph.
 */
const QUOTED_MENTION = /[“"']([^“”"'\n]{1,60})[”"']/g;

/**
 * Cue patterns, compiled once at module load.
 *
 * Compiled once rather than per call because cue matching runs over every block of every
 * lesson in the corpus. Each pattern is a single literal phrase between two boundary
 * classes — no alternation, no nested quantifier — so it is linear in the text and has no
 * backtracking behaviour to exploit.
 */
const CUE_PATTERNS: ReadonlyMap<string, RegExp> = new Map(
  SIGHT_CUE_RULES.map((rule) => [
    rule.phrase,
    // WORD BOUNDARIES, NOT BARE SUBSTRINGS. `haystack.includes(cue)` fired on any
    // occurrence anywhere: `column` matched inside `columns`, and `look at` matched the
    // gloss "to see, to look at" inside a vocabulary entry. A cue must stand as its own
    // words. Matched case-insensitively against the ORIGINAL text rather than against a
    // lowercased copy, because `toLowerCase()` is not length-preserving for every script
    // in this corpus and the offsets below have to index the real string.
    new RegExp(`(^|[^a-zA-Z])(${rule.phrase})([^a-zA-Z]|$)`, "gi"),
  ]),
);

/** Whether the cue at `[index, index + length)` sits inside a quoted gloss. */
function isQuotedMention(text: string, index: number, length: number): boolean {
  const lineStart = text.lastIndexOf("\n", index) + 1;
  const newline = text.indexOf("\n", index);
  const lineEnd = newline === -1 ? text.length : newline;
  const line = text.slice(lineStart, lineEnd);
  const offset = index - lineStart;
  for (const match of line.matchAll(QUOTED_MENTION)) {
    const start = match.index ?? 0;
    if (offset >= start && offset + length <= start + match[0].length) return true;
  }
  return false;
}

/**
 * Whether one cue phrase counts as a reference to the page anywhere in `text`.
 *
 * A cue fires when ANY of its occurrences survives; one genuine "look at the chart above"
 * is not cancelled by three figurative uses elsewhere in the same lesson.
 */
function cueFires(text: string, rule: SightCueRule, context: SightCueContext): boolean {
  // A definite reference to an artifact the document does not contain cannot be pointing
  // at the page. Nothing else in the lesson can change that, so answer before scanning.
  if (rule.anchor === "artifact" && !context.hasPageArtifact) return false;

  const pattern = CUE_PATTERNS.get(rule.phrase);
  if (pattern === undefined) return false;
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text)) !== null) {
    const index = (match.index ?? 0) + match[1]!.length;
    // Step one past the start rather than past the whole match: the trailing boundary
    // character is allowed to be the leading boundary of the next occurrence.
    pattern.lastIndex = index + 1;

    if (rule.anchor === "self-anchored") return true;
    if (isQuotedMention(text, index, rule.phrase.length)) continue;
    if (rule.anchor === "artifact") return true;
    if (PROPOSITIONAL_COMPLEMENT.test(text.slice(index + rule.phrase.length))) continue;
    return true;
  }
  return false;
}

/**
 * Which {@link SIGHT_CUES} genuinely point at the page, in list order.
 *
 * The bias here is asymmetric on purpose and has not changed: the cost of a false
 * NEGATIVE — a lesson wrongly advertised as drivable — is a driver told to look at
 * something at speed, while the cost of a false positive is one lesson missing from the
 * driving edition. So every rule above drops a cue only where the drop is defensible from
 * the document's own structure or from the grammar of the sentence, and where neither can
 * decide, the cue keeps firing. "Look at your collarbone" and "you can still see the
 * seams" are still counted, because no mechanical test separates them from "look at the
 * accent" without a lexicon of every thing a page can hold.
 *
 * @param context what the enclosing lesson offers a cue to point at. Defaults to reading
 *   `text` itself, which is right for a whole lesson and conservative for a fragment.
 */
export function matchedSightCues(text: string, context?: SightCueContext): string[] {
  const resolved = context ?? { hasPageArtifact: hasPageArtifact(text) };
  return SIGHT_CUE_RULES.filter((rule) => cueFires(text, rule, resolved)).map(
    (rule) => rule.phrase,
  );
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
  context?: SightCueContext,
): BlockModality {
  const maxColumns = options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  const text = `${block.title}\n${block.markdown}`;
  const reasons: ModalityReasonCode[] = [];

  const isWritingBlock = block.type === "writing";
  if (isWritingBlock) reasons.push("writing-block");
  if (block.type === "script") reasons.push("script-block");
  const cues = matchedSightCues(text, context);
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

  // One judgement about page artifacts for the WHOLE lesson, shared by every derivation
  // below. A cue in one block routinely points at a table in another — "look at the
  // table" then the table — so a block asked about its own text alone would call that
  // figurative and quietly drop a real requirement. The core derivation uses this same
  // lesson-level answer for the same reason: detaching a block changes what a renderer
  // reads out, not whether the author was pointing at something real.
  const cueContext: SightCueContext = { hasPageArtifact: hasPageArtifact(text) };

  const blocks = lesson.blocks.map((block, index) =>
    deriveBlockModality(block, index, options, cueContext),
  );

  // TWO DIFFERENT QUESTIONS, AND THEY WERE ONE VARIABLE UNTIL NOW.
  //
  //   writingSegments      sections that teach the HAND. They make a lesson `pen`.
  //   detachableSegments   sections a hands-free renderer may SET ASIDE. They make the
  //                        core differ from the whole, and say nothing about pen.
  //
  // Every writing segment is detachable, so while `writing` was the only detachable type
  // the two lists were identical and one variable served both. The moment a second type
  // became detachable the conflation bit: filtering on `detachable` and calling the
  // result `writingSegments` made every inline-letters section claim a lesson needs a pen
  // to READ a letter (`pen` 53 -> 309 corpus-wide, and 276 reported "writing segments"
  // that teach no writing at all). Detachability is about what a renderer may skip;
  // pen-ness is about what the learner's hand must do. They are now separate.
  const writingSegments = blocks
    .filter((block) => block.type === "writing")
    .map((block) => block.title);
  const detachableSegments = blocks.filter((block) => block.detachable).map((block) => block.title);

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
  const cues = matchedSightCues(text, cueContext);
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
  const coreCues = matchedSightCues(coreText, cueContext);
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
    delivery: stringValue(lesson.frontmatter.delivery) || null,
    derived,
    modality: accepted,
    coreModality: coreAccepted,
    coreDerived,
    coreReasons,
    blocks,
    writingSegments,
    detachableSegments,
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
