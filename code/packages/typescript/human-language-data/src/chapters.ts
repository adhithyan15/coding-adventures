// HL05 chapter-capability gates — migration step 2, report-only.
//
// HL05 gave every chapter an authored promise: a `canDo` sentence and a `payoff`
// lesson that proves it. This module is the machinery that checks the promise, and
// it deliberately does NOT fail a build.
//
// WHY REPORT-ONLY, AND WHY THAT IS NOT TIMIDITY.
//
// At the time this landed, 279 of the corpus's 377 book chapters carried a capability
// entry and 98 did not. Wiring these nine rules straight into `validateCurriculum()`
// as errors would have turned a measurement of pre-existing debt into 98 build
// failures on a corpus nobody had regressed — the precedent HL-V01 set, and the same
// reason the LaTeX warning baselines ship unseeded. A track flips to hard errors once
// its own debt reaches zero, exactly as the schema-v2 migration does.
//
// So every function here collects and returns; nothing throws, and nothing decides
// what to do about what it finds. `report.ts` publishes the findings and the counts.
//
// THE ONE RULE WORTH READING TWICE is `chapter-payoff-not-closed`. A payoff may only
// assess atoms the reader has actually been taught — in this chapter or an earlier
// one in the same track. Without it a chapter could claim a capability by testing
// material from three chapters ahead, which reads as a working ledger right up until
// a learner hits it and cannot do the thing the chapter promised.

import type {
  ChapterCapability,
  ChapterPolicy,
  BookCorpus,
  TrackChapters,
} from "./types.js";
import type { ParsedLesson } from "./parse.js";

/** Every gate code HL05 defines. Stable strings — the report and tests key on them. */
export const CHAPTER_GATE_CODES = [
  "chapter-missing-capability",
  "chapter-unknown-payoff-lesson",
  "chapter-payoff-not-closed",
  "chapter-payoff-not-representative",
  "chapter-duplicate",
  "chapter-title-drift",
  "pattern-slot-not-closed",
  "pattern-missing-production",
  "pattern-multiple-atoms",
] as const;

export type ChapterGateCode = (typeof CHAPTER_GATE_CODES)[number];

/** One violation. `chapter` is absent on findings that are not about a chapter. */
export interface ChapterFinding {
  code: ChapterGateCode;
  language: string;
  chapter?: number;
  /** Lesson the finding is about, where one applies. */
  lessonId?: string;
  message: string;
}

export interface ChapterGateInput {
  books: BookCorpus;
  lessons: ParsedLesson[];
  trackChapters: TrackChapters[];
  policy: ChapterPolicy;
}

/** Per-track rollup, so a track can see when its own debt has reached zero. */
export interface TrackChapterCoverage {
  language: string;
  /** Chapters the track's book actually contains. */
  bookChapters: number;
  /** Chapters with an authored capability entry. */
  declaredChapters: number;
  /** Chapters in the book with no entry at all. */
  missingCapability: number;
  /** Findings of every kind for this track. */
  findings: number;
  /** True once this track can be flipped to hard errors. */
  clean: boolean;
}

export interface ChapterGateReport {
  findings: ChapterFinding[];
  tracks: TrackChapterCoverage[];
  summary: {
    bookChapters: number;
    declaredChapters: number;
    chaptersWithoutCapability: number;
    payoffsNotClosed: number;
    payoffsNotRepresentative: number;
    unknownPayoffLessons: number;
    titleDrift: number;
    duplicateChapters: number;
    /** Tracks whose debt is already zero — the ones that may flip to errors. */
    cleanTracks: number;
    /** The threshold the representativeness rule ran at, so a reader can reproduce it. */
    payoffRepresentativeness: number;
  };
}

/**
 * Atoms a lesson introduces.
 *
 * Schema-v1 lessons carry no `introduces` block at all, so they contribute nothing —
 * which is honest rather than convenient: a v1 chapter genuinely has no machine-readable
 * record of what it taught, and pretending otherwise would let its payoff pass the
 * closure check by comparing against an empty set. Those chapters surface instead as
 * `chapter-payoff-not-closed`, and the fix is the schema-v2 migration, not a looser gate.
 */
