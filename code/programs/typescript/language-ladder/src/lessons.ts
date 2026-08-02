// ---------------------------------------------------------------------------
// lessons.ts — the bridge from the written curriculum to the study app.
//
// Until now this app only knew about *letters*: it read `data/scripts/*.json`
// and drilled glyphs. But the repository also contains ~670 hand-written lesson
// files — every one carrying `id`, `concept_tag`, `prerequisites` and
// `reviews_of` in its frontmatter. That is a complete study corpus, and the
// package `@coding-adventures/human-language-data` already knows how to parse
// it. It simply had no consumers. This module is the missing wire.
//
// WHY NOT JUST CALL THE PACKAGE'S LOADER?
// Because `loader.ts` reads the disk with `node:fs`, and we run in a browser.
// The package is layered for exactly this: `parseLesson` is a PURE function
// from file *contents* to a parsed lesson. So we let Vite hand us the contents
// at build time (`import.meta.glob`, below) and call the pure layer ourselves.
// No filesystem in the browser, and no second copy of the parsing rules.
// ---------------------------------------------------------------------------

// NOTE the deep import. The package's barrel (`index.ts`) also re-exports
// `loader.ts` (node:fs) and `cli.ts` (process), and a bundler cannot always
// tree-shake those away — importing the barrel produced a browser build that
// loaded fine and then died at startup with "process is not defined". Reaching
// straight for the pure parsing module keeps Node out of the browser bundle.
import { parseLesson, type ParsedLesson } from "@coding-adventures/human-language-data/src/parse.ts";

/**
 * Every lesson markdown file, as raw text, keyed by path. Vite inlines these at
 * build time — `eager` so there is no async waterfall on first paint, and
 * `?raw` because we want the source, not a module.
 *
 * The glob is relative to THIS file; `../../../../learning/...` is the same
 * path `data.ts` already uses to reach the curriculum.
 */
