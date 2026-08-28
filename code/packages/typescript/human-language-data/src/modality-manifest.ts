// Modality manifest — the derived modality data, emitted as a file (spec: HL08).
//
// ---------------------------------------------------------------------------
// Why this file exists at all
// ---------------------------------------------------------------------------
//
// `modality.ts` already knows, for every one of the 1,096 lessons, whether it can be
// learned by ear in a car. But that knowledge was computed at runtime and printed into
// a human-readable gap report, and a paragraph of English is not something a book
// builder can filter on. The moment the project decided to ship **two editions from
// one source** —
//
//   * the complete book, which keeps everything, handwriting drills included; and
//   * a dictation-friendly driving edition, which keeps only what a driver can do —
//
// the derivation had to become *data*. This module is that data: independently
// mergeable per-language JSON shards, keyed by lesson, that any renderer can read and filter without importing
// TypeScript, without a Markdown parser, and without re-deriving anything.
//
// It is a **derived artifact**, in exactly the sense `core/generated-book-hashes/*.json`
// is. Nobody edits it by hand. The alternative — adding `modality:` to 1,096
// frontmatter files — was considered and deliberately rejected by HL08, because 1,096
// authored copies of a computed fact are 1,096 places for it to go stale. Authored
// overrides still exist for the genuinely exceptional lesson (`modality:` plus a
// `modality_reason:`), and this manifest records that an override happened rather than
// replacing the mechanism.
//
// ---------------------------------------------------------------------------
// The shape, and the one thing it must be able to grow
// ---------------------------------------------------------------------------
//
// Today modality is a property of a **whole lesson**: one label, and a driver either
// gets the lesson or does not. That is a blunt instrument, and everyone knows it. A
// lesson whose teaching is entirely spoken but which ends with a two-minute "now trace
// the letter" segment is stamped `pen`, and a driving edition drops the whole thing —
// the commuter loses eight minutes of perfectly audible content to be protected from
// two minutes they could simply skip.
//
// The fix (HL-C41, in flight as this lands) is **block-level modality**: a lesson gains
// a `coreModality` describing its main body, and the separable writing segment is
// marked as its own optional block. This manifest is designed so that lands as an
// ADDITION, never a rewrite:
//
//   * Every lesson row is a JSON **object**, not a positional tuple. New keys are free.
//   * `modality` keeps its current meaning permanently: the STRONGEST channel the
//     lesson needs anywhere in it. It is the safe, conservative filter. A consumer
//     that never learns about `coreModality` keeps producing a correct — merely
//     pessimistic — driving edition forever. That is the right direction to fail in:
//     a driver offered too little is never handed a pen at 70mph.
//   * `coreModality`, once HL-C41 defines it, is a NEW optional key beside it and is
//     by construction no stronger than `modality`. A consumer opts in by reading
//     `entry.coreModality ?? entry.modality`, which is correct before and after.
//   * `features.blockModality` in the header is a one-glance capability flag, so a
//     consumer branches on the FILE rather than probing 1,096 rows to discover
//     whether this build carries block data.
//
// The shape of `coreModality`'s companion block records is deliberately NOT guessed
// here. An invented field HL-C41 then has to contradict is worse than no field at all:
// an absent key is additive, a wrong key is a breaking change.

import { fnv1a64 } from "./hash.js";
import {
  DEFAULT_LINEARISABLE_TABLE_COLUMNS,
  deriveLessonModality,
  modalityFindings,
  orderChapterLessons,
  unionModalities,
  type LessonModality,
  type Modality,
  type ModalityFinding,
  type ModalityOptions,
  type ModalityReasonCode,
} from "./modality.js";
import type { ParsedLesson } from "./parse.js";

/** One independently mergeable manifest per language lives here. */
export const MODALITY_MANIFEST_DIR = "core/lesson-modality";

/**
 * Bumped only for a change no existing reader can survive.
 *
 * Adding a key is not such a change — see the header. This number moving means every
 * consumer must be updated in lockstep, so it should move approximately never.
 */
export const MODALITY_MANIFEST_VERSION = 1;

// ---------------------------------------------------------------------------
// The emitted types
// ---------------------------------------------------------------------------

