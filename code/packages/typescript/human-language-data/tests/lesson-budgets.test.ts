import { describe, expect, it } from "vitest";
import { loadChapterPolicy, loadEverything } from "../src/loader.js";
import {
  declaredLessonBudgetUnits,
  measureLessonBudgets,
  renderLessonBudgets,
  type LessonBudgetPolicy,
} from "../src/lesson-budgets.js";
import { parseLesson } from "../src/parse.js";

const POLICY: LessonBudgetPolicy = { idioms: 1, senses: 1, cultureClaims: 2 };

function lesson(
  id: string,
  declarations: { idioms?: string[]; senses?: string[]; cultureClaims?: string[] },
) {
  const lines = ["schema_version: 2", `id: ${id}`, "sequence: 10"];
  if (declarations.idioms) lines.push(`introduces_idioms: [${declarations.idioms.join(", ")}]`);
  if (declarations.senses) lines.push(`introduces_senses: [${declarations.senses.join(", ")}]`);
  if (declarations.cultureClaims) {
    lines.push(`introduces_culture_claims: [${declarations.cultureClaims.join(", ")}]`);
  }
  return parseLesson(
    `---\n${lines.join("\n")}\n---\n\n# Test\n\nSay it.\n`,
    "spanish",
    "Latin",
  );
}

describe("lesson content budgets", () => {
  it("counts explicit stable ids and deduplicates repeated ids", () => {
    const parsed = lesson("ES-BUDGET", {
      idioms: ["ES-IDIOM-A", "ES-IDIOM-A"],
      senses: ["ES-SENSE-A"],
      cultureClaims: ["ES-CULTURE-A", "ES-CULTURE-B"],
    });
    expect(declaredLessonBudgetUnits(parsed, "idiom")).toEqual(["ES-IDIOM-A"]);
    expect(measureLessonBudgets([parsed], POLICY).summary).toEqual({
      lessons: 1,
      measuredLessons: 1,
      idiomMeasuredLessons: 1,
      senseMeasuredLessons: 1,
      cultureClaimMeasuredLessons: 1,
      idioms: 1,
      senses: 1,
      cultureClaims: 2,
      overBudgetLessons: 0,
    });
  });

  it("reports each policy excess without failing the corpus", () => {
    const report = measureLessonBudgets(
      [
        lesson("ES-OVER", {
          idioms: ["I-1", "I-2"],
          senses: ["S-1", "S-2"],
          cultureClaims: ["C-1", "C-2", "C-3"],
        }),
      ],
      POLICY,
    );
    expect(report.excesses.map((item) => [item.kind, item.count, item.budget])).toEqual([
      ["idiom", 2, 1],
      ["sense", 2, 1],
      ["culture-claim", 3, 2],
    ]);
    expect(report.summary.overBudgetLessons).toBe(1);
    expect(renderLessonBudgets(report).join("\n")).toContain("ES-OVER");
  });

  it("does not infer a clean bill from unannotated prose", () => {
    const parsed = parseLesson(
      "---\nschema_version: 2\nid: ES-LEGACY\nsequence: 10\n---\n\n# Idioms and senses\n\nA culture claim.\n",
      "spanish",
      "Latin",
    );
    const report = measureLessonBudgets([parsed], POLICY);
    expect(report.summary.measuredLessons).toBe(0);
    expect(report.findings).toEqual([]);
  });

  it("counts explicit empty lists as reviewed zeroes", () => {
    const report = measureLessonBudgets(
      [lesson("ES-REVIEWED-ZERO", { idioms: [], senses: [], cultureClaims: [] })],
      POLICY,
    );
    expect(report.summary.measuredLessons).toBe(1);
    expect(report.summary.idioms).toBe(0);
    expect(report.summary.senses).toBe(0);
    expect(report.summary.cultureClaims).toBe(0);
  });
});

describe("the committed corpus", () => {
  const { lessons } = loadEverything();
  const policy = loadChapterPolicy();
  const budgets = {
    idioms: policy.maxNewIdiomsPerLesson ?? 1,
    senses: policy.maxNewSensesPerLesson ?? 1,
    cultureClaims: policy.maxNewCultureClaimsPerLesson ?? 2,
  };

  it("pins the policy values and the honest first measurement", () => {
    expect(budgets).toEqual(POLICY);
    const report = measureLessonBudgets(lessons, budgets);
    expect(report.summary.lessons).toBeGreaterThanOrEqual(2018);
    expect(report.summary.measuredLessons).toBe(0);
    expect(report.summary.idiomMeasuredLessons).toBe(0);
    expect(report.summary.senseMeasuredLessons).toBe(0);
    expect(report.summary.cultureClaimMeasuredLessons).toBe(0);
    expect(report.summary.idioms).toBe(0);
    expect(report.summary.senses).toBe(0);
    expect(report.summary.cultureClaims).toBe(0);
    expect(report.summary.overBudgetLessons).toBe(0);
  });
});
