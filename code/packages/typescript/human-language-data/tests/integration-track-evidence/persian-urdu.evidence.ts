import { expect } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { buildCurriculumGapReport } from "../../src/report.js";
import type { IntegrationTrackEvidence } from "./helpers.js";

const EXPECTED_COUNTS = {
  persian: { 3: 6, 4: 6, 5: 5 },
  urdu: { 3: 8, 4: 10, 5: 7 },
} as const;

export const integrationTrackEvidence: IntegrationTrackEvidence = {
  id: "persian-urdu",
  assert({ registry, lessons, books }): void {
    const report = buildCurriculumGapReport({ registry, lessons, books });
    for (const language of ["persian", "urdu"] as const) {
      for (const chapterNumber of [3, 4, 5] as const) {
        const chapter = lessons.filter(
          (lesson) => lesson.language === language && lesson.realization.chapter === chapterNumber,
        );
        expect(chapter).toHaveLength(EXPECTED_COUNTS[language][chapterNumber]);
        expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
        expect(chapter.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
        expect(report.duration.violations.filter(
          (lesson) => lesson.language === language && lesson.chapter === chapterNumber,
        )).toEqual([]);
        expect(report.prerequisites.laterChapterWithoutPrerequisites.filter(
          (lesson) => lesson.language === language && lesson.chapter === chapterNumber,
        )).toEqual([]);
      }
    }
    expect(lessons.find((lesson) => lesson.realization.lessonId === "FA-C05-khodahafez")!
      .frontmatter.headword).toBe("خداحافظ");
    expect(lessons.find((lesson) => lesson.realization.lessonId === "UR-C05-khuda-hafiz")!
      .frontmatter.headword).toBe("خدا حافظ");
  },
};
