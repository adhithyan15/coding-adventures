// ---------------------------------------------------------------------------
// exam-inventory.ts — coverage against what the EXAM tests, not against what
// the corpus happens to contain.
//
// WHY THIS MODULE EXISTS
// `levels.ts` reports which CEFR node a lesson points at, and `level-gate.ts`
// reports whether a track has met criteria this repository invented for itself
// (a headword count, a revisit count). Both are corpus-internal. The owner's
// question is a different one:
//
//     "Can someone pass that level of exam with just reading the book and
//      slowly following its gentle ramp?"
//
// No number produced by walking our own lessons can answer that, because every
// such number rises when you add a lesson — including a lesson on something the
// exam does not test. The answer has to be measured against an EXTERNAL,
// FINITE list of what a candidate is expected to hold.
//
// The Plan Curricular del Instituto Cervantes publishes exactly that: a grammar
// inventory in 15 categories, with separate A1 and A2 columns. `core/
// exam-inventory-es-a1.json` restates the A1 column in our own words and, for
// each point, names the atoms whose presence would demonstrate it.
//
// WHY THE MAPPING IS A PROBE AND NOT AN ANNOTATION
// The obvious design is a `coveredBy:` field a human fills in once. That field
// is a claim about the corpus made at a moment in time, and it goes stale the
// first time a lesson is renamed or an atom is retired — silently, and in the
// flattering direction, because nothing recomputes it.
//
// A probe is executable. `probe: ["ES-GRAMMAR-NOUN-GENDER"]` means "this point
// is covered exactly when that atom is introduced by some lesson", and the
// answer is recomputed on every run. Retire the atom and coverage falls. That
// is the property the gate needs and the annotation cannot have.
//
// `probe: null` is not "unknown" and not "skip". It means no atom in the corpus
// corresponds to this point, which is a finding rather than a gap in the data —
// so a null probe counts as UNCOVERED and is reported by name.
// ---------------------------------------------------------------------------
import type { ParsedLesson } from "./parse.js";
import { introducedAtoms as atomsIntroducedBy } from "./ramp.js";

/** Content dimensions a level inventory must enumerate before it is complete. */
export const EXAM_CONTENT_DIMENSIONS = [
  "communicative-functions",
  "grammar",
  "phonology-orthography",
  "lexicon",
] as const;
export type ExamContentDimension = (typeof EXAM_CONTENT_DIMENSIONS)[number];

export interface ExamInventoryScopeEntry {
  /** Partial evidence remains useful, but may never suppress the backlog item. */
  status: "complete" | "partial";
  /** Exact awarding-body or checked-in project source for this dimension. */
  source: string;
  /** What this source does and does not enumerate, in the project's own words. */
  note: string;
}

/** One thing an examiner may expect a candidate at this level to hold. */
export interface ExamPoint {
  id: string;
  category: string;
  label: string;
  /**
   * Atoms whose presence demonstrates the point, or `null` when nothing in the
   * corpus corresponds to it. ALL listed atoms must be introduced — a point
   * covered by half its atoms is not covered, because a candidate asked for the
   * plural of a paradigm does not get partial credit for the singular.
   */
  probe: string[] | null;
  /** Why a point is unmapped, when the answer is more interesting than "no". */
  note?: string;
}

export interface ExamInventory {
  version: number;
  language: string;
  level: string;
  about: string;
  source: string;
  /** Completeness is derived: every required dimension must say `complete`. */
  scope: Record<ExamContentDimension, ExamInventoryScopeEntry>;
  probeSemantics: string;
  points: ExamPoint[];
}

/** A single partial dimension keeps the whole inventory honestly partial. */
export function isExamInventoryComplete(inventory: ExamInventory): boolean {
  return EXAM_CONTENT_DIMENSIONS.every((dimension) => inventory.scope[dimension].status === "complete");
}

/** What one point resolved to against a given corpus. */
export interface PointCoverage {
  id: string;
  category: string;
  label: string;
  covered: boolean;
  /** Atoms the probe asked for that the corpus does not introduce. */
  missingAtoms: string[];
  /** True when the point has no probe at all — nothing in the corpus fits it. */
  unmapped: boolean;
  note?: string;
}

export interface ExamCoverage {
  language: string;
  level: string;
  /** Point coverage can be measured even while the source inventory is partial. */
  inventoryComplete: boolean;
  /** Points enumerated by the inventory. */
  enumerated: number;
  /** Points every one of whose atoms is introduced somewhere in the track. */
  covered: number;
  /** Points with no corresponding atom at all. */
  unmapped: number;
  /** Points with a probe whose atoms are only partly present. */
  partial: number;
  /** covered / enumerated, rounded to a whole percent. */
  percent: number;
  /** Per-category totals, so a gap can be named rather than merely counted. */
  byCategory: Record<string, { enumerated: number; covered: number }>;
  points: PointCoverage[];
}

