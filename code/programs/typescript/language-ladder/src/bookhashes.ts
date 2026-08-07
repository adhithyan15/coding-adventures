import manifestJson from "../../../../learning/human-languages/core/generated-book-hashes.json";
import { combineChapterHash } from "@coding-adventures/human-language-data/src/hash.ts";
import type { Lesson } from "./lessons.ts";

interface BookHashEntry {
  language: string;
  chapter: number;
  sourceHash: string;
  lessonIds: string[];
  tex: string;
}

const ENTRIES = manifestJson.chapters as BookHashEntry[];

/**
 * The HL05 capability ledgers, loaded the same way lessons are.
 *
 * The chapter fingerprint covers the two capability fields the book PRINTS, so
 * reproducing it needs them. Globbed rather than read through the package loader,
 * which is Node-only — `lessons.ts` makes the same call for the same reason.
 */
const CHAPTER_LEDGERS = import.meta.glob(
  "../../../../learning/human-languages/*/chapters.json",
  { import: "default", eager: true },
) as Record<string, { language: string; chapters: ChapterCapabilityEntry[] }>;

interface ChapterCapabilityEntry {
  chapter: number;
  canDo?: string;
  payoff?: { summary?: string };
}

const CAPABILITIES = new Map<string, ChapterCapabilityEntry>();
for (const ledger of Object.values(CHAPTER_LEDGERS)) {
  for (const entry of ledger.chapters ?? []) {
    CAPABILITIES.set(`${ledger.language}:${entry.chapter}`, entry);
  }
}

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
  return combineChapterHash(
    chapterLessons.map((lesson) => ({
      id: lesson.id,
      sequence: lesson.sequence!,
      sourceHash: lesson.sourceHash!,
    })),
    CAPABILITIES.get(`${language}:${chapter}`),
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