/**
 * What this build of the manifest is able to say.
 *
 * A capability flag rather than a version bump, because block-level modality is
 * strictly *additional* information: a manifest with it and one without are both
 * version 1, and both answer "can a driver do this lesson?" correctly. The flag exists
 * so a consumer can ask that question of the file instead of inferring it from rows.
 */
export interface ModalityManifestFeatures {
  /**
   * True once per-block modality is emitted (HL-C41). While false, every lesson's
   * requirement is a single whole-lesson label and an edition filter can only include
   * or exclude whole lessons.
   */
  blockModality: boolean;
}

/** The tunables the numbers below were measured under, recorded so they are auditable. */
export interface ModalityManifestPolicy {
  /** Widest table still considered speakable. 0 until the narration lineariser lands. */
  maxLinearisableTableColumns: number;
}

/** One lesson, as the app and both book editions see it. */
export interface ModalityManifestLesson {
  id: string;
  language: string;
  /** Authored chapter, or null on a lesson whose chapter did not parse. */
  chapter: number | null;
  /** Authored `sequence`; null on legacy lessons that carry none. */
  sequence: number | null;
  /**
   * The strongest channel the lesson needs ANYWHERE in it — permanently the
   * conservative filter. See the header note on `coreModality`.
   */
  modality: Modality;
  /** What the structure alone said, before any authored override. */
  derived: Modality;
  /**
   * `modality === "voice"`. Precomputed so a filter is a field read, not a rule.
   *
   * This one denormalisation earns its bytes; the monotone closure (`pen` implies
   * `sight`) deliberately does NOT and is absent. It is a three-entry lookup table, so
   * emitting `["sight","pen"]` beside every pen lesson would add sixty kilobytes of
   * restating `requiredChannels()`, which this package exports for the handful of
   * consumers that want it.
   */
  drivable: boolean;
  /** Which derivation rules fired, in order — why this lesson is labelled as it is. */
  reasons: ModalityReasonCode[];
  /**
   * The channel the lesson needs once its DETACHABLE blocks are set aside — today the
   * inline-letters `script` section and any `writing` section. By construction never
   * stronger than {@link modality}.
   *
   * This is the key the header promised and the manifest never emitted, which is the
   * whole reason it is being added now. Ten authoring waves put honest
   * `## The letters in this word` sections into eighteen tracks; every one of those
   * lessons derives `modality: sight` and `coreModality: voice`, and with no
   * `coreModality` published, a consumer had no way to tell "needs eyes throughout"
   * from "needs eyes only for a section a fluent reader skims". Four separate tracks
   * reported their new chapters as undrivable on that basis, and two agents reached
   * opposite conclusions about the cause.
   *
   * Read it as `entry.coreModality ?? entry.modality`, which is correct against both
   * old and new builds — and branch on `features.blockModality` if you want to know
   * which you are holding.
   */
  coreModality: Modality;
  /** Rules that fired for the core, in derivation order. */
  coreReasons: ModalityReasonCode[];
  /**
   * `coreModality === "voice"` — the OPT-IN filter, beside the conservative `drivable`.
   *
   * Both are published on purpose. `drivable` stays exactly what it always was: the
   * strongest channel anywhere in the lesson, so a consumer that never learns about
   * detachable blocks keeps producing a correct, merely pessimistic driving edition
   * forever. `coreDrivable` is what a renderer that CAN set a section aside should
   * read. Nothing about `drivable` moves in this change; that is the compatibility
   * promise this file's header makes, and switching it would break every consumer
   * that trusted it.
   */
  coreDrivable: boolean;
  /**
   * Titles of the sections a hands-free renderer may set aside, in body order.
   *
   * Emitted only when non-empty. This is what makes `coreDrivable` auditable rather
   * than asserted: a reader can see WHICH sections were discounted and disagree.
   */
  detachableSegments?: string[];
  /**
   * The authored strand marker, present only when the lesson declares one.
   *
   * A spoken-only edition filters on this rather than inferring the strand from
   * `type` or from the computed modality, both of which mean something else.
   */
  delivery?: string;
  /** Fingerprint of the lesson AST this row was derived from. */
  sourceHash: string;
  /** Present only when the author wrote an explicit `modality:`. */
  authored?: string;
  /** Present only when the author wrote a `modality_reason:`. */
  authoredReason?: string;
  /** Present, and always true, only when an accepted override contradicts the derivation. */
  overridden?: boolean;
}

