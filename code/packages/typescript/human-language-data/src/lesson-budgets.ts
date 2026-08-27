/**
 * HL10 sections 5.5, 7.1 and 7.2 -- the lesson-level content budgets.
 *
 * These three burdens cannot be recovered safely from prose. `LEX` atoms are
 * ordinary words as well as new senses, an unfamiliar phrase is not
 * necessarily an idiom, and a culture paragraph can make zero, one or several
 * claims. Guessing would certify lessons the corpus has not actually measured.
 *
 * Authors therefore declare stable unit ids in optional frontmatter lists:
 *
 *   introduces_idioms: [ES-IDIOM-TENER-QUE]
 *   introduces_senses: [ES-SENSE-QUEDAR-REMAIN]
 *   introduces_culture_claims: [ES-CULTURE-TU-USTED-01]
 *
 * An explicit empty list means "reviewed: zero"; an absent list means "not yet
 * measured". Like the grammar-cell ledger, this starts as an honest annotation burn-down.
 * The report is deliberately report-only while legacy lessons are backfilled.
 */

import { stripControlCharacters as clean } from "./constants.js";
import type { ParsedLesson } from "./parse.js";

export type LessonBudgetKind = "idiom" | "sense" | "culture-claim";

export interface LessonBudgetPolicy {
  idioms: number;
  senses: number;
  cultureClaims: number;
}

export interface LessonBudgetFinding {
  lessonId: string;
  language: string;
  kind: LessonBudgetKind;
  unitId: string;
}

export interface LessonBudgetExcess {
  lessonId: string;
  language: string;
  kind: LessonBudgetKind;
  count: number;
  budget: number;
}

export interface LessonBudgetReport {
  findings: LessonBudgetFinding[];
  excesses: LessonBudgetExcess[];
  policy: LessonBudgetPolicy;
  summary: {
    lessons: number;
    measuredLessons: number;
    idiomMeasuredLessons: number;
    senseMeasuredLessons: number;
    cultureClaimMeasuredLessons: number;
    idioms: number;
    senses: number;
    cultureClaims: number;
    overBudgetLessons: number;
  };
}

const KEYS: Record<LessonBudgetKind, string> = {
  idiom: "introduces_idioms",
  sense: "introduces_senses",
  "culture-claim": "introduces_culture_claims",
};

function lessonId(lesson: ParsedLesson): string {
  const raw = (lesson.frontmatter as Record<string, unknown>).id;
  return typeof raw === "string" ? raw : "<unidentified lesson>";
}

function hasDeclaration(lesson: ParsedLesson, kind: LessonBudgetKind): boolean {
  return Array.isArray((lesson.frontmatter as Record<string, unknown>)[KEYS[kind]]);
}

/** Unique, non-empty ids from one explicit declaration list. */
export function declaredLessonBudgetUnits(
  lesson: ParsedLesson,
  kind: LessonBudgetKind,
): string[] {
  const raw = (lesson.frontmatter as Record<string, unknown>)[KEYS[kind]];
  if (!Array.isArray(raw)) return [];
  return [...new Set(raw.filter((value): value is string => typeof value === "string" && value.trim() !== ""))];
}

/** Measure all explicitly declared new idioms, senses and culture claims. */
export function measureLessonBudgets(
  lessons: ParsedLesson[],
  policy: LessonBudgetPolicy,
): LessonBudgetReport {
  const findings: LessonBudgetFinding[] = [];
  const excesses: LessonBudgetExcess[] = [];
  const measured = new Set<string>();
  const measuredByKind: Record<LessonBudgetKind, number> = {
    idiom: 0,
    sense: 0,
    "culture-claim": 0,
  };
  const overBudget = new Set<string>();

  for (const lesson of lessons) {
    const id = lessonId(lesson);
    const lessonKey = `${lesson.language}\u0000${id}`;
    if ((Object.keys(KEYS) as LessonBudgetKind[]).every((kind) => hasDeclaration(lesson, kind))) {
      measured.add(lessonKey);
    }
    for (const kind of Object.keys(KEYS) as LessonBudgetKind[]) {
      if (hasDeclaration(lesson, kind)) measuredByKind[kind] += 1;
      const units = declaredLessonBudgetUnits(lesson, kind);
      if (units.length === 0) continue;
      for (const unitId of units) {
        findings.push({ lessonId: id, language: lesson.language, kind, unitId });
      }
      const budget =
        kind === "idiom" ? policy.idioms : kind === "sense" ? policy.senses : policy.cultureClaims;
      if (units.length > budget) {
        excesses.push({ lessonId: id, language: lesson.language, kind, count: units.length, budget });
        overBudget.add(lessonKey);
      }
    }
  }

  return {
    findings,
    excesses,
    policy,
    summary: {
      lessons: lessons.length,
      measuredLessons: measured.size,
      idiomMeasuredLessons: measuredByKind.idiom,
      senseMeasuredLessons: measuredByKind.sense,
      cultureClaimMeasuredLessons: measuredByKind["culture-claim"],
      idioms: findings.filter((finding) => finding.kind === "idiom").length,
      senses: findings.filter((finding) => finding.kind === "sense").length,
      cultureClaims: findings.filter((finding) => finding.kind === "culture-claim").length,
      overBudgetLessons: overBudget.size,
    },
  };
}

/** Human-readable lines for the curriculum gap report. */
export function renderLessonBudgets(report: LessonBudgetReport): string[] {
  const s = report.summary;
  const lines = [
    `lesson content budgets (report-only): ${s.measuredLessons} of ${s.lessons} lessons measured -- ` +
      `${s.idioms} idioms, ${s.senses} senses, ${s.cultureClaims} culture claims; ` +
      `${s.overBudgetLessons} lessons over budget`,
    `  annotation coverage: idioms ${s.idiomMeasuredLessons}, senses ${s.senseMeasuredLessons}, ` +
      `culture claims ${s.cultureClaimMeasuredLessons}`,
  ];
  for (const excess of report.excesses.slice(0, 5)) {
    lines.push(
      `  ${clean(excess.lessonId)}: ${excess.count} ${excess.kind}s (budget ${excess.budget})`,
    );
  }
  return lines;
}
