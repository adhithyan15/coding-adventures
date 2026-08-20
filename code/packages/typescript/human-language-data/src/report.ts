import type { ParsedLesson } from "./parse.js";
import { runChapterGates, type ChapterGateReport } from "./chapters.js";
import { CEFR_LEVELS, summarizeLevels, type LevelSummary } from "./levels.js";
import { measureRamp, type RampReport } from "./ramp.js";
import { measureContinuity, type ContinuityReport } from "./continuity.js";
import { measureScriptClosure, type ScriptClosureReport } from "./script-closure.js";
import { runLevelGate, type LevelGateReport } from "./level-gate.js";
import type { AssessmentPolicy } from "./assessment.js";
import { measureWritingStages, type WritingStageReport } from "./writing-stages.js";
import type {
  BookCorpus,
  ChapterPolicy,
  CurriculumSpine,
  LanguageCurriculum,
  LanguageRegistry,
  TrackChapters,
} from "./types.js";
import { summarizeModality, type ModalityOptions, type ModalitySummary } from "./modality.js";
import { stripControlCharacters as clean } from "./constants.js";

export const DURATION_THRESHOLD_SECONDS = 300;

export interface DurationEstimate {
  lessonId: string;
  language: string;
  chapter: number | null;
  declaredSeconds: number;
  computedSeconds: number;
  effectiveSeconds: number;
  wordCount: number;
  promptCount: number;
  repeatCueCount: number;
  explicitPauseSeconds: number;
  authoredAudioSeconds: number;
  /** Sum of explicit `response_seconds` budgets on compiled activities. */
  activityResponseSeconds: number;
  reasons: Array<"declared" | "computed">;
}

export interface UnknownPrerequisite {
  lessonId: string;
  language: string;
  prerequisite: string;
}

export interface PrerequisiteRoot {
  lessonId: string;
  language: string;
  chapter: number | null;
}

export interface BookCoverage {
  language: string;
  hasBook: boolean;
  lessonChapters: number[];
  bookChapters: number[];
  missingBookChapters: number[];
  orphanBookChapters: number[];
  coveragePercent: number;
}

export type TrackSchemaStatus = "legacy" | "mixed" | "version-2";

export interface TrackSchemaCoverage {
  language: string;
  status: TrackSchemaStatus;
  lessonCount: number;
  versions: Record<string, number>;
}