/**
 * The atoms a track introduces, which is the only thing a probe consults.
 *
 * `introduces` rather than `practises`: a lesson that drills an atom it did not
 * introduce is revision, and revision of something never taught is not what a
 * candidate needs. Restricting to `introduces` also makes the count stable
 * under the review/synthesis rungs, which practise widely by design.
 *
 * The per-lesson read is delegated to `ramp.ts` rather than repeated here. Its
 * comment explains why: the frontmatter key is FLAT and dotted
 * (`introduces.knowledge`), and reading it as a nested object returns undefined
 * for every lesson in the corpus — a mistake that once reported all 279
 * authored chapters as broken. A local copy of that read is a local copy of
 * that bug waiting to be reintroduced.
 */
export function trackIntroducedAtoms(
  lessons: readonly ParsedLesson[],
  language: string,
): Set<string> {
  const atoms = new Set<string>();
  for (const lesson of lessons) {
    if (lesson.language !== language) continue;
    for (const atom of atomsIntroducedBy(lesson)) atoms.add(atom);
  }
  return atoms;
}

/**
 * Resolve an inventory against a corpus.
 *
 * Deliberately total: every enumerated point appears in `points`, covered or
 * not. A report that listed only the failures would make the denominator
 * invisible, and the denominator is the whole argument — "48 of 79" is a
 * position, "31 gaps" is a mood.
 */
export function measureExamCoverage(
  inventory: ExamInventory,
  lessons: readonly ParsedLesson[],
): ExamCoverage {
  const taught = trackIntroducedAtoms(lessons, inventory.language);
  const points: PointCoverage[] = inventory.points.map((point) => {
    const missingAtoms = (point.probe ?? []).filter((atom) => !taught.has(atom));
    return {
      id: point.id,
      category: point.category,
      label: point.label,
      covered: point.probe !== null && missingAtoms.length === 0,
      missingAtoms,
      unmapped: point.probe === null,
      ...(point.note === undefined ? {} : { note: point.note }),
    };
  });

  // Accumulated in a Map, not an object literal. `byCategory[point.category]
  // ??= ...` on a literal is a prototype-pollution sink: with
  // `category: "__proto__"` the lookup returns `Object.prototype`, which is
  // truthy, so `??=` never assigns and `bucket.enumerated += 1` writes onto
  // `Object.prototype` — after which every object built in the process
  // inherits `enumerated: NaN`. It also corrupted this very report, because
  // the polluting category vanished from the own keys while still counting
  // toward the totals, so the per-category lines and the summary disagreed.
  //
  // `Object.create(null)` also closes that, and was the first fix here, but it
  // leaks an awkwardness into a PUBLIC return type: a consumer calling
  // `coverage.byCategory.hasOwnProperty(x)` or interpolating the object into a
  // string gets a TypeError, because there is no prototype to supply either.
  // A Map finished with `Object.fromEntries` is safe for the same reason —
  // `fromEntries` uses CreateDataProperty, so `__proto__` lands as an ordinary
  // own property and no setter fires — while handing back a normal object.
  const tally = new Map<string, { enumerated: number; covered: number }>();
  for (const point of points) {
    let bucket = tally.get(point.category);
    if (bucket === undefined) tally.set(point.category, (bucket = { enumerated: 0, covered: 0 }));
    bucket.enumerated += 1;
    if (point.covered) bucket.covered += 1;
  }
  const byCategory: Record<string, { enumerated: number; covered: number }> =
    Object.fromEntries(tally);

  const covered = points.filter((point) => point.covered).length;
  return {
    language: inventory.language,
    level: inventory.level,
    inventoryComplete: isExamInventoryComplete(inventory),
    enumerated: points.length,
    covered,
    unmapped: points.filter((point) => point.unmapped).length,
    partial: points.filter((point) => !point.unmapped && !point.covered).length,
    // Guarded because `0/0` is `NaN`, and `NaN >= 60` is false — fail-safe
    // against a threshold, but a percentage that renders as "NaN%" in a report
    // is a bug wearing the costume of a finding.
    percent: points.length === 0 ? 0 : Math.round((covered / points.length) * 100),
    byCategory,
    points,
  };
}

/**
 * A one-screen report, ordered worst category first.
 *
 * The ordering is the point: a list sorted by id tells you what exists, and a
 * list sorted by shortfall tells you what to do next.
 */
export function formatExamCoverage(coverage: ExamCoverage): string {
  const lines: string[] = [
    `${coverage.language} ${coverage.level}${coverage.inventoryComplete ? "" : " (partial inventory)"}: ` +
      `${coverage.covered}/${coverage.enumerated} points covered (${coverage.percent}%)`,
    `  ${coverage.unmapped} with no corresponding atom, ${coverage.partial} only partly taught`,
  ];
  const categories = Object.entries(coverage.byCategory).sort(
    (a, b) => a[1].covered / a[1].enumerated - b[1].covered / b[1].enumerated || a[0].localeCompare(b[0]),
  );
  for (const [category, totals] of categories) {
    lines.push(`  ${totals.covered}/${totals.enumerated}  ${category}`);
  }
  return lines.join("\n");
}
