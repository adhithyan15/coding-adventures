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

/**
 * Fingerprint a chapter from its lessons, and from the capability the book prints.
 *
 * `capability` is optional because only the BOOK renders a chapter opening. The
 * narration export builds a spoken script from lessons alone, so it calls this
 * without one and its hashes are unaffected — a capability edit must not churn 789
 * narration files that cannot have changed.
 *
 * Only the two fields the book actually prints are hashed. Hashing the whole
 * capability would make `payoff.note` — deliberately non-printed tooling prose —
 * regenerate every chapter that carries one, which is churn with no reader-visible
 * cause. The rule: a fingerprint covers what the artifact SHOWS, no more.
 *
 * Before this, `chapters.json` was invisible to the fingerprint. CI still caught a
 * stale chapter, because `book-cli --check` compares full text — but
 * `generated-book-hashes.json` came out byte-identical, so `language-ladder`'s
 * `bookHashStatus` reported a genuinely stale `.tex` as synced.
 */
export function canonicalChapterHash(
  lessons: ParsedLesson[],
  capability?: { canDo?: string; payoff?: { summary?: string } },
): string {
  return combineChapterHash(
    lessons.map((lesson) => ({
      id: lesson.realization.lessonId,
      sequence: Number(lesson.frontmatter.sequence),
      sourceHash: lesson.sourceHash,
    })),
    capability,
  );
}

/**
 * The combining step, over already-computed lesson hashes.
 *
 * Exported separately because the BROWSER app must reproduce this exact value and
 * has no `ParsedLesson` — it loads lesson contents through `import.meta.glob`, not
 * through the Node-only loader. `language-ladder` previously reproduced only the
 * lesson half via `combineLessonHashes`, which was fine while that WAS the whole
 * fingerprint; folding the capability in without giving the app a seam to reach it
 * would have turned "always synced" into "always stale" — the same broken signal,
 * inverted.
 */
export function combineChapterHash(
  entries: LessonHashEntry[],
  capability?: { canDo?: string; payoff?: { summary?: string } },
): string {
  const lessonPart = combineLessonHashes(entries);
  // Gated on `canDo`, matching `chapterIntro`'s own condition exactly: a capability
  // with no `canDo` prints NOTHING, so it must hash as though it were absent. The
  // fingerprint tracks what the artifact shows.
  const printed = capability?.canDo
    ? { canDo: capability.canDo, payoff: capability.payoff?.summary ?? "" }
    : null;
  // A chapter with no printed opening hashes exactly as it did before, so adding the
  // opening did not renumber the fingerprints of chapters that have no opening.
  return printed === null ? lessonPart : fnv1a64(JSON.stringify({ lessonPart, printed }));
}