export interface CurriculumGapReport {
  schemaVersion: 1;
  durationModel: {
    version: 2;
    thresholdSeconds: number;
    spokenWordsPerMinute: number;
    learnerResponseSecondsPerPrompt: number;
    repeatCueSeconds: number;
    safetyMarginPercent: number;
    audioDuration: "max(spoken estimate, authored audio seconds)";
    effectiveDuration: "max(declared, computed)";
  };
  summary: {
    registeredTracks: number;
    totalLessons: number;
    authoredBooks: number;
    durationViolations: number;
    unknownPrerequisites: number;
    laterChapterLessonsWithoutPrerequisites: number;
    tracksWithoutBooks: number;
    lessonChaptersWithoutBooks: number;
    legacySchemaTracks: number;
    mixedSchemaTracks: number;
    version2SchemaTracks: number;
    /**
     * HL08: lessons a hands-free view can deliver, and that share of the corpus.
     *
     * Counted on the lesson CORE — the lesson minus any detachable writing segment —
     * so a lesson that is voice apart from a short interspersed writing aside counts
     * here. The book still prints the segment; see `modality.ts`.
     */
    drivableLessons: number;
    drivablePercent: number;
    /** Chapters a commuter cannot even start — drivable prefix 0. */
    chaptersWithoutDrivablePrefix: number;
    /** Authored `modality:` overrides that contradict the derivation unexplained. */
    unexplainedModalityOverrides: number;
    /**
     * HL05 chapter-gate headline counts. `null` when the caller supplied no chapter
     * ledgers — distinguishable from 0, which means "measured, and none found".
     */
    /** HL-C10: lessons no spine node claims, so no level could be derived. */
    lessonsWithoutLevel: number | null;
    /** HL08: lessons above `maxNewAtomsPerLesson`. Null when no policy was supplied. */
    rampOverBudgetLessons: number | null;
    /** HL08: lessons above `maxNewGlyphsPerLesson` — the decoding burden, newly counted. */
    scriptRampOverBudgetLessons: number | null;
    /** HL08: lessons opening more than one writing system at once. */
    lessonsOpeningMultipleScripts: number | null;
    /**
     * HL11: lessons asking the reader to decode a glyph nobody taught them.
     *
     * The question HL08's glyph budget cannot ask. A track satisfies that budget
     * perfectly while teaching no letters at all, which is what most non-Latin
     * tracks do, so a pace cap reports them as gentle.
     */
    scriptClosureViolations: number;
    /**
     * HL11: glyphs the exposure rule removed from load-bearing sets.
     *
     * Reported beside the violations, because the lesson count alone cannot see
     * the exemption's real size: it counts lessons the rule FLIPPED to clean,
     * and misses everything it shaved off lessons that violate anyway.
     */
    scriptExposureExemptedGlyphs: number;
    /** HL11: non-Latin tracks with no script lesson at all. */
    tracksTeachingNoScript: number;
    /**
     * HL11: native-script headwords with no romanization.
     *
     * The remediation queue, not a violation count. A headword beside its
     * romanization is exposure; without one it is something the reader has to
     * decode, so each of these becomes exempt the moment somebody writes down
     * how to say it.
     */
    headwordsWithoutRomanization: number;
    /** HL09: lessons with no declared reading order. Until zero, the rest is provisional. */
    lessonsWithoutSequence: number;
    /** HL09: lessons reviewing a lesson the learner has not reached yet. */
    forwardReviews: number;
    /** HL09: atoms taught once and never practised again — the headline gap. */
    atomsNeverRevisited: number;
    /** HL09: uses of target-language material a later lesson teaches. */
    forwardReferences: number;
    chaptersWithoutCapability: number | null;
    chapterPayoffsNotRepresentative: number | null;
    chapterGateCleanTracks: number | null;
    /** HL19: registered tracks with no explicit, valid writing-stage evidence. */
    tracksWithoutWritingStageEvidence: number | null;
    /** HL19: cumulative (track, level, stage) pairs still unproved. */
    missingWritingStagePairs: number | null;
    /** HL19: authored evidence that fails the strict evidence contract. */
    invalidWritingStageEvidence: number | null;
  };
  duration: {
    violations: DurationEstimate[];
  };
  prerequisites: {
    unknown: UnknownPrerequisite[];
    roots: PrerequisiteRoot[];
    laterChapterWithoutPrerequisites: PrerequisiteRoot[];
  };
  books: {
    tracks: BookCoverage[];
  };
  schemas: {
    tracks: TrackSchemaCoverage[];
  };
  /**
   * HL08 — what can be learned by ear.
   *
   * Report only, per the HL-V01 precedent: the findings list is published so the
   * debt is visible and burnable, and nothing here fails a build. Gates arrive
   * per track once each track's debt clears.
   */
  /**
   * HL05 chapter-capability gates (HL-C03), report-only.
   *
   * Absent when the caller supplied no chapter ledgers, so an existing consumer that
   * never passes them keeps its current report shape rather than seeing an empty
   * section it cannot distinguish from "measured and found nothing".
   */
  chapters?: ChapterGateReport;
  /**
   * HL-C10: what level each lesson builds toward, derived from the spine.
   *
   * Absent when the caller supplied no curricula/spine, so an existing consumer keeps its
   * report shape rather than seeing an empty section it cannot tell from "measured none".
   */
  levels?: LevelSummary;
  /**
   * HL08 gentle-ramp budgets — vocabulary atoms AND target-script glyphs.
   *
   * `measureRamp` existed and was called by nothing but its own unit test, so the
   * budgets in `chapter-policy.json` were policy in the sense that a sign is policy.
   * Absent when the caller supplied no policy, so a consumer that never passes one
   * keeps its report shape rather than seeing a section measured against defaults
   * nobody chose.
   */
  ramp?: RampReport;
  /**
   * HL09 step 1 — whether the course has a memory of itself.
   *
   * The ramp budgets measure how big each step is; this measures whether the
   * steps hold together. Always present: unlike `ramp` it needs no policy, since
   * order, reinforcement and forward references are all properties of the lessons.
   */
  continuity: ContinuityReport;
  /**
   * HL11 — closure: whether the reader was ever taught the letters they are shown.
   *
   * Always present, like `continuity` and for the same reason: it needs no
   * policy. Latin-script tracks are absent from it by construction, since their
   * reader arrives already knowing the alphabet.
   */
  scriptClosure: ScriptClosureReport;
  /** HL19: cumulative writing capability, distinct from incidental writing practice. */
  writingStages?: WritingStageReport;
  /**
   * HL09 §3.1 — what it takes to CLAIM a level, as opposed to touch one.
   *
   * Present only when levels, ramp and continuity were all computed, since the
   * gate needs all four criteria. Absent is "not measured", never "not attained".
   */
  levelGate?: LevelGateReport;
  modality: ModalitySummary;
}