const LESSON_SOURCES = import.meta.glob(
  "../../../../learning/human-languages/*/lessons/*.md",
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;

/** A lesson, ready for the UI and the scheduler. */
export interface Lesson {
  /** Stable slug id from frontmatter, e.g. "ES-C17-futuro". The persistence key. */
  id: string;
  /** Track directory name, e.g. "spanish". */
  language: string;
  headword: string;
  gloss: string;
  /** word | phrase | practice | practice-mix | writing | review */
  type: string;
  chapter: number;
  /** "" when the lesson carries none (writing lessons are exempt). */
  concept: string;
  prerequisites: string[];
  reviewsOf: string[];
  /**
   * Etymological roots this lesson cites (e.g. `["bonus", "dies"]`). The join
   * key for cross-language CONNECTIONS: two lessons in different languages that
   * share a root are etymologically linked. `[]` when the lesson cites none.
   */
  roots: string[];
  /** Latin-script reading; equals `headword` for Latin-script tracks. */
  romanization: string;
  /** Script slug the package assigned from the track name. */
  script: string;
  /** ≤120-char memory anchor; "" when not yet authored. */
  etymologyHook: string;
  /** Lossless authored lesson Markdown, shared with the book corpus. */
  body: string;
  /** The author's upper bound for this micro-lesson. */
  estMinutes: number;
}

/**
 * Pull the track name out of a curriculum path.
 *
 *   ".../learning/human-languages/spanish/lessons/ES-C17-futuro.md" → "spanish"
 *
 * Returns "" if the path doesn't have the expected shape, so a stray file can
 * never masquerade as a language.
 */
export function languageFromPath(path: string): string {
  const match = /\/human-languages\/([^/]+)\/lessons\//.exec(path);
  return match?.[1] ?? "";
}

/**
 * Turn one parsed lesson plus its id into the app's view of it.
 *
 * `parseLesson` gives us a `Realization` (the taxonomy's view — concept,
 * headword, type…) plus the raw frontmatter. The id lives only in the
 * frontmatter, and it is the one field we cannot do without: it is what we key
 * saved progress by, so it must survive lessons being added, renamed or
 * reordered. A lesson with no id is skipped rather than guessed at.
 */
export function toLesson(parsed: ParsedLesson): Lesson | null {
  const fm = parsed.frontmatter;
  const id = typeof fm.id === "string" ? fm.id.trim() : "";
  if (id === "") return null;

  const r = parsed.realization;
  const arr = (v: unknown): string[] =>
    Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
  const maxSeconds =
    typeof fm["duration.max_seconds"] === "string"
      ? Number(fm["duration.max_seconds"])
      : undefined;
  const estMinutes =
    maxSeconds !== undefined && Number.isFinite(maxSeconds)
      ? Math.ceil(maxSeconds / 60)
      : Number(typeof fm.est_minutes === "string" ? fm.est_minutes : 0);

  return {
    id,
    language: parsed.language,
    headword: r.headword,
    gloss: r.gloss,
    type: r.type,
    chapter: r.chapter,
    concept: r.concept,
    prerequisites: arr(fm.prerequisites),
    reviewsOf: arr(fm.reviews_of),
    roots: arr(fm.roots),
    romanization: r.romanization,
    script: r.script,
    etymologyHook: r.etymologyHook,
    body: parsed.body,
    estMinutes,
  };
}

/**
 * Parse every lesson in the curriculum, sorted by id so the order — and
 * therefore every scheduler index — is deterministic across builds. That
 * determinism matters: see `progress.ts`, which nonetheless refuses to rely on
 * it for saved data.
 */
export function loadLessons(
  sources: Record<string, string> = LESSON_SOURCES,
): Lesson[] {
  const out: Lesson[] = [];
  for (const [path, source] of Object.entries(sources)) {
    const language = languageFromPath(path);
    if (language === "") continue;
    const lesson = toLesson(parseLesson(source, language));
    if (lesson) out.push(lesson);
  }
  return out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** The distinct track names present, sorted. */
export function languagesOf(lessons: Lesson[]): string[] {
  return [...new Set(lessons.map((l) => l.language))].sort();
}

/** Minimum of a scheduler item that `nextDue` needs. Keeps this module free of
 *  a scheduler import, and keeps the function trivially testable. */
export interface DueLike {
  dueAtSession: number;
}

/** Where `nextDue` landed: the lesson to show, and the cursor to carry forward. */
export interface NextDue {
  /** Index into the lessons array, or null if nothing is due. */
  index: number | null;
  /** The advanced cursor — pass it back in on the next call. */
  cursor: number;
}

/**
 * Find the next due lesson, scanning forward from a ROTATING cursor over the
 * interleaved (round-robin-by-language) order.
 *
 * Why a cursor rather than "first due from the front"? Two reasons, both found
 * by using it:
 *
 *   * Leitner boxes 0 and 1 fall due again after a single session, so a
 *     from-the-front scan hands back the lesson you just answered, forever.
 *   * Restarting also defeats the interleaving — you would sit in whichever
 *     language sorts first instead of moving across them.
 *
 * The scan visits every pool position exactly once (`pool.length` steps, modulo
 * arithmetic), so it always terminates and never skips. `null` means nothing is
 * due; the caller decides what to do about that.
 */
export function nextDue(
  groups: number[][],
  pool: { scriptIndex: number; letterIndex: number }[],
  schedule: DueLike[],
  session: number,
  cursor: number,
  /**
   * Optional filter, applied DURING the scan.
   *
   * This is how prerequisite gating enters the rotation. Filtering afterwards
   * — picking, then rejecting, then substituting a fallback — collapses to
   * serving that one fallback over and over, because the same pick is rejected
   * every time. Skipping unacceptable indices *inside* the scan keeps the
   * cursor advancing and the rotation intact.
   */
  accept: (index: number) => boolean = () => true,
): NextDue {
  if (pool.length === 0) return { index: null, cursor };
  let at = cursor;
  for (let step = 0; step < pool.length; step++) {
    at = (at + 1) % pool.length;
    const entry = pool[at]!;
    const index = groups[entry.scriptIndex]?.[entry.letterIndex];
    if (index === undefined) continue;
    if (!accept(index)) continue;
    const state = schedule[index];
    if (state && state.dueAtSession <= session) return { index, cursor: at };
  }
  return { index: null, cursor: at };
}

/**
 * Group lesson indices by language, preserving each language's internal order.
 * This is the shape `interleave.buildPool` wants: it already knows how to
 * round-robin across groups, so cross-LANGUAGE interleaving comes free from the
 * code that was written for cross-SCRIPT interleaving.
 */
export function indicesByLanguage(lessons: Lesson[]): number[][] {
  const languages = languagesOf(lessons);
  const position = new Map(languages.map((l, i) => [l, i]));
  const groups: number[][] = languages.map(() => []);
  lessons.forEach((lesson, index) => {
    const g = position.get(lesson.language);
    if (g !== undefined) groups[g]!.push(index);
  });
  return groups;
}