function frontmatterList(lesson: ParsedLesson, key: string): string[] {
  const value = lesson.frontmatter[key];
  if (Array.isArray(value)) return value.filter((item): item is string => typeof item === "string");
  return typeof value === "string" && value.trim() ? [value.trim()] : [];
}

function introducedAtoms(lesson: ParsedLesson): string[] {
  // TWO SOURCES, AND BOTH ARE LOAD-BEARING. The frontmatter key is FLAT and dotted —
  // `introduces.knowledge`, not a nested `introduces: { knowledge }` object — because
  // that is how the frontmatter parser flattens it. Reading the nested shape returns
  // `undefined` for every lesson in the corpus, which does not fail loudly: it silently
  // makes the "taught so far" set empty, so `chapter-payoff-not-closed` fires on all 279
  // authored chapters and reads like total corpus debt. That is exactly what the first
  // run of this gate reported, and it was this bug, not the corpus.
  //
  // Block-level `hl-knowledge` directives are the schema-v2 source and carry the atoms
  // the frontmatter summary can omit, so the answer is the union of both.
  const atoms = new Set(frontmatterList(lesson, "introduces.knowledge"));
  for (const block of lesson.blocks ?? []) {
    for (const atom of block.knowledge?.introduces ?? []) atoms.add(atom);
  }
  return [...atoms];
}

function lessonChapter(lesson: ParsedLesson): number | null {
  const chapter = lesson.realization?.chapter;
  return typeof chapter === "number" ? chapter : null;
}

/**
 * Run all nine HL05 gates over the corpus.
 *
 * Multi-pass and total: every chapter of every track is visited even after a finding,
 * so one broken ledger cannot hide the rest. That is `curriculum.ts`'s established
 * style and the reason this returns a list rather than a verdict.
 */
