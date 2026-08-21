import type { ParsedLesson } from "./parse.js";
import type {
  BookCorpus,
  LanguageCurriculum,
  LanguageRegistry,
} from "./types.js";

/** The small part of generated-book-hashes.json needed by the progress table. */
export interface GeneratedBookChapterRef {
  language: string;
  chapter: number;
}

/** One registry-ordered, entirely derived track summary. */
export interface TrackProgress {
  id: string;
  name: string;
  family: string;
  script: string;
  canonicalLessons: number;
  mappedLessons: number;
  bookChapters: number;
  latestBookChapter?: number;
  generatedBookChapters: number;
}

function addToSetMap(map: Map<string, Set<number>>, key: string, value: number): void {
  const values = map.get(key) ?? new Set<number>();
  values.add(value);
  map.set(key, values);
}

/**
 * Derive progress from the same registry, lessons, realization maps, and generated
 * book manifest consumed by the app and publication job.
 *
 * Registry order is load-bearing: it is the authored default mixed-language walk,
 * and using it here guarantees that a newly registered language appears in the README
 * even before it has a lesson or book chapter.
 */
export function buildTrackProgress(
  registry: LanguageRegistry,
  lessons: ParsedLesson[],
  curricula: LanguageCurriculum[],
  books: BookCorpus,
  generatedBookChapters: GeneratedBookChapterRef[],
): TrackProgress[] {
  const lessonCounts = new Map<string, number>();
  for (const lesson of lessons) {
    lessonCounts.set(lesson.language, (lessonCounts.get(lesson.language) ?? 0) + 1);
  }

  const mappedLessonIds = new Map<string, Set<string>>();
  for (const curriculum of curricula) {
    const ids = new Set<string>();
    for (const segment of curriculum.path) {
      for (const lessonId of segment.lessons) ids.add(lessonId);
    }
    for (const extension of curriculum.extensions) {
      for (const lessonId of extension.lessons) ids.add(lessonId);
    }
    mappedLessonIds.set(curriculum.language, ids);
  }

  const authoredChapterNumbers = new Map<string, Set<number>>();
  for (const book of books.books) {
    for (const chapter of book.chapters) {
      addToSetMap(authoredChapterNumbers, book.language, chapter.chapter);
    }
  }

  const generatedChapterNumbers = new Map<string, Set<number>>();
  for (const chapter of generatedBookChapters) {
    addToSetMap(generatedChapterNumbers, chapter.language, chapter.chapter);
  }

  return registry.languages.map((language) => {
    const authored = authoredChapterNumbers.get(language.id) ?? new Set<number>();
    const generated = generatedChapterNumbers.get(language.id) ?? new Set<number>();
    const latestBookChapter = authored.size === 0 ? undefined : Math.max(...authored);
    return {
      id: language.id,
      name: language.name,
      family: language.family,
      script: language.script,
      canonicalLessons: lessonCounts.get(language.id) ?? 0,
      mappedLessons: mappedLessonIds.get(language.id)?.size ?? 0,
      bookChapters: authored.size,
      latestBookChapter,
      generatedBookChapters: generated.size,
    };
  });
}

function titleCaseScript(script: string): string {
  return script
    .split("-")
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join("-");
}

function bookProgress(track: TrackProgress): string {
  if (track.latestBookChapter === undefined) return "No authored chapters";
  const chapterWord = track.bookChapters === 1 ? "chapter" : "chapters";
  return `${track.bookChapters} ${chapterWord}; through Ch. ${track.latestBookChapter}; ${track.generatedBookChapters} generated`;
}

/** One independently mergeable Markdown progress card. */
export function renderTrackProgressCard(track: TrackProgress): string {
  return [
    `# ${track.name} progress`,
    "",
    `- Track: [${track.name}](../${track.id}/README.md)`,
    `- Family / script: ${track.family} / ${titleCaseScript(track.script)}`,
    `- Canonical lessons: ${track.canonicalLessons}`,
    `- Mapped lessons: ${track.mappedLessons}`,
    `- Book progress: ${bookProgress(track)}`,
    "",
    "This file is generated from canonical curriculum data. Do not edit it by hand.",
    "",
  ].join("\n");
}

/** Render only facts that can be recomputed; prose distinctions belong below the table. */
export function renderTrackProgressTable(tracks: TrackProgress[]): string {
  const lines = [
    "| Language | Family / script | Canonical lessons | Mapped lessons | Book progress |",
    "|---|---|---:|---:|---|",
  ];
  for (const track of tracks) {
    lines.push(
      `| [${track.name}](./${track.id}/README.md) | ${track.family} / ${titleCaseScript(track.script)} | ${track.canonicalLessons} | ${track.mappedLessons} | ${bookProgress(track)} |`,
    );
  }
  return lines.join("\n");
}
