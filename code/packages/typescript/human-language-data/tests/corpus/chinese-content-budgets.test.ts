import { expect, it } from "vitest";
import { defaultCurriculumRoot, loadChapterPolicy, loadTrackLessons } from "../../src/loader.js";
import { measureLessonBudgets } from "../../src/lesson-budgets.js";

const REVIEWED = [
  "ZH-C01-tones", "ZH-C01-ni", "ZH-C01-hao", "ZH-C01-nihao",
  "ZH-C01-tone-sandhi", "ZH-C01-hao-fond", "ZH-C01-practice",
  "ZH-C03-yi", "ZH-C03-er", "ZH-C03-san", "ZH-C03-si",
  "ZH-C03-wu", "ZH-C03-practice",
];

it("ratchets the reviewed Chinese Chapter 1 and 3 content budgets", () => {
  const lessons = loadTrackLessons("chinese", defaultCurriculumRoot());
  const byId = new Map(lessons.map((lesson) => [String(lesson.frontmatter.id), lesson]));
  const policy = loadChapterPolicy();
  const report = measureLessonBudgets(lessons, {
    idioms: policy.maxNewIdiomsPerLesson ?? 1,
    senses: policy.maxNewSensesPerLesson ?? 1,
    cultureClaims: policy.maxNewCultureClaimsPerLesson ?? 2,
  });

  // This is a lower bound, not a false claim that the other 178 legacy lessons
  // or the Japanese track have already been reviewed.
  expect(report.summary.measuredLessons).toBeGreaterThanOrEqual(3056);
  expect(report.summary.idiomMeasuredLessons).toBeGreaterThanOrEqual(3056);
  expect(report.summary.senseMeasuredLessons).toBeGreaterThanOrEqual(3056);
  expect(report.summary.cultureClaimMeasuredLessons).toBeGreaterThanOrEqual(3056);
  expect(report.excesses).toEqual([]);

  for (const id of REVIEWED) {
    const lesson = byId.get(id);
    expect(lesson, `${id} stays in the canonical Chinese corpus`).toBeDefined();
    expect(lesson!.frontmatter.schema_version).toBe("2");
    for (const key of ["introduces_idioms", "introduces_senses", "introduces_culture_claims"]) {
      expect(lesson!.frontmatter[key], `${id} declares ${key}`).toBeInstanceOf(Array);
    }
    expect(Number(lesson!.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
  }
  expect(report.findings.filter(({ lessonId }) => REVIEWED.includes(lessonId))).toEqual([
    {
      language: "chinese",
      lessonId: "ZH-C03-si",
      kind: "culture-claim",
      unitId: "ZH-CULTURE-SI-FLOOR-NUMBER-01",
    },
  ]);
  expect(report.findings.every(({ unitId }) => unitId.startsWith("ZH-"))).toBe(true);
});