/** One chapter, including the number a commuting learner actually asks for. */
export interface ModalityManifestChapter {
  chapter: number;
  lessonCount: number;
  voice: number;
  sight: number;
  pen: number;
  /** Union of its lessons' requirements, weakest first. */
  modalities: Modality[];
  /** How many lessons, in authored order, are `voice` before the first that is not. */
  drivablePrefix: number;
  /** The lesson that ends the prefix, or null when the whole chapter is drivable. */
  firstNonVoiceLesson: string | null;
  /** True when every lesson in the chapter is `voice`. */
  drivable: boolean;
  /**
   * The prefix, spelled out in order.
   *
   * Redundant with `drivablePrefix` on purpose. A driving-edition renderer wants the
   * ids, and making it re-sort the lesson list to recover them is precisely how two
   * implementations of "authored order" end up quietly disagreeing.
   */
  drivableLessonIds: string[];
}

/** One track's rollup. */
export interface ModalityManifestTrack {
  language: string;
  lessonCount: number;
  voice: number;
  sight: number;
  pen: number;
  /** Share of the track learnable by ear alone, to a whole percent. */
  drivablePercent: number;
  /** Sum of every chapter's drivable prefix — lessons actually reachable in the car. */
  drivablePrefixTotal: number;
  modalities: Modality[];
  chapters: ModalityManifestChapter[];
}

/** The corpus in one glance. Every field here is pinned by a test. */
export interface ModalityManifestSummary {
  totalLessons: number;
  voice: number;
  sight: number;
  pen: number;
  /** Identical to `voice` today; named separately because it is the question asked. */
  drivableLessons: number;
  drivablePercent: number;
  trackCount: number;
  chapterCount: number;
  /** Lessons reachable in the car once prerequisite order is respected. */
  drivablePrefixTotal: number;
  /** Chapters a commuter can finish end to end. */
  fullyDrivableChapters: number;
  /** Chapters a commuter cannot even START by ear (drivable prefix 0). */
  unstartableChapters: number;
  /** Lessons whose accepted modality came from an author, not the derivation. */
  overriddenLessons: number;
  /** Lessons with no parseable chapter — counted in tracks, absent from chapters. */
  lessonsWithoutChapter: number;
}

/** The whole artifact. */
export interface ModalityManifest {
  version: number;
  algorithm: "fnv1a64";
  features: ModalityManifestFeatures;
  policy: ModalityManifestPolicy;
  /** Fingerprint of every lesson AST the manifest was derived from. */
  sourceHash: string;
  summary: ModalityManifestSummary;
  tracks: ModalityManifestTrack[];
  lessons: ModalityManifestLesson[];
  /**
   * Override problems found while deriving. Empty in a healthy corpus; emitted rather
   * than thrown, following the multi-pass habit every validator in this package has.
   */
  findings: ModalityFinding[];
}

/** Stable fields carried by every language metadata owner. */
export type ModalityManifestHeader = Pick<
  ModalityManifest,
  "version" | "algorithm" | "features" | "policy"
>;

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

/**
 * The order a reader meets the lessons in: track, then chapter, then authored
 * sequence, then id.
 *
 * Every null sorts LAST within its bucket rather than as zero. A legacy lesson with no
 * `sequence` has not declared that it comes first; giving it position 0 would invent a
 * claim the author never made and would silently reorder the drivable prefix. Same
 * reasoning for a lesson whose chapter did not parse. `localeCompare` on the id is the
 * final tiebreak so the file is byte-stable no matter what order the filesystem handed
 * the lessons over in — which matters enormously here, because `--check` compares bytes
 * and `readdirSync` order is a property of the disk, not of the curriculum.
 */
