import { combineChapterHash } from "@coding-adventures/human-language-data/src/hash.ts";
import type { Lesson } from "./lessons.ts";

interface BookHashEntry {
  language: string;
  chapter: number;
  sourceHash: string;
  lessonIds: string[];
  tex: string;
}

interface BookHashManifest {
  chapters: BookHashEntry[];
}

/**
 * The HL05 capability ledgers, loaded LAZILY alongside the manifest.
 *
 * The chapter fingerprint covers the four capability fields the book PRINTS,
 * so reproducing it needs them. These were globbed eagerly, which put all 22
 * tracks' `chapters.json` into one 500 kB chunk — the single largest eager
 * chunk in the app, and the one that broke the ceiling. Like the manifest,
 * they exist only to compute a diagnostic label, so they load after the app.
 */
const CHAPTER_LEDGER_LOADERS = import.meta.glob(
  "../../../../learning/human-languages/*/chapters.json",
  { import: "default" },
) as Record<string, () => Promise<{ language: string; chapters: ChapterCapabilityEntry[] }>>;

interface ChapterCapabilityEntry {
  chapter: number;
  title?: string;
  label?: string;
  canDo?: string;
  payoff?: { summary?: string };
}

/**
 * The book-hash manifest, loaded LAZILY.
 *
 * This file is 136 kB and grows with every chapter in every track. It was a
 * static import, which put all of it in the eager chunk — and the eager chunk
 * has a hard 500 kB ceiling that five chapters of Spanish were enough to break
 * (500,459 bytes). What it buys is one diagnostic word in a metadata line:
 * "book synced" or "book stale". That is not worth 136 kB on first paint.
 *
 * So the manifest arrives after the app does. Until it lands, every chapter
 * reports `not-generated`, which is the same answer the app already gives for
 * a chapter the book has never covered — an honest "I do not know yet" rather
 * than a wrong claim. Call `whenBookHashesReady()` and re-render when it
 * resolves; tests await it in `beforeAll`.
 */
const BOOK_HASH_MANIFEST_LOADERS = import.meta.glob(
  "../../../../learning/human-languages/core/generated-book-hashes/*.json",
  { import: "default" },
) as Record<string, () => Promise<BookHashManifest>>;

let ENTRIES: BookHashEntry[] = [];

const MANIFEST_READY: Promise<void> = Promise.all([
  loadBookHashes(),
  loadCapabilities(),
])
  .then(() => {})
  .catch((error: unknown) => {
    // A failed load is not fatal: the status line degrades to
    // "not-generated" and the rest of the app is unaffected. It is still
    // reported, because a silent catch here once turned a plain ordering bug
    // into an empty capability map that looked like ordinary "no data".
    console.warn("book-hash manifest failed to load; status will read not-generated", error);
    ENTRIES = [];
  });

/** Resolves once the manifest is in memory. Re-render after this to show status. */
export function whenBookHashesReady(): Promise<void> {
  return MANIFEST_READY;
}

const CAPABILITIES = new Map<string, ChapterCapabilityEntry>();

async function loadBookHashes(): Promise<void> {
  const manifests = await Promise.all(
    Object.values(BOOK_HASH_MANIFEST_LOADERS).map((load) => load()),
  );
  ENTRIES = manifests.flatMap((manifest) => manifest.chapters ?? []);
}

async function loadCapabilities(): Promise<void> {
  const ledgers = await Promise.all(
    Object.values(CHAPTER_LEDGER_LOADERS).map((load) => load()),
  );
  for (const ledger of ledgers) {
    for (const entry of ledger.chapters ?? []) {
      CAPABILITIES.set(`${ledger.language}:${entry.chapter}`, entry);
    }
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