export function runChapterGates(input: ChapterGateInput): ChapterGateReport {
  const { books, lessons, trackChapters, policy } = input;
  const findings: ChapterFinding[] = [];
  const tracks: TrackChapterCoverage[] = [];

  const byLanguage = new Map<string, TrackChapters>();
  for (const track of trackChapters) byLanguage.set(track.language, track);

  const lessonsById = new Map<string, ParsedLesson>();
  for (const lesson of lessons) lessonsById.set(lesson.realization.lessonId, lesson);

  // Atoms introduced, per track, per chapter — the input to the closure rule.
  const atomsByTrackChapter = new Map<string, Map<number, Set<string>>>();
  for (const lesson of lessons) {
    const chapter = lessonChapter(lesson);
    if (chapter === null) continue;
    let byChapter = atomsByTrackChapter.get(lesson.language);
    if (!byChapter) {
      byChapter = new Map();
      atomsByTrackChapter.set(lesson.language, byChapter);
    }
    let set = byChapter.get(chapter);
    if (!set) {
      set = new Set();
      byChapter.set(chapter, set);
    }
    for (const atom of introducedAtoms(lesson)) set.add(atom);
  }

  for (const book of books.books) {
    const language = book.language;
    const track = byLanguage.get(language);
    const entries: ChapterCapability[] = track ? track.chapters : [];
    const before = findings.length;

    // ---- chapter-duplicate ------------------------------------------------
    const seen = new Map<number, number>();
    for (const entry of entries) seen.set(entry.chapter, (seen.get(entry.chapter) ?? 0) + 1);
    for (const [chapter, count] of seen) {
      if (count > 1) {
        findings.push({
          code: "chapter-duplicate",
          language,
          chapter,
          message: `${language} chapter ${chapter} has ${count} entries in chapters.json; exactly one is allowed.`,
        });
      }
    }

    const entryByChapter = new Map<number, ChapterCapability>();
    for (const entry of entries) if (!entryByChapter.has(entry.chapter)) entryByChapter.set(entry.chapter, entry);

    let missingCapability = 0;

    for (const bookChapter of book.chapters) {
      const number = bookChapter.chapter;
      const entry = entryByChapter.get(number);

      // ---- chapter-missing-capability ------------------------------------
      if (!entry || !entry.canDo?.trim() || !entry.payoff) {
        missingCapability += 1;
        findings.push({
          code: "chapter-missing-capability",
          language,
          chapter: number,
          message: `${language} chapter ${number} ("${bookChapter.title ?? "untitled"}") has no authored canDo/payoff.`,
        });
        continue;
      }

      // ---- chapter-title-drift -------------------------------------------
      // The book is generated from `book-generation.json`; the ledger is authored.
      // They are two records of the same fact, so they must agree or one is lying.
      if (bookChapter.title && entry.title && bookChapter.title !== entry.title) {
        findings.push({
          code: "chapter-title-drift",
          language,
          chapter: number,
          message: `${language} chapter ${number} title drift: book says "${bookChapter.title}", chapters.json says "${entry.title}".`,
        });
      }

      // ---- chapter-unknown-payoff-lesson ---------------------------------
      const payoffLesson = lessonsById.get(entry.payoff.lesson);
      if (!payoffLesson) {
        findings.push({
          code: "chapter-unknown-payoff-lesson",
          language,
          chapter: number,
          lessonId: entry.payoff.lesson,
          message: `${language} chapter ${number} payoff lesson "${entry.payoff.lesson}" does not exist.`,
        });
        continue;
      }
      const payoffChapter = lessonChapter(payoffLesson);
      if (payoffChapter !== null && payoffChapter !== number) {
        findings.push({
          code: "chapter-unknown-payoff-lesson",
          language,
          chapter: number,
          lessonId: entry.payoff.lesson,
          message: `${language} chapter ${number} payoff lesson "${entry.payoff.lesson}" belongs to chapter ${payoffChapter}.`,
        });
      }

      // ---- chapter-payoff-not-closed -------------------------------------
      // Everything taught in THIS chapter or any earlier one in the same track.
      const taught = new Set<string>();
      const byChapter = atomsByTrackChapter.get(language);
      if (byChapter) {
        for (const [chapterNumber, atoms] of byChapter) {
          if (chapterNumber <= number) for (const atom of atoms) taught.add(atom);
        }
      }
      const assesses = entry.payoff.assesses ?? [];
      const unclosed = assesses.filter((atom) => !taught.has(atom));
      if (unclosed.length > 0) {
        findings.push({
          code: "chapter-payoff-not-closed",
          language,
          chapter: number,
          lessonId: entry.payoff.lesson,
          message: `${language} chapter ${number} payoff assesses ${unclosed.length} atom(s) never taught by chapter ${number}: ${unclosed.slice(0, 5).join(", ")}${unclosed.length > 5 ? ", …" : ""}.`,
        });
      }

      // ---- chapter-payoff-not-representative -----------------------------
      // Guarded on a non-empty chapter: a chapter that introduces nothing has no
      // share to meet, and dividing by zero would report every v1 chapter as
      // unrepresentative for a reason that has nothing to do with its payoff.
      const introduced = byChapter?.get(number);
      if (introduced && introduced.size > 0) {
        const covered = assesses.filter((atom) => introduced.has(atom)).length;
        const share = covered / introduced.size;
        if (share < policy.payoffRepresentativeness) {
          findings.push({
            code: "chapter-payoff-not-representative",
            language,
            chapter: number,
            lessonId: entry.payoff.lesson,
            message: `${language} chapter ${number} payoff assesses ${covered}/${introduced.size} of the chapter's atoms (${share.toFixed(2)}), below the ${policy.payoffRepresentativeness} floor.`,
          });
        }
      }
    }

    tracks.push({
      language,
      bookChapters: book.chapters.length,
      declaredChapters: entries.length,
      missingCapability,
      findings: findings.length - before,
      clean: findings.length === before,
    });
  }

  // ---- the three `pattern` gates ----------------------------------------
  // HL-C05 adds the `pattern` lesson type. Until it lands there are no pattern
  // lessons, so these three report zero — which is the correct answer, not a
  // stub. Wiring them now means the gates exist the moment the first pattern is
  // authored, rather than being remembered later.
  findings.push(...runPatternGates(lessons));

  const count = (code: ChapterGateCode): number =>
    findings.filter((finding) => finding.code === code).length;

  return {
    findings,
    tracks,
    summary: {
      bookChapters: books.books.reduce((sum, book) => sum + book.chapters.length, 0),
      declaredChapters: trackChapters.reduce((sum, track) => sum + track.chapters.length, 0),
      chaptersWithoutCapability: count("chapter-missing-capability"),
      payoffsNotClosed: count("chapter-payoff-not-closed"),
      payoffsNotRepresentative: count("chapter-payoff-not-representative"),
      unknownPayoffLessons: count("chapter-unknown-payoff-lesson"),
      titleDrift: count("chapter-title-drift"),
      duplicateChapters: count("chapter-duplicate"),
      cleanTracks: tracks.filter((track) => track.clean).length,
      payoffRepresentativeness: policy.payoffRepresentativeness,
    },
  };
}

