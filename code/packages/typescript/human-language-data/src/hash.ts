import type { Frontmatter } from "./frontmatter.js";
import type { ParsedLesson } from "./parse.js";

export interface LessonHashEntry {
  id: string;
  sequence: number;
  sourceHash: string;
}

type CanonicalLesson = Pick<
  ParsedLesson,
  "language" | "script" | "frontmatter" | "body" | "preamble" | "blocks"
>;

function sortedFrontmatter(frontmatter: Frontmatter): Frontmatter {
  return Object.fromEntries(
    Object.entries(frontmatter).sort(([left], [right]) => left.localeCompare(right)),
  );
}

/** Browser-safe FNV-1a over UTF-8 bytes, used only as a deterministic drift fingerprint. */
export function fnv1a64(value: string): string {
  let hash = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(value)) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return `fnv1a64:${hash.toString(16).padStart(16, "0")}`;
}

/** Stable serialization of the canonical lesson AST shared by books and the app. */
export function canonicalLessonSource(lesson: CanonicalLesson): string {
  return JSON.stringify({
    language: lesson.language,
    script: lesson.script,
    frontmatter: sortedFrontmatter(lesson.frontmatter),
    body: lesson.body,
    preamble: lesson.preamble,
    blocks: lesson.blocks,
  });
}

export function canonicalLessonHash(lesson: CanonicalLesson): string {
  return fnv1a64(canonicalLessonSource(lesson));
}

/** Combine independently computed lesson hashes into one authored chapter fingerprint. */
export function combineLessonHashes(entries: LessonHashEntry[]): string {
  const ordered = [...entries].sort(
    (left, right) => left.sequence - right.sequence || left.id.localeCompare(right.id),
  );
  return fnv1a64(JSON.stringify(ordered));
}

export function canonicalChapterHash(lessons: ParsedLesson[]): string {
  return combineLessonHashes(
    lessons.map((lesson) => ({
      id: lesson.realization.lessonId,
      sequence: Number(lesson.frontmatter.sequence),
      sourceHash: lesson.sourceHash,
    })),
  );
}
