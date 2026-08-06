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

import type { ParsedLesson } from "./parse.js";

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
 * Zero, on purpose. HL08 sequences the work: the narration exporter that linearises
 * a two-column table into "*X* means *Y*" is migration step 3, and the table
 * remediation pass is step 4. Until the lineariser exists, claiming a table is
 * speakable would be claiming a capability nothing implements — and the failure mode
 * is the worst one available, a learner told a lesson is drivable who then silently
 * misses the content the table carried. So today every table means eyes, and this
 * number moves to 2 the day the lineariser lands.
 */
export const DEFAULT_LINEARISABLE_TABLE_COLUMNS = 0;

/** Tunables, so the width above can be raised without editing a call site. */
export interface ModalityOptions {
  /** Widest table still considered speakable. Default {@link DEFAULT_LINEARISABLE_TABLE_COLUMNS}. */
  maxLinearisableTableColumns?: number;
}

/** Why a lesson derived the modality it did — one entry per rule that fired. */
export type ModalityReasonCode =
  | "writing-type"
  | "script-block"
  | "sight-cue"
  | "wide-table"
  | "no-visual-dependency";

/**
 * A reported problem with an authored `modality:` override.
 *
 * Findings, not exceptions. This module follows the multi-pass habit the rest of the
 * package already has: walk the whole corpus, collect everything, report once. A
 * validator that throws on the first bad override tells an author about one mistake
 * and hides the other forty.
 */
export interface ModalityFinding {
  code: "modality-unknown-value" | "modality-unexplained-override";
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
  /** What the structure says the lesson needs. */
  derived: Modality;
  /** What the lesson actually claims — the override when one is accepted. */
  modality: Modality;
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
  /** Union of its lessons' requirements, weakest first. */
  modalities: Modality[];
  /**
   * How many lessons, **in authored order**, are `voice` before the first that is
   * not. "You can do the first six of this chapter's nine lessons in the car."
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
  /** Share of the track learnable by ear alone, rounded to a whole percent. */
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
 * Count the columns in a Markdown table row.
 *
 * A GFM row is cells fenced by pipes — `| a | b | c |` — so the cell count is the
 * number of pipe-separated fields with the empty leading/trailing fields dropped. A
 * pipe escaped as `\|` is content, not a fence, so it is skipped. Hand-scanned
 * rather than regex-split: the scan is one linear pass with no backtracking, and it
 * is the only place that has to understand escaping.
 *
 *   | word | gloss |          -> 2 columns   (speakable once the lineariser lands)
 *   | yo | tú | él | ella |   -> 4 columns   (a paradigm grid; eyes required)
 */
export function tableRowColumns(line: string): number {
  const fields: string[] = [];
  let current = "";
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (character === "\\" && line[index + 1] === "|") {
      current += "|";
      index += 1;
      continue;
    }
    if (character === "|") {
      fields.push(current);
      current = "";
      continue;
    }
    current += character;
  }
  fields.push(current);
  // A fenced row opens and closes with a pipe, producing an empty field at each
  // end. Those are the fence, not cells. An unfenced row (`a | b`) has neither.
  if (fields.length > 0 && (fields[0] ?? "").trim() === "") fields.shift();
  if (fields.length > 0 && (fields[fields.length - 1] ?? "").trim() === "") fields.pop();
  return fields.length;
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

/**
 * Derive one lesson's modality from its type and block structure.
 *
 * Pure, and deliberately ignorant of `skills:` — see the header. The rules are
 * applied in the order HL08 states them, and every rule that fires is recorded, so
 * an author fixing a `sight` lesson learns whether the culprit was the table, the
 * cue, or both.
 */
export function deriveLessonModality(
  lesson: ParsedLesson,
  options: ModalityOptions = {},
): LessonModality {
  const maxColumns = options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  const text = lessonText(lesson);
  const reasons: ModalityReasonCode[] = [];

  // Rule 1 — a writing lesson teaches the hand. Nothing else can override that
  // downward, so we do not even look at the body.
  const isWriting = lesson.realization.type === "writing";
  if (isWriting) reasons.push("writing-type");

  // Rule 2 — the three ways a lesson can need eyes. All three are evaluated even
  // once one has fired, because the report is more useful when it names every cause.
  const hasScriptBlock = lesson.blocks.some((block) => block.type === "script");
  if (hasScriptBlock) reasons.push("script-block");
  const cues = matchedSightCues(text);
  if (cues.length > 0) reasons.push("sight-cue");
  const tableColumns = widestTableColumns(text);
  const wideTable = tableColumns > maxColumns;
  if (wideTable) reasons.push("wide-table");

  const derived: Modality = isWriting
    ? "pen"
    : hasScriptBlock || cues.length > 0 || wideTable
      ? "sight"
      : "voice";
  // Rule 3 — nothing needed eyes or a hand, so it plays in the car.
  if (reasons.length === 0) reasons.push("no-visual-dependency");

  const authoredRaw = stringValue(lesson.frontmatter.modality).trim();
  const authored = authoredRaw === "" ? null : authoredRaw;
  const authoredReason = stringValue(lesson.frontmatter.modality_reason).trim();
  // An unusable authored value falls back to the derivation rather than poisoning the
  // corpus with an unknown channel; the finding below is how the author hears about it.
  const accepted = authored !== null && isModality(authored) ? authored : derived;

  return {
    lessonId: lesson.realization.lessonId,
    language: lesson.language,
    chapter: Number.isFinite(lesson.realization.chapter) ? lesson.realization.chapter : null,
    sequence: numberOrNull(stringValue(lesson.frontmatter.sequence)),
    derived,
    modality: accepted,
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
 * Counted from the front and stopped at the first lesson that is not `voice`. It is
 * deliberately NOT "how many voice lessons does this chapter contain" — the lessons
 * are prerequisite-ordered, so a `voice` lesson sitting behind a `sight` one is not
 * reachable in the car no matter how ear-friendly it is on its own.
 */
export function drivablePrefix(entries: readonly LessonModality[]): number {
  const ordered = orderChapterLessons(entries);
  let count = 0;
  for (const entry of ordered) {
    if (entry.modality !== "voice") break;
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
    tracks.push({
      language,
      lessonCount: trackEntries.length,
      voice,
      sight: trackEntries.filter((entry) => entry.modality === "sight").length,
      pen: trackEntries.filter((entry) => entry.modality === "pen").length,
      drivablePercent: percent(voice, trackEntries.length),
      drivablePrefixTotal: chapters.reduce((sum, chapter) => sum + chapter.drivablePrefix, 0),
      chapters,
    });
  }

  const voice = entries.filter((entry) => entry.modality === "voice").length;
  return {
    maxLinearisableTableColumns:
      options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS,
    totalLessons: entries.length,
    voice,
    sight: entries.filter((entry) => entry.modality === "sight").length,
    pen: entries.filter((entry) => entry.modality === "pen").length,
    drivablePercent: percent(voice, entries.length),
    tracks,
    findings,
  };
}