export interface CurriculumGapReportInput {
  registry: LanguageRegistry;
  lessons: ParsedLesson[];
  books: BookCorpus;
  /** HL08 tunables; defaults to no table being speakable until the lineariser lands. */
  modality?: ModalityOptions;
  /** HL-C10 level derivation needs the realization paths and the spine. */
  curricula?: LanguageCurriculum[];
  spine?: CurriculumSpine;
  /** HL05 chapter ledgers; when omitted the chapter gates do not run. */
  trackChapters?: TrackChapters[];
  /** HL05/HL08 thresholds; required for the chapter gates to run. */
  chapterPolicy?: ChapterPolicy;
  /** HL16/HL19 writing-stage ladder and its first-required-at levels. */
  assessmentPolicy?: AssessmentPolicy;
}

const SPOKEN_WORDS_PER_MINUTE = 120;
const RESPONSE_SECONDS_PER_PROMPT = 8;
const REPEAT_CUE_SECONDS = 4;
const SAFETY_MARGIN = 0.15;

function stringValue(value: ParsedLesson["frontmatter"][string] | undefined): string {
  return typeof value === "string" ? value : "";
}

function stringList(value: ParsedLesson["frontmatter"][string] | undefined): string[] {
  if (Array.isArray(value)) return value;
  return typeof value === "string" && value.trim() !== "" ? [value] : [];
}

function nonNegativeNumber(value: string): number | undefined {
  if (value.trim() === "") return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : undefined;
}

function declaredDurationSeconds(lesson: ParsedLesson): number {
  // The frontmatter parser flattens the indented `max_seconds` member from the
  // HL04 duration block. The old spelling keeps reports useful during migration.
  const maxSeconds =
    nonNegativeNumber(stringValue(lesson.frontmatter["duration.max_seconds"])) ??
    nonNegativeNumber(stringValue(lesson.frontmatter.max_seconds));
  if (maxSeconds !== undefined) return Math.ceil(maxSeconds);
  const minutes = nonNegativeNumber(stringValue(lesson.frontmatter.est_minutes));
  return minutes === undefined ? 0 : Math.ceil(minutes * 60);
}

function stripHtmlComments(markdown: string): string {
  const parts: string[] = [];
  let cursor = 0;
  while (cursor < markdown.length) {
    const start = markdown.indexOf("<!--", cursor);
    if (start === -1) {
      parts.push(markdown.slice(cursor));
      break;
    }
    parts.push(markdown.slice(cursor, start), " ");
    const end = markdown.indexOf("-->", start + 4);
    if (end === -1) break;
    cursor = end + 3;
  }
  return parts.join("");
}

function stripMarkdownLinks(markdown: string): string {
  type State = "text" | "label" | "destination-start" | "destination";
  const output: string[] = [];
  const label: string[] = [];
  const destination: string[] = [];
  let state: State = "text";
  let image = false;

  const reset = (): void => {
    label.length = 0;
    destination.length = 0;
    image = false;
    state = "text";
  };
  const flushUnclosed = (suffix = ""): void => {
    output.push(image ? "![" : "[", ...label);
    if (state === "destination-start" || state === "destination") output.push("]");
    if (state === "destination") output.push("(", ...destination);
    if (suffix !== "") output.push(suffix);
    reset();
  };

  for (let index = 0; index < markdown.length; index += 1) {
    const character = markdown[index];
    if (state === "text") {
      if (character === "!" && markdown[index + 1] === "[") {
        image = true;
        state = "label";
        index += 1;
      } else if (character === "[") {
        state = "label";
      } else {
        output.push(character);
      }
    } else if (state === "label") {
      if (character === "]") state = "destination-start";
      else label.push(character);
    } else if (state === "destination-start") {
      if (character === "(") state = "destination";
      else flushUnclosed(character);
    } else if (character === ")") {
      output.push(image ? " " : label.join(""));
      reset();
    } else {
      destination.push(character);
    }
  }
  if (state !== "text") flushUnclosed();
  return output.join("");
}

function stripAngleTags(text: string): string {
  const parts: string[] = [];
  let cursor = 0;
  while (cursor < text.length) {
    const start = text.indexOf("<", cursor);
    if (start === -1) {
      parts.push(text.slice(cursor));
      break;
    }
    const end = text.indexOf(">", start + 1);
    if (end === -1) {
      parts.push(text.slice(cursor));
      break;
    }
    parts.push(text.slice(cursor, start), " ");
    cursor = end + 1;
  }
  return parts.join("");
}

