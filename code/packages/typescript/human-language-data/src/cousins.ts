// ---------------------------------------------------------------------------
// cousins.ts — the cross-track join behind the generated cousin panel.
//
// HL10 §6.7 wants a reader who knows French to see `hijo · fils · figlio ·
// filho` under a Spanish word, and a reader who does not to see nothing
// missing. This module answers the only hard question in that: given a lesson,
// which lessons in OTHER languages teach a reflex of the same etymon?
//
// WHY `roots:` AND NOT `concept_tag`
// The spec said `concept_tag` until HL-C88 measured what that join returns, and
// the two keys make different claims. A cousin panel asserts *reflexes of the
// same etymon*. `concept_tag` joins lessons that teach the same IDEA, which is
// a different claim and is frequently untrue of the words it pairs:
//
//     concept_tag VERB-GO  ->  ir · andare · aller
//         Spanish ir is from ire, Italian andare from ambitare, and French
//         aller from a third source entirely. Three unrelated verbs.
//
//     roots hora-latin     ->  la hora · heure · ora · hora
//         All four reflexes of hora. A real cousin set.
//
// Generating panels from `concept_tag` would emit false etymology at scale, in
// the one layer of the course whose whole value is that its etymology can be
// trusted. So the join key is `roots:`, and this module will not accept another.
// ---------------------------------------------------------------------------
import type { ParsedLesson } from "./parse.js";

/**
 * The Romance tracks, in the order a panel lists them.
 *
 * Latin is deliberately ABSENT. It is the ancestor, not a sibling, and the
 * etymology block above the panel already names it — printing it again as a
 * "cousin" would misdescribe the relationship the panel exists to show.
 *
 * Catalan is named in the spec and has no track yet; when one lands, adding it
 * here is the whole change.
 */
export const ROMANCE_COUSINS = ["french", "italian", "portuguese"] as const;

/** One sibling word, ready to print. */
export interface Cousin {
  language: string;
  lessonId: string;
  headword: string;
  /** The shared `roots:` slug that justifies the pairing. */
  root: string;
}

/** Every lesson that carries a given root, keyed by root slug. */
export type CousinIndex = ReadonlyMap<string, readonly CousinEntry[]>;

interface CousinEntry {
  language: string;
  lessonId: string;
  headword: string;
  order: [number, number, string];
}

/**
 * The `roots:` slugs a lesson declares.
 *
 * The parser hands this back as an ARRAY, not the raw YAML string — a first
 * draft of this function assumed a string and silently returned nothing for
 * every lesson in the corpus. The string branch is kept because a hand-built
 * fixture can still carry one, and because failing open here would mean a panel
 * that quietly stops appearing rather than a test that fails.
 */
function rootSlugs(lesson: ParsedLesson): string[] {
  const raw = lesson.frontmatter.roots;
  const parts = Array.isArray(raw)
    ? raw.map((slug) => String(slug))
    : typeof raw === "string"
      ? raw.split(",")
      : [];
  return parts.map((slug) => slug.trim().replace(/[[\]]/g, "")).filter((slug) => slug !== "");
}

/**
 * A TOTAL order over candidate lessons: chapter, then sequence, then id.
 *
 * All three parts are load-bearing, and the first draft had only the middle one.
 *
 *   * **chapter first**, because most lessons outside Spanish carry no
 *     `sequence:` at all -- 41 of 105 French lessons have one. Falling back to
 *     "no sequence sorts last" made an unsequenced chapter-1 lesson LOSE to a
 *     sequenced chapter-33 one, which is the opposite of the documented rule.
 *   * **id last**, because without it the sequence-less candidates all tied, and
 *     a tie was resolved by whichever the corpus yielded first -- i.e. by
 *     `readdirSync` order, which differs between filesystems. Reversing the
 *     corpus array changed the printed cousin for 35 lessons, which is exactly
 *     the churn the fixed language order was meant to prevent.
 */
function orderKey(lesson: ParsedLesson): [number, number, string] {
  const chapter = Number(lesson.realization.chapter);
  const sequence = Number(lesson.frontmatter.sequence);
  return [
    Number.isFinite(chapter) ? chapter : Number.MAX_SAFE_INTEGER,
    Number.isFinite(sequence) ? sequence : Number.MAX_SAFE_INTEGER,
    lesson.realization.lessonId,
  ];
}

function comesFirst(a: readonly [number, number, string], b: readonly [number, number, string]): boolean {
  if (a[0] !== b[0]) return a[0] < b[0];
  if (a[1] !== b[1]) return a[1] < b[1];
  return a[2] <= b[2];
}

/**
 * Index the whole corpus by root slug.
 *
 * Built once and reused: the panel is wanted on every lesson that has cousins,
 * and rebuilding this per lesson would be quadratic in a corpus of ~1,900.
 */
export function buildCousinIndex(lessons: readonly ParsedLesson[]): CousinIndex {
  const index = new Map<string, CousinEntry[]>();
  for (const lesson of lessons) {
    const entry: CousinEntry = {
      language: lesson.language,
      lessonId: lesson.realization.lessonId,
      headword: lesson.realization.headword,
      order: orderKey(lesson),
    };
    for (const root of rootSlugs(lesson)) {
      const bucket = index.get(root);
      if (bucket) bucket.push(entry);
      else index.set(root, [entry]);
    }
  }
  return index;
}

/**
 * The cousins to print under one lesson, or `[]` when it has none.
 *
 * Three rules, each of which exists because the alternative prints something
 * false or something noisy:
 *
 *   * **Never the lesson's own language.** A Spanish word is not its own cousin,
 *     and Spanish lessons frequently share a root with each other.
 *   * **One word per language.** A root can be carried by several lessons in a
 *     track; the panel wants the language's word, not a list of the places it
 *     was mentioned. The EARLIEST by reading order wins, so the panel is stable
 *     as later lessons are added.
 *   * **A fixed language order**, not corpus order, so the same lesson renders
 *     identically on every build. Generated output that reorders itself makes
 *     every book hash churn for no reason.
 */
export function cousinsFor(
  index: CousinIndex,
  lesson: ParsedLesson,
  languages: readonly string[] = ROMANCE_COUSINS,
): Cousin[] {
  const own = lesson.language;
  const bestByLanguage = new Map<string, Cousin & { order: [number, number, string] }>();
  for (const root of rootSlugs(lesson)) {
    for (const entry of index.get(root) ?? []) {
      if (entry.language === own) continue;
      if (!languages.includes(entry.language)) continue;
      if (entry.headword.trim() === "") continue;
      const existing = bestByLanguage.get(entry.language);
      if (existing !== undefined && comesFirst(existing.order, entry.order)) continue;
      bestByLanguage.set(entry.language, {
        language: entry.language,
        lessonId: entry.lessonId,
        headword: entry.headword,
        root,
        order: entry.order,
      });
    }
  }
  return languages
    .map((language) => bestByLanguage.get(language))
    .filter((cousin): cousin is Cousin & { order: [number, number, string] } => cousin !== undefined)
    .map(({ order: _order, ...cousin }) => cousin);
}
