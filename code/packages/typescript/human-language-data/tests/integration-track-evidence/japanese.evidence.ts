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
    // 734 -> 743: chapter 131 writes small ゃ, small ょ and を, each one lesson
    // after the word that holds it, with two more words and one review.
    // 743 -> 752: chapter 132 writes そ, れ and る the same way, each one lesson
    // after the word that holds it (そこ, これ, くるま), with two more words
    // (そと, それ) and one review.
    // 752 -> 764: chapter 133 writes the last four basic hiragana, き, け, ぬ
    // and へ, the same way (えき, いけ, いぬ, へや), with three more words
    // (きのう, けさ, ぬの) and one review.
    // 764 -> 788: chapters 134 and 135 write eight voiced kana, で, ば, べ, ぶ
    // and び, ぐ, げ, ぎ, the same way: each one lesson after the word that
    // holds it (でんわ, かばん, たべる, しんぶん, びょういん, いりぐち,
    // げつようび, ぎんこう), with six more words (でんしゃ, ばしょ, べんきょう,
    // どようび, でぐち, げんき) and one review per chapter. A2 words such as
    // でんわ and べんきょう needed these signs.
    expect(japanese).toHaveLength(788);
    expect(new Set(japanese.map((lesson) => lesson.realization.chapter)))
      .toEqual(new Set(Array.from({ length: 135 }, (_, index) => index + 1)));
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