/**
 * The three `pattern` rules (HL05 / HL-C05).
 *
 * A `pattern` lesson teaches one reusable frame — "I want ___" — and the rules exist
 * so it cannot quietly become an ordinary vocabulary lesson wearing a new type name:
 * it must introduce exactly one `*-PATTERN-*` atom, every slot filler it names must be
 * something the reader already has, and it must actually make the reader produce the
 * frame at least three times rather than only admire it.
 */
export function runPatternGates(lessons: ParsedLesson[]): ChapterFinding[] {
  const findings: ChapterFinding[] = [];
  for (const lesson of lessons) {
    const type = (lesson as unknown as { realization?: { type?: string } }).realization?.type;
    if (type !== "pattern") continue;

    const atoms = introducedAtoms(lesson);
    const patternAtoms = atoms.filter((atom) => atom.includes("-PATTERN-"));
    if (patternAtoms.length !== 1) {
      findings.push({
        code: "pattern-multiple-atoms",
        language: lesson.language,
        chapter: lessonChapter(lesson) ?? undefined,
        lessonId: lesson.realization.lessonId,
        message: `${lesson.realization.lessonId} is a pattern lesson introducing ${patternAtoms.length} *-PATTERN-* atoms; exactly one is required.`,
      });
    }

    const blocks = lesson.blocks ?? [];
    const production = blocks.filter((block) => block.type === "guided-production");
    const instantiations = production.reduce(
      (sum, block) =>
        sum +
        (block.markdown ?? "").split("\n").filter((line: string) => /^\s*[-*\d]/.test(line)).length,
      0,
    );
    if (production.length === 0 || instantiations < 3) {
      findings.push({
        code: "pattern-missing-production",
        language: lesson.language,
        chapter: lessonChapter(lesson) ?? undefined,
        lessonId: lesson.realization.lessonId,
        message: `${lesson.realization.lessonId} is a pattern lesson with ${instantiations} guided-production instantiation(s); at least 3 are required.`,
      });
    }

    const closure = new Set(frontmatterList(lesson, "requires.knowledge"));
    for (const block of lesson.blocks ?? []) {
      for (const atom of block.knowledge?.assesses ?? []) closure.add(atom);
    }
    for (const atom of atoms) {
      if (atom.includes("-PATTERN-")) continue;
      if (!closure.has(atom)) {
        findings.push({
          code: "pattern-slot-not-closed",
          language: lesson.language,
          chapter: lessonChapter(lesson) ?? undefined,
          lessonId: lesson.realization.lessonId,
          message: `${lesson.realization.lessonId} names slot filler "${atom}" that is not in the lesson's declared closure.`,
        });
      }
    }
  }
  return findings;
}
