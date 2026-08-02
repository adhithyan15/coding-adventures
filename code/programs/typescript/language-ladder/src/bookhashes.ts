import manifestJson from "../../../../learning/human-languages/core/generated-book-hashes.json";
import { combineLessonHashes } from "@coding-adventures/human-language-data/src/hash.ts";
import type { Lesson } from "./lessons.ts";

interface BookHashEntry {
  language: string;
  chapter: number;
  sourceHash: string;
  lessonIds: string[];
  tex: string;
}

const ENTRIES = manifestJson.chapters as BookHashEntry[];

export type BookHashStatus = "not-generated" | "synced" | "stale";

export function expectedBookHash(language: string, chapter: number): BookHashEntry | undefined {
  return ENTRIES.find((entry) => entry.language === language && entry.chapter === chapter);
}

export function actualChapterHash(
  lessons: Lesson[],
  language: string,
  chapter: number,
): string | undefined {
  const chapterLessons = lessons.filter(
    (lesson) => lesson.language === language && lesson.chapter === chapter,
  );
  if (
    chapterLessons.length === 0 ||
    chapterLessons.some(
      (lesson) => lesson.sourceHash === undefined || lesson.sequence === undefined,
    )
  ) {
    return undefined;
  }
  return combineLessonHashes(
    chapterLessons.map((lesson) => ({
      id: lesson.id,
      sequence: lesson.sequence!,
      sourceHash: lesson.sourceHash!,
    })),
  );
}

/** Compare the browser-loaded lesson AST with the hash embedded in the book. */
export function bookHashStatus(
  lessons: Lesson[],
  language: string,
  chapter: number,
): BookHashStatus {
  const expected = expectedBookHash(language, chapter);
  if (!expected) return "not-generated";
  return actualChapterHash(lessons, language, chapter) === expected.sourceHash ? "synced" : "stale";
}
