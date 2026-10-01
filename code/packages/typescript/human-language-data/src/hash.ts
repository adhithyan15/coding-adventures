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

const UTF8 = new TextEncoder();
const TWO_32 = 0x1_0000_0000;

/**
 * Browser-safe FNV-1a over UTF-8 bytes, used only as a deterministic drift fingerprint.
 *
 * FNV-1a is two steps per byte, starting from a fixed offset basis:
 *
 *     hash = 0xcbf29ce484222325
 *     for each byte:  hash ^= byte;  hash = (hash * 0x100000001b3) mod 2^64
 *
 * The obvious way to write that in JavaScript is with BigInt. It is correct, and this
 * function was written that way until the corpus reached about 20,000 lessons. By then
 * it was the single most expensive function in the gap report: every lesson is hashed,
 * and each byte allocated fresh BigInts for the xor, the multiply and the truncation.
 * Profiling put about 4s of a 24s report in this loop.
 *
 * So the 64-bit state is held as two unsigned 32-bit halves, `hi` and `lo`, in ordinary
 * numbers. The prime makes the multiply cheap to split, because it is 2^40 + 0x1b3:
 *
 *     hash * prime = hash * 0x1b3  +  hash << 40
 *
 *   - `lo * 0x1b3` is below 2^41, so it is exact in a double. Its low 32 bits are the
 *     new `lo`, and everything above them carries into `hi`.
 *   - `hi * 0x1b3` only matters modulo 2^32, which is exactly what `Math.imul` gives.
 *   - `hash << 40` moves `lo` up by 40 bits. That lands `lo << 8` in the high half,
 *     and pushes `hi` entirely past bit 63, where the modulus discards it.
 *
 * The xor touches only the low byte, so it only ever changes `lo`.
 *
 * The output is the same 16 hex digits as before: `hi`, then `lo`, each padded to 8.
 * The published test vectors in tests/hash.test.ts pin that, and so do the generated
 * book and narration hash ledgers. Every lesson hash in them must come out
 * byte-identical, or those checks fail.
 */
export function fnv1a64(value: string): string {
  let hi = 0xcbf29ce4;
  let lo = 0x84222325;
  const bytes = UTF8.encode(value);
  for (let index = 0; index < bytes.length; index += 1) {
    lo = (lo ^ bytes[index]!) >>> 0;
    const low = lo * 0x1b3;
    hi = (Math.imul(hi, 0x1b3) + Math.floor(low / TWO_32) + (lo << 8)) >>> 0;
    lo = low >>> 0;
  }
  return `fnv1a64:${hi.toString(16).padStart(8, "0")}${lo.toString(16).padStart(8, "0")}`;
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
 * Only the four fields the book actually prints are hashed. Hashing the whole
 * capability would make `payoff.note` — deliberately non-printed tooling prose —
 * regenerate every chapter that carries one, which is churn with no reader-visible
 * cause. The rule: a fingerprint covers what the artifact SHOWS, no more.
 *
 * Before this, `chapters.json` was invisible to the fingerprint. CI still caught a
 * stale chapter, because `book-cli --check` compares full text — but
 * the generated book-hash ledger came out byte-identical, so `language-ladder`'s
 * `bookHashStatus` reported a genuinely stale `.tex` as synced.
 */
export function canonicalChapterHash(
  lessons: ParsedLesson[],
  capability?: {
    title?: string;
    label?: string;
    canDo?: string;
    payoff?: { summary?: string };
  },
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
  capability?: {
    title?: string;
    label?: string;
    canDo?: string;
    payoff?: { summary?: string };
  },
): string {
  const lessonPart = combineLessonHashes(entries);
  const printed = capability
    ? {
        title: capability.title ?? "",
        label: capability.label ?? "",
        canDo: capability.canDo ?? "",
        payoff: capability.canDo ? capability.payoff?.summary ?? "" : "",
      }
    : null;
  // A caller with no chapter capability — narration is the intentional one — hashes
  // exactly as before, so book-only metadata cannot churn an audio artifact.
  return printed === null ? lessonPart : fnv1a64(JSON.stringify({ lessonPart, printed }));
}