function instructionalText(markdown: string): string {
  return stripAngleTags(stripMarkdownLinks(stripHtmlComments(markdown))).replace(
    /[`*_>#|~]/g,
    " ",
  );
}

function countWords(text: string): number {
  return text.match(/[\p{L}\p{N}]+(?:['’\-][\p{L}\p{N}]+)*/gu)?.length ?? 0;
}

function countPromptLines(markdown: string): number {
  const imperative = /^(?:say|repeat|answer|choose|write|read|translate|recall|practice|produce|ask|respond|point|cover|rebuild|listen|speak|try)\b/i;
  let count = 0;
  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine
      .replace(/^\s*(?:#{1,6}|[-*+] |\d+[.)] )\s*/, "")
      .trim();
    if (line === "") continue;
    if (/[?؟]/u.test(line) || imperative.test(line)) count += 1;
  }
  return count;
}

function explicitPauseSeconds(markdown: string): number {
  const pattern = /\b(?:pause|wait)\s+(?:for\s+)?(\d+)\s*(?:seconds?|secs?|s)\b/giu;
  let total = 0;
  for (const match of markdown.matchAll(pattern)) total += Number(match[1]);
  return total;
}

/** Estimate a lesson independently from its author-declared duration. */
export function estimateLessonDuration(lesson: ParsedLesson): DurationEstimate {
  const text = instructionalText(lesson.body);
  const wordCount = countWords(text);
  const promptCount = countPromptLines(stripHtmlComments(lesson.body));
  const repeatCueCount = text.match(/\b(?:repeat|again|twice|three\s+times)\b/giu)?.length ?? 0;
  const pauseSeconds = explicitPauseSeconds(lesson.body);
  const authoredAudioSeconds = Math.ceil(
    nonNegativeNumber(stringValue(lesson.frontmatter.audio_duration_seconds)) ??
      nonNegativeNumber(stringValue(lesson.frontmatter.audio_seconds)) ??
      0,
  );
  const spokenSeconds = Math.ceil((wordCount / SPOKEN_WORDS_PER_MINUTE) * 60);
  const activityResponseSeconds = lesson.blocks.reduce(
    (total, block) =>
      total + (block.activities ?? []).reduce((sum, activity) => sum + activity.responseSeconds, 0),
    0,
  );
  const responseSeconds = promptCount * RESPONSE_SECONDS_PER_PROMPT + activityResponseSeconds;
  const repeatSeconds = repeatCueCount * REPEAT_CUE_SECONDS;
  const subtotal = Math.max(spokenSeconds, authoredAudioSeconds) + responseSeconds + repeatSeconds + pauseSeconds;
  const computedSeconds = Math.ceil(subtotal * (1 + SAFETY_MARGIN));
  const declaredSeconds = declaredDurationSeconds(lesson);
  const effectiveSeconds = Math.max(declaredSeconds, computedSeconds);
  const reasons: DurationEstimate["reasons"] = [];
  if (declaredSeconds >= DURATION_THRESHOLD_SECONDS) reasons.push("declared");
  if (computedSeconds >= DURATION_THRESHOLD_SECONDS) reasons.push("computed");

  return {
    lessonId: lesson.realization.lessonId,
    language: lesson.language,
    chapter: Number.isFinite(lesson.realization.chapter) ? lesson.realization.chapter : null,
    declaredSeconds,
    computedSeconds,
    effectiveSeconds,
    wordCount,
    promptCount,
    repeatCueCount,
    explicitPauseSeconds: pauseSeconds,
    authoredAudioSeconds,
    activityResponseSeconds,
    reasons,
  };
}

function sortedNumbers(values: Iterable<number>): number[] {
  return [...new Set(values)].sort((a, b) => a - b);
}

function bookCoverage(
  language: string,
  lessons: ParsedLesson[],
  books: BookCorpus,
): BookCoverage {
  const lessonChapters = sortedNumbers(
    lessons
      .filter((lesson) => lesson.language === language && Number.isFinite(lesson.realization.chapter))
      .map((lesson) => lesson.realization.chapter),
  );
  const book = books.books.find((candidate) => candidate.language === language);
  const bookChapters = sortedNumbers(book?.chapters.map((chapter) => chapter.chapter) ?? []);
  const bookSet = new Set(bookChapters);
  const lessonSet = new Set(lessonChapters);
  const covered = lessonChapters.filter((chapter) => bookSet.has(chapter)).length;
  return {
    language,
    hasBook: book !== undefined,
    lessonChapters,
    bookChapters,
    missingBookChapters: lessonChapters.filter((chapter) => !bookSet.has(chapter)),
    orphanBookChapters: bookChapters.filter((chapter) => !lessonSet.has(chapter)),
    coveragePercent: lessonChapters.length === 0 ? 100 : Math.round((covered / lessonChapters.length) * 100),
  };
}

function schemaCoverage(language: string, lessons: ParsedLesson[]): TrackSchemaCoverage {
  const trackLessons = lessons.filter((lesson) => lesson.language === language);
  const versions: Record<string, number> = {};
  for (const lesson of trackLessons) {
    const authored = stringValue(lesson.frontmatter.schema_version).trim();
    const version = authored === "" ? "1" : authored;
    versions[version] = (versions[version] ?? 0) + 1;
  }
  const version2 = versions["2"] ?? 0;
  const status: TrackSchemaStatus =
    trackLessons.length > 0 && version2 === trackLessons.length
      ? "version-2"
      : version2 === 0
        ? "legacy"
        : "mixed";
  return { language, status, lessonCount: trackLessons.length, versions };
}

/** Build a deterministic, machine-readable snapshot of known migration gaps. */
export function buildCurriculumGapReport(input: CurriculumGapReportInput): CurriculumGapReport {
  const { registry, lessons, books } = input;
  const lessonIds = new Set(lessons.map((lesson) => lesson.realization.lessonId));
  const durationViolations = lessons
    .map(estimateLessonDuration)
    .filter((estimate) => estimate.effectiveSeconds >= DURATION_THRESHOLD_SECONDS)
    .sort((a, b) => a.language.localeCompare(b.language) || a.lessonId.localeCompare(b.lessonId));

  const unknown: UnknownPrerequisite[] = [];
  const roots: PrerequisiteRoot[] = [];
  for (const lesson of lessons) {
    const prerequisites = stringList(lesson.frontmatter.prerequisites);
    const root = {
      lessonId: lesson.realization.lessonId,
      language: lesson.language,
      chapter: Number.isFinite(lesson.realization.chapter) ? lesson.realization.chapter : null,
    };
    if (prerequisites.length === 0) roots.push(root);
    for (const prerequisite of prerequisites) {
      if (!lessonIds.has(prerequisite)) {
        unknown.push({ lessonId: lesson.realization.lessonId, language: lesson.language, prerequisite });
      }
    }
  }
  roots.sort((a, b) => a.language.localeCompare(b.language) || a.lessonId.localeCompare(b.lessonId));
  unknown.sort((a, b) => a.language.localeCompare(b.language) || a.lessonId.localeCompare(b.lessonId));
  const laterChapterWithoutPrerequisites = roots.filter((root) => (root.chapter ?? 0) > 1);

  const coverage = registry.languages.map((language) => bookCoverage(language.id, lessons, books));
  const schemas = registry.languages.map((language) => schemaCoverage(language.id, lessons));
  const modality = summarizeModality(lessons, input.modality ?? {});
  const chaptersWithoutDrivablePrefix = modality.tracks.reduce(
    (sum, track) =>
      sum + track.chapters.filter((chapter) => chapter.drivablePrefix === 0).length,
    0,
  );

  // HL05 chapter gates. Only run when the caller supplied both the ledgers and the
  // policy: the representativeness rule is meaningless without its threshold, and a
  // silent default would publish a number measured at a floor nobody chose.
  const levels =
    input.curricula && input.spine
      ? summarizeLevels(lessons, input.curricula, input.spine)
      : undefined;

  // HL08 ramp budgets. Needs only the policy — every lesson carries its own atoms and
  // its own script, so unlike the chapter gates this does not wait on the ledgers.
  const ramp = input.chapterPolicy ? measureRamp(lessons, input.chapterPolicy) : undefined;
  const continuity = measureContinuity(lessons);
  const scriptClosure = measureScriptClosure(lessons);
  const writingStages =
    input.assessmentPolicy && input.curricula && input.spine
      ? measureWritingStages(
          input.assessmentPolicy,
          registry.languages.map((language) => language.id),
          lessons,
          input.curricula,
          input.spine,
        )
      : undefined;
  const levelGate =
    levels && ramp && input.curricula && input.spine
      ? runLevelGate({
          lessons,
          levels,
          curricula: input.curricula,
          spine: input.spine,
          ramp,
          continuity,
          writingStages,
        })
      : undefined;

  const chapterGates =
    input.trackChapters && input.chapterPolicy
      ? runChapterGates({
          books,
          lessons,
          trackChapters: input.trackChapters,
          policy: input.chapterPolicy,
        })
      : undefined;

  return {
    schemaVersion: 1,
    durationModel: {
      version: 2,
      thresholdSeconds: DURATION_THRESHOLD_SECONDS,
      spokenWordsPerMinute: SPOKEN_WORDS_PER_MINUTE,
      learnerResponseSecondsPerPrompt: RESPONSE_SECONDS_PER_PROMPT,
      repeatCueSeconds: REPEAT_CUE_SECONDS,
      safetyMarginPercent: SAFETY_MARGIN * 100,
      audioDuration: "max(spoken estimate, authored audio seconds)",
      effectiveDuration: "max(declared, computed)",
    },
    summary: {
      registeredTracks: registry.languages.length,
      totalLessons: lessons.length,
      authoredBooks: books.books.length,
      durationViolations: durationViolations.length,
      unknownPrerequisites: unknown.length,
      laterChapterLessonsWithoutPrerequisites: laterChapterWithoutPrerequisites.length,
      tracksWithoutBooks: coverage.filter((track) => !track.hasBook).length,
      lessonChaptersWithoutBooks: coverage.reduce((sum, track) => sum + track.missingBookChapters.length, 0),
      legacySchemaTracks: schemas.filter((track) => track.status === "legacy").length,
      mixedSchemaTracks: schemas.filter((track) => track.status === "mixed").length,
      version2SchemaTracks: schemas.filter((track) => track.status === "version-2").length,
      lessonsWithoutLevel: levels?.unmapped ?? null,
      rampOverBudgetLessons: ramp?.summary.lessonViolations ?? null,
      scriptRampOverBudgetLessons: ramp?.script.summary.lessonViolations ?? null,
      lessonsOpeningMultipleScripts: ramp?.script.summary.systemViolations ?? null,
      scriptClosureViolations: scriptClosure.summary.violations,
      scriptExposureExemptedGlyphs: scriptClosure.summary.exposureExemptedGlyphs,
      tracksTeachingNoScript: scriptClosure.summary.tracksTeachingNothing,
      headwordsWithoutRomanization: scriptClosure.summary.headwordsWithoutRomanization,
      lessonsWithoutSequence: continuity.summary.lessonsWithoutSequence,
      forwardReviews: continuity.summary.forwardReviews,
      atomsNeverRevisited: continuity.summary.atomsNeverRevisited,
      forwardReferences: continuity.summary.forwardReferences,
      chaptersWithoutCapability: chapterGates?.summary.chaptersWithoutCapability ?? null,
      chapterPayoffsNotRepresentative: chapterGates?.summary.payoffsNotRepresentative ?? null,
      chapterGateCleanTracks: chapterGates?.summary.cleanTracks ?? null,
      tracksWithoutWritingStageEvidence: writingStages
        ? writingStages.summary.tracks - writingStages.summary.tracksWithAnyEvidence
        : null,
      missingWritingStagePairs: writingStages?.summary.missingTrackLevelStages ?? null,
      invalidWritingStageEvidence: writingStages?.summary.invalidEvidenceBlocks ?? null,
      drivableLessons: modality.coreVoice,
      drivablePercent: modality.drivablePercent,
      chaptersWithoutDrivablePrefix,
      unexplainedModalityOverrides: modality.findings.filter(
        (finding) => finding.code === "modality-unexplained-override",
      ).length,
    },
    duration: { violations: durationViolations },
    prerequisites: { unknown, roots, laterChapterWithoutPrerequisites },
    books: { tracks: coverage },
    schemas: { tracks: schemas },
    chapters: chapterGates,
    levels,
    ramp,
    continuity,
    scriptClosure,
    writingStages,
    levelGate,
    modality,
  };
}

/** Compact text companion for CI logs and human review. */
export function renderCurriculumGapReport(report: CurriculumGapReport): string {
  const summary = report.summary;
  const lines = [
    "Human Languages curriculum gap report",
    "====================================",
    `${summary.registeredTracks} tracks, ${summary.totalLessons} lessons, ${summary.authoredBooks} books`,
    `${summary.durationViolations} lessons at or above ${report.durationModel.thresholdSeconds} effective seconds`,
    `${summary.unknownPrerequisites} unknown prerequisites; ${summary.laterChapterLessonsWithoutPrerequisites} later-chapter lessons without prerequisites`,
    `${summary.tracksWithoutBooks} tracks without books; ${summary.lessonChaptersWithoutBooks} lesson chapters without book chapters`,
    `${summary.legacySchemaTracks} legacy, ${summary.mixedSchemaTracks} mixed, ${summary.version2SchemaTracks} version-2 schema tracks`,
    `${summary.drivableLessons} drivable lessons (${summary.drivablePercent}% of the corpus)`,
    ...(report.levels
      ? [
          `levels: ${CEFR_LEVELS.filter((l) => report.levels!.byLevel[l] > 0)
            .map((l) => `${report.levels!.byLevel[l]} ${l}`)
            .join(", ")}; ${report.levels!.unmapped} unmapped (${report.levels!.mappedPercent}% placed)`,
        ]
      : []),
    ...(report.ramp
      ? [
          `ramp: ${report.ramp.summary.lessonViolations} lessons above ${report.ramp.policy.maxNewAtomsPerLesson} new atoms ` +
            `(${report.ramp.summary.unmeasurableLessons} unmeasurable, ${report.ramp.summary.measurablePercent}% measurable); ` +
            `${report.ramp.summary.chapterViolations} chapters above ${report.ramp.policy.maxNewAtomsPerChapter}`,
          `script ramp: ${report.ramp.script.summary.lessonViolations} lessons above ` +
            `${report.ramp.script.policy.maxNewGlyphsPerLesson} new glyphs; ` +
            `${report.ramp.script.summary.systemViolations} opening more than ` +
            `${report.ramp.script.policy.maxNewScriptSystemsPerLesson} writing system at once` +
            (report.ramp.script.summary.steepestLesson
              ? `; steepest ${report.ramp.script.summary.steepestLesson.lessonId} at ${report.ramp.script.summary.steepestLesson.glyphs}`
              : ""),
          `cousin layer: ${report.ramp.script.summary.lessonsWithForeignScript} lessons show another script ` +
            `(max ${report.ramp.script.summary.maxForeignGlyphsInALesson} glyphs) — context, never charged to the budget`,
        ]
      : []),
    `script closure: ${report.scriptClosure.summary.violations} lessons ask the reader to decode ` +
      `a glyph nobody taught them, across ${report.scriptClosure.summary.tracksWithScript} non-Latin tracks; ` +
      `${report.scriptClosure.summary.tracksTeachingNothing} of those tracks teach NO letters at all`,
    `exposure: ${report.scriptClosure.summary.headwordsWithoutRomanization} native-script headwords carry no ` +
      `romanization, so they are load-bearing rather than exposure; the rule exempts ` +
      `${report.scriptClosure.summary.exposureExemptedGlyphs} glyphs and makes ` +
      `${report.scriptClosure.summary.exposureOnly} lessons clean on its own` +
      (report.scriptClosure.summary.tracksWithUnknownScript > 0
        ? `; ${report.scriptClosure.summary.tracksWithUnknownScript} tracks UNMEASURED (unknown script: ` +
          `${report.scriptClosure.unknownScriptTracks.join(", ")})`
        : ""),
    `order: ${report.continuity.summary.lessonsWithoutSequence} lessons with no declared sequence ` +
      `across ${report.continuity.summary.tracksWithUnorderedLessons} tracks; ` +
      `${report.continuity.summary.forwardPrerequisites} prerequisites and ` +
      `${report.continuity.summary.forwardReviews} reviews pointing forward`,
    `reinforcement: ${report.continuity.summary.atomsNeverRevisited} of ${report.continuity.summary.atomsTaught} ` +
      `atoms never revisited (${report.continuity.summary.neverRevisitedPercent}%); missed windows ` +
      Object.entries(report.continuity.summary.missedByWindow)
        .map(([name, count]) => `${name} ${count}`)
        .join(", "),
    `forward references: ${report.continuity.summary.forwardReferences} uses of material a later lesson teaches`,
    ...(report.writingStages
      ? [
          `writing stages: ${report.writingStages.summary.tracksCompleteAtPreA1}/${report.writingStages.summary.tracks} ` +
            `tracks prove the cumulative pre-A1 ladder; ${report.writingStages.summary.evidenceBlocks} evidence blocks, ` +
            `${report.writingStages.summary.invalidEvidenceBlocks} invalid; ` +
            `${report.writingStages.summary.missingTrackLevelStages} missing (track, level, stage) pairs`,
        ]
      : []),
    ...(report.levelGate
      ? [
          // VOCABULARY FIRST, and deliberately so (HL-C183). Spine coverage was
          // being quoted as the headline for a corpus that had not cleared
          // pre-A1: Spanish read "33/33 spine nodes" while teaching 267 words
          // against a 300-word pre-A1 floor and a 16,000-word C2 one. Spine
          // coverage measures FUNCTIONAL reach — a rung per can-do statement.
          // This measures whether there are enough words to stand on the rungs.
          // Printing it above the level line is what stops the wrong number
          // being read as completeness.
          `VOCABULARY vs HL09 §3.1 targets: ` +
            [...report.levelGate.tracks]
              .sort((a, b) => b.vocabulary - a.vocabulary || a.language.localeCompare(b.language))
              .slice(0, 5)
              .map((t) => {
                const next = t.inProgressAt ?? "C2";
                const target = report.levelGate!.vocabularyTargets[next];
                // TWO numbers, because printing only the first one misled a whole
                // session (HL-C195). `vocabulary` is the track's TOTAL headword
                // count; the level gate's criterion counts only headwords taught
                // AT OR BELOW the level in progress. Spanish read "227/300" while
                // the blocker said 48 — and sixteen new words on an A1 node moved
                // the blocker by zero. The second number is the one to author
                // against; the first is context.
                const atLevel = t.blockers.find((b) => b.criterion === "vocabulary");
                const scoped = atLevel ? target - atLevel.shortfall : t.vocabulary;
                return `${t.language} ${scoped}/${target} at-or-below ${next} (${t.vocabulary} total)`;
              })
              .join(", ") +
            `; ${report.levelGate.tracks.filter((t) => {
              const next = t.inProgressAt ?? "C2";
              return t.vocabulary < report.levelGate!.vocabularyTargets[next];
            }).length} of ${report.levelGate.tracks.length} tracks short of the level they are working on`,
          `levels ATTAINED (HL09 §3.1): ${CEFR_LEVELS.filter((l) => report.levelGate!.summary.attainedByLevel[l] > 0)
            .map((l) => `${report.levelGate!.summary.attainedByLevel[l]} tracks at ${l}`)
            .join(", ") || "none"}; ` +
            `${report.levelGate!.summary.tracksOverstating} track(s) touch a level they have not attained`,
        ]
      : []),
    ...(report.chapters
      ? [
          `${report.chapters.summary.chaptersWithoutCapability} of ${report.chapters.summary.bookChapters} book chapters without an HL05 capability; ` +
            `${report.chapters.summary.payoffsNotRepresentative} payoffs below the ${report.chapters.summary.payoffRepresentativeness} representativeness floor; ` +
            `${report.chapters.summary.cleanTracks} tracks already clean`,
        ]
      : []),
    "",
    "Longest effective lessons:",
  ];
  for (const lesson of [...report.duration.violations]
    .sort((a, b) => b.effectiveSeconds - a.effectiveSeconds || a.lessonId.localeCompare(b.lessonId))
    .slice(0, 20)) {
    lines.push(
      `  ${lesson.lessonId}: ${lesson.effectiveSeconds}s ` +
        `(declared ${lesson.declaredSeconds}s, computed ${lesson.computedSeconds}s; ${lesson.reasons.join("+")})`,
    );
  }
  lines.push("", ...renderModalitySection(report.modality));
  lines.push("");
  return `${lines.join("\n")}\n`;
}

/**
 * The HL08 modality section: "how much of this can I do in the car?"
 *
 * Whole-corpus percentage first, then per-track counts, then the chapter numbers a
 * commuter plans around. The full per-chapter prefix table lives in the JSON view —
 * there are hundreds of chapters — so the text companion prints each track's totals
 * and then names only the chapters whose prefix is 0, which are the ones that cannot
 * be started in the car at all and therefore the ones worth remediating first.
 */
function renderModalitySection(modality: CurriculumGapReport["modality"]): string[] {
  const lines = [
    "Modality (HL08) — derived from lesson type and block structure, never from `skills`:",
    `  ${modality.voice} voice, ${modality.sight} sight, ${modality.pen} pen ` +
      `of ${modality.totalLessons} lessons; ${modality.drivablePercent}% drivable`,
    `  tables of more than ${modality.maxLinearisableTableColumns} column(s) count as sight`,
    // The book prints every block, so the voice/sight/pen counts above describe the
    // book. Drivability is counted on the CORE — the lesson minus its detachable
    // writing segments — because that is what a hands-free view can deliver. The two
    // numerators are identical until a track carries an interspersed writing segment,
    // and this line is where the difference becomes visible instead of silent.
    `  ${modality.coreVoice} lessons have a voice CORE; ` +
      `${modality.lessonsWithWritingSegments} carry a detachable writing segment ` +
      `(${modality.coreVoice - modality.voice} rescued for the hands-free view)`,
  ];
  for (const track of modality.tracks) {
    const rescued = track.coreVoice - track.voice;
    lines.push(
      `  ${track.language}: ${track.voice} voice, ${track.sight} sight, ${track.pen} pen; ` +
        `${track.drivablePercent}% drivable; ${track.chapters.length} chapters, ` +
        `${track.drivablePrefixTotal} lessons reachable in chapter-prefix order` +
        (rescued > 0 ? `; ${rescued} rescued by a detachable writing segment` : ""),
    );
  }

  const blocked = modality.tracks.flatMap((track) =>
    track.chapters.filter((chapter) => chapter.drivablePrefix === 0),
  );
  lines.push("", `Chapters that cannot be started by ear (drivable prefix 0): ${blocked.length}`);
  for (const chapter of blocked.slice(0, 20)) {
    lines.push(
      `  ${chapter.language} ch${chapter.chapter}: 0 of ${chapter.lessonCount} ` +
        `(first blocker ${chapter.firstNonVoiceLesson ?? "n/a"})`,
    );
  }

  lines.push("", `Modality findings (report-only): ${modality.findings.length}`);
  for (const finding of modality.findings.slice(0, 20)) {
    lines.push(`  ${clean(finding.code)}: ${clean(finding.message)}`);
  }
  return lines;
}
