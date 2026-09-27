import { expect } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import type { IntegrationTrackEvidence } from "./helpers.js";

const TWO_ACTIVITY_LESSONS = new Set([
  "JA-C13-family-reception", "JA-C13-family-check", "JA-R14-one-to-five",
  "JA-R15-six-to-ten", "JA-R16-first-five-things", "JA-R17-the-other-ten",
  "JA-C18-count-the-face", "JA-R18-counting-things",
]);

export const integrationTrackEvidence: IntegrationTrackEvidence = {
  id: "japanese",
  assert({ curriculumGapReport: report, lessons }): void {
    const japanese = lessons.filter((lesson) => lesson.language === "japanese");
    // 427 -> 734: chapters 72-130 (295 word lessons and twelve reviews) take
    // Japanese to A1.
    expect(japanese).toHaveLength(734);
    expect(new Set(japanese.map((lesson) => lesson.realization.chapter)))
      .toEqual(new Set(Array.from({ length: 130 }, (_, index) => index + 1)));
    expect(japanese.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
    expect(japanese.map((lesson) => [
      lesson.realization.lessonId,
      compileLessonActivities(lesson.blocks).length,
    ])).toEqual(expect.arrayContaining(
      [...TWO_ACTIVITY_LESSONS].map((lessonId) => [lessonId, 2]),
    ));
    expect(japanese.filter((lesson) => !TWO_ACTIVITY_LESSONS.has(lesson.realization.lessonId))
      .every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
    expect(report.duration.violations.filter((lesson) => lesson.language === "japanese")).toEqual([]);
    expect(report.prerequisites.laterChapterWithoutPrerequisites.filter(
      (lesson) => lesson.language === "japanese",
    )).toEqual([]);

    const headwords = new Map(japanese.map((lesson) => [
      lesson.realization.lessonId, lesson.realization.headword,
    ]));
    expect(headwords.get("JA-C01-konnichiwa")).toBe("こんにちは");
    expect(headwords.get("JA-C01-nihongo")).toBe("日本語");
    expect(headwords.get("JA-C01-koohii")).toBe("コーヒー");
    expect(lessons.find((lesson) => lesson.realization.lessonId === "JA-C01-arigatou")!
      .frontmatter.register).toBe("plain-casual");
    expect(lessons.find((lesson) => lesson.realization.lessonId === "JA-C01-gozaimasu")!
      .frontmatter.register).toBe("teineigo-polite");
  },
};