function compareLessons(left: LessonModality, right: LessonModality): number {
  if (left.language !== right.language) return left.language.localeCompare(right.language);
  const leftChapter = left.chapter ?? Number.POSITIVE_INFINITY;
  const rightChapter = right.chapter ?? Number.POSITIVE_INFINITY;
  if (leftChapter !== rightChapter) return leftChapter - rightChapter;
  const leftSequence = left.sequence ?? Number.POSITIVE_INFINITY;
  const rightSequence = right.sequence ?? Number.POSITIVE_INFINITY;
  if (leftSequence !== rightSequence) return leftSequence - rightSequence;
  return left.lessonId.localeCompare(right.lessonId);
}

// ---------------------------------------------------------------------------
// The corpus fingerprint
// ---------------------------------------------------------------------------

/**
 * One hash over every lesson AST the manifest describes.
 *
 * This is the compact form of "the manifest matches the corpus". `--check` compares the
 * whole file byte for byte and is the real gate; this field is for consumers that cache
 * — an app can keep a rendered driving edition and re-render only when the hash moves,
 * instead of diffing 1,096 rows.
 *
 * `combineLessonHashes` from `hash.ts` is deliberately NOT reused: it orders by
 * `sequence` first, and a large share of this corpus has no sequence at all.
 * `Number(undefined)` is `NaN`, every comparison against NaN is false, and the sort
 * therefore degenerates to whatever order the directory walk produced. A fingerprint
 * whose input order depends on the filesystem is not a fingerprint, it is a coin flip.
 * Sorting by id is total and stable, so the same corpus always hashes the same.
 */
function modalityRowsHash(rows: readonly Pick<ModalityManifestLesson, "id" | "sourceHash">[]): string {
  const entries = rows
    .map((row) => [row.id, row.sourceHash] as const)
    .sort((left, right) => left[0].localeCompare(right[0]));
  return fnv1a64(JSON.stringify(entries));
}

