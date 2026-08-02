import type { ParsedLesson } from "./parse.js";
import type { BookCorpus, LanguageRegistry } from "./types.js";

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
    version: 1;
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
}

export interface CurriculumGapReportInput {
  registry: LanguageRegistry;
  lessons: ParsedLesson[];
  books: BookCorpus;
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
  // The legacy parser flattens the indented `max_seconds` member from the HL04
  // duration block, so both spellings keep the report useful during migration.
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

function instructionalText(markdown: string): string {
  return stripHtmlComments(markdown)
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/<[^>]+>/g, " ")
    .replace(/[`*_>#|~]/g, " ")
    .replace(/^-{3,}\s*$/gm, " ");
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
  const promptCount = countPromptLines(lesson.body);
  const repeatCueCount = text.match(/\b(?:repeat|again|twice|three\s+times)\b/giu)?.length ?? 0;
  const pauseSeconds = explicitPauseSeconds(lesson.body);
  const authoredAudioSeconds = Math.ceil(
    nonNegativeNumber(stringValue(lesson.frontmatter.audio_duration_seconds)) ??
      nonNegativeNumber(stringValue(lesson.frontmatter.audio_seconds)) ??
      0,
  );
  const spokenSeconds = Math.ceil((wordCount / SPOKEN_WORDS_PER_MINUTE) * 60);
  const responseSeconds = promptCount * RESPONSE_SECONDS_PER_PROMPT;
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

  return {
    schemaVersion: 1,
    durationModel: {
      version: 1,
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
    },
    duration: { violations: durationViolations },
    prerequisites: { unknown, roots, laterChapterWithoutPrerequisites },
    books: { tracks: coverage },
    schemas: { tracks: schemas },
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
  lines.push("");
  return `${lines.join("\n")}\n`;
}