export function modalityCorpusHash(lessons: readonly ParsedLesson[]): string {
  return modalityRowsHash(
    lessons.map((lesson) => ({ id: lesson.realization.lessonId, sourceHash: lesson.sourceHash })),
  );
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

function percent(part: number, whole: number): number {
  return whole === 0 ? 0 : Math.round((part / whole) * 100);
}

function count(entries: readonly LessonModality[], modality: Modality): number {
  return entries.filter((entry) => entry.modality === modality).length;
}

function manifestLesson(entry: LessonModality, sourceHash: string): ModalityManifestLesson {
  const row: ModalityManifestLesson = {
    id: entry.lessonId,
    language: entry.language,
    chapter: entry.chapter,
    sequence: entry.sequence,
    modality: entry.modality,
    derived: entry.derived,
    drivable: entry.modality === "voice",
    reasons: entry.reasons,
    coreModality: entry.coreModality,
    coreReasons: entry.coreReasons,
    coreDrivable: entry.coreModality === "voice",
    sourceHash,
  };
  // Omitted rather than emitted empty, for the same reason as the override fields
  // below: most lessons have no detachable section, and `"detachableSegments": []`
  // on a thousand rows is noise a reader must learn to skip.
  if (entry.detachableSegments.length > 0) {
    row.detachableSegments = entry.detachableSegments;
  }
  // Omitted when absent, same rule: today only the Tamil writing strand declares it.
  if (entry.delivery) {
    row.delivery = entry.delivery;
  }
  // The three override fields are omitted rather than emitted empty. They apply to a
  // handful of lessons out of 1,096, and `"authoredReason": ""` a thousand times over
  // is noise every reader must learn to skip. Absent means "the author said nothing",
  // which is exactly what HL08 wants the common case to cost: nothing.
  if (entry.authored !== null) row.authored = entry.authored;
  if (entry.authoredReason !== "") row.authoredReason = entry.authoredReason;
  if (entry.overridden) row.overridden = true;
  return row;
}

function manifestChapter(
  chapter: number,
  entries: readonly LessonModality[],
): ModalityManifestChapter {
  const ordered = orderChapterLessons(entries);
  // Walk from the front and stop at the first lesson that is not `voice`. It is
  // deliberately NOT "collect every voice lesson": the lessons are prerequisite
  // ordered, so a voice lesson sitting behind a sight one is not reachable in the car
  // however ear-friendly it is on its own.
  const drivableLessonIds: string[] = [];
  for (const entry of ordered) {
    if (entry.modality !== "voice") break;
    drivableLessonIds.push(entry.lessonId);
  }
  const prefix = drivableLessonIds.length;
  return {
    chapter,
    lessonCount: ordered.length,
    voice: count(ordered, "voice"),
    sight: count(ordered, "sight"),
    pen: count(ordered, "pen"),
    modalities: unionModalities(ordered.map((entry) => entry.modality)),
    drivablePrefix: prefix,
    firstNonVoiceLesson: prefix < ordered.length ? (ordered[prefix]?.lessonId ?? null) : null,
    // An empty chapter is not "fully drivable" — it is empty. Saying otherwise would
    // let a track with no authored lessons advertise a car-ready course.
    drivable: ordered.length > 0 && prefix === ordered.length,
    drivableLessonIds,
  };
}

/**
 * Build the manifest from parsed lessons.
 *
 * Pure — no filesystem, no clock, no randomness — so the same corpus always produces
 * the same bytes. That property is what turns `--check` into a gate rather than a
 * suggestion.
 *
 * Grouping goes through `Map`, never a plain object used as a dictionary. The keys are
 * language names and chapter numbers read out of authored files, and an object keyed by
 * strings from disk is one `__proto__` away from a prototype-pollution bug. A `Map` has
 * no prototype chain to fall into.
 */
export function buildModalityManifest(
  lessons: readonly ParsedLesson[],
  options: ModalityOptions = {},
): ModalityManifest {
  const maxLinearisableTableColumns =
    options.maxLinearisableTableColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;

  // Derive once, keeping each lesson's AST hash beside its derivation. A `Map` keyed by
  // lesson id would silently drop a row if two tracks ever shipped the same id, so the
  // hash rides along in the same record instead.
  const derived: Array<{ entry: LessonModality; sourceHash: string }> = lessons.map((lesson) => ({
    entry: deriveLessonModality(lesson, { maxLinearisableTableColumns }),
    sourceHash: lesson.sourceHash,
  }));
  derived.sort((left, right) => compareLessons(left.entry, right.entry));

  const findings: ModalityFinding[] = [];
  for (const { entry } of derived) findings.push(...modalityFindings(entry));
  findings.sort(
    (left, right) =>
      left.language.localeCompare(right.language) ||
      left.lessonId.localeCompare(right.lessonId) ||
      left.code.localeCompare(right.code),
  );

  const byLanguage = new Map<string, LessonModality[]>();
  for (const { entry } of derived) {
    const bucket = byLanguage.get(entry.language);
    if (bucket) bucket.push(entry);
    else byLanguage.set(entry.language, [entry]);
  }

  const tracks: ModalityManifestTrack[] = [];
  for (const language of [...byLanguage.keys()].sort()) {
    const trackEntries = byLanguage.get(language) ?? [];
    const byChapter = new Map<number, LessonModality[]>();
    for (const entry of trackEntries) {
      // A lesson whose chapter did not parse still counts toward its track — it exists
      // and the learner meets it — but it cannot belong to an ordered prefix, so it is
      // left out of the chapter list rather than bucketed into a chapter 0 that no book
      // prints. `summary.lessonsWithoutChapter` keeps that omission visible.
      if (entry.chapter === null) continue;
      const bucket = byChapter.get(entry.chapter);
      if (bucket) bucket.push(entry);
      else byChapter.set(entry.chapter, [entry]);
    }
    const chapters = [...byChapter.keys()]
      .sort((left, right) => left - right)
      .map((chapter) => manifestChapter(chapter, byChapter.get(chapter) ?? []));
    const voice = count(trackEntries, "voice");
    tracks.push({
      language,
      lessonCount: trackEntries.length,
      voice,
      sight: count(trackEntries, "sight"),
      pen: count(trackEntries, "pen"),
      drivablePercent: percent(voice, trackEntries.length),
      drivablePrefixTotal: chapters.reduce((sum, chapter) => sum + chapter.drivablePrefix, 0),
      modalities: unionModalities(trackEntries.map((entry) => entry.modality)),
      chapters,
    });
  }

  const allEntries = derived.map(({ entry }) => entry);
  const allChapters = tracks.flatMap((track) => track.chapters);
  const voice = count(allEntries, "voice");

  return {
    version: MODALITY_MANIFEST_VERSION,
    algorithm: "fnv1a64",
    // True as of HL-C48: every row now carries `coreModality`, `coreReasons`,
    // `coreDrivable`, and `detachableSegments` where it has any. The flag was honest
    // while it was false — the data genuinely was not emitted — and flipping it
    // without emitting the data would have been the actual bug.
    features: { blockModality: true },
    policy: { maxLinearisableTableColumns },
    sourceHash: modalityCorpusHash(lessons),
    summary: {
      totalLessons: allEntries.length,
      voice,
      sight: count(allEntries, "sight"),
      pen: count(allEntries, "pen"),
      drivableLessons: voice,
      drivablePercent: percent(voice, allEntries.length),
      trackCount: tracks.length,
      chapterCount: allChapters.length,
      drivablePrefixTotal: allChapters.reduce((sum, chapter) => sum + chapter.drivablePrefix, 0),
      fullyDrivableChapters: allChapters.filter((chapter) => chapter.drivable).length,
      unstartableChapters: allChapters.filter((chapter) => chapter.drivablePrefix === 0).length,
      overriddenLessons: allEntries.filter((entry) => entry.overridden).length,
      lessonsWithoutChapter: allEntries.filter((entry) => entry.chapter === null).length,
    },
    tracks,
    lessons: derived.map(({ entry, sourceHash }) => manifestLesson(entry, sourceHash)),
    findings,
  };
}

/**
 * The manifest's canonical on-disk bytes.
 *
 * Two-space indent and a trailing newline, matching `core/generated-book-hashes/*.json`
 * — a generated JSON file a human will nonetheless open in a diff, so it is formatted
 * for reading. Having exactly one definition of the bytes is what makes `--write` and
 * `--check` structurally incapable of disagreeing about formatting.
 */
export function serializeModalityManifest(manifest: ModalityManifest): string {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

function compareManifestLessons(
  left: ModalityManifestLesson,
  right: ModalityManifestLesson,
): number {
  return left.language.localeCompare(right.language) ||
    (left.chapter ?? Number.POSITIVE_INFINITY) -
      (right.chapter ?? Number.POSITIVE_INFINITY) ||
    (left.sequence ?? Number.POSITIVE_INFINITY) -
      (right.sequence ?? Number.POSITIVE_INFINITY) ||
    left.id.localeCompare(right.id);
}

function manifestChapterFromRows(
  chapter: number,
  rows: readonly ModalityManifestLesson[],
): ModalityManifestChapter {
  const ordered = [...rows].sort(compareManifestLessons);
  const drivableLessonIds: string[] = [];
  for (const row of ordered) {
    if (row.modality !== "voice") break;
    drivableLessonIds.push(row.id);
  }
  const drivablePrefix = drivableLessonIds.length;
  return {
    chapter,
    lessonCount: ordered.length,
    voice: ordered.filter((row) => row.modality === "voice").length,
    sight: ordered.filter((row) => row.modality === "sight").length,
    pen: ordered.filter((row) => row.modality === "pen").length,
    modalities: unionModalities(ordered.map((row) => row.modality)),
    drivablePrefix,
    firstNonVoiceLesson:
      drivablePrefix < ordered.length ? (ordered[drivablePrefix]?.id ?? null) : null,
    drivable: ordered.length > 0 && drivablePrefix === ordered.length,
    drivableLessonIds,
  };
}

/**
 * Reconstruct every aggregate rollup from direct lesson owners.
 *
 * The owner tree stores no source hash, summary, track, or chapter aggregate. Keeping
 * this fold beside the original builder gives generators and readers one definition of
 * those public fields and prevents a derived rollup from becoming a new merge seam.
 */
export function buildModalityManifestFromRows(
  header: ModalityManifestHeader,
  inputLessons: readonly ModalityManifestLesson[],
  inputFindings: readonly ModalityFinding[],
): ModalityManifest {
  const lessons = [...inputLessons].sort(compareManifestLessons);
  const findings = [...inputFindings].sort(
    (left, right) =>
      left.language.localeCompare(right.language) ||
      left.lessonId.localeCompare(right.lessonId) ||
      left.code.localeCompare(right.code),
  );
  const byLanguage = new Map<string, ModalityManifestLesson[]>();
  for (const lesson of lessons) {
    const bucket = byLanguage.get(lesson.language);
    if (bucket) bucket.push(lesson);
    else byLanguage.set(lesson.language, [lesson]);
  }

  const tracks: ModalityManifestTrack[] = [];
  for (const language of [...byLanguage.keys()].sort()) {
    const trackLessons = byLanguage.get(language) ?? [];
    const byChapter = new Map<number, ModalityManifestLesson[]>();
    for (const lesson of trackLessons) {
      if (lesson.chapter === null) continue;
      const bucket = byChapter.get(lesson.chapter);
      if (bucket) bucket.push(lesson);
      else byChapter.set(lesson.chapter, [lesson]);
    }
    const chapters = [...byChapter.keys()]
      .sort((left, right) => left - right)
      .map((chapter) => manifestChapterFromRows(chapter, byChapter.get(chapter) ?? []));
    const voice = trackLessons.filter((lesson) => lesson.modality === "voice").length;
    tracks.push({
      language,
      lessonCount: trackLessons.length,
      voice,
      sight: trackLessons.filter((lesson) => lesson.modality === "sight").length,
      pen: trackLessons.filter((lesson) => lesson.modality === "pen").length,
      drivablePercent: percent(voice, trackLessons.length),
      drivablePrefixTotal: chapters.reduce((sum, chapter) => sum + chapter.drivablePrefix, 0),
      modalities: unionModalities(trackLessons.map((lesson) => lesson.modality)),
      chapters,
    });
  }

  const chapters = tracks.flatMap((track) => track.chapters);
  const voice = lessons.filter((lesson) => lesson.modality === "voice").length;
  return {
    version: header.version,
    algorithm: header.algorithm,
    features: header.features,
    policy: header.policy,
    sourceHash: modalityRowsHash(lessons),
    summary: {
      totalLessons: lessons.length,
      voice,
      sight: lessons.filter((lesson) => lesson.modality === "sight").length,
      pen: lessons.filter((lesson) => lesson.modality === "pen").length,
      drivableLessons: voice,
      drivablePercent: percent(voice, lessons.length),
      trackCount: tracks.length,
      chapterCount: chapters.length,
      drivablePrefixTotal: chapters.reduce((sum, chapter) => sum + chapter.drivablePrefix, 0),
      fullyDrivableChapters: chapters.filter((chapter) => chapter.drivable).length,
      unstartableChapters: chapters.filter((chapter) => chapter.drivablePrefix === 0).length,
      overriddenLessons: lessons.filter((lesson) => lesson.overridden).length,
      lessonsWithoutChapter: lessons.filter((lesson) => lesson.chapter === null).length,
    },
    tracks,
    lessons,
    findings,
  };
}

/**
 * Reassemble the public corpus view from independently committed language shards.
 *
 * The aggregate is derived at read time so no language PR ever rewrites a shared
 * summary line. Every field is reconstructed from the shard rows; a stale or missing
 * shard still fails `check:modality` rather than being papered over here.
 */
export function mergeModalityManifests(
  manifests: readonly ModalityManifest[],
): ModalityManifest {
  if (manifests.length === 0) throw new Error("no modality manifest shards found");
  const first = manifests[0]!;
  for (const manifest of manifests.slice(1)) {
    if (
      manifest.version !== first.version ||
      manifest.algorithm !== first.algorithm ||
      manifest.features.blockModality !== first.features.blockModality ||
      manifest.policy.maxLinearisableTableColumns !== first.policy.maxLinearisableTableColumns
    ) {
      throw new Error("incompatible modality manifest shards");
    }
  }

  const lessons = manifests.flatMap((manifest) => manifest.lessons)
    .sort(compareManifestLessons);
  const findings = manifests.flatMap((manifest) => manifest.findings)
    .sort((left, right) =>
      left.language.localeCompare(right.language) ||
      left.lessonId.localeCompare(right.lessonId) ||
      left.code.localeCompare(right.code),
    );
  return buildModalityManifestFromRows({
    version: first.version,
    algorithm: first.algorithm,
    features: first.features,
    policy: first.policy,
  }, lessons, findings);
}
