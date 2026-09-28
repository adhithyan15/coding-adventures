import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Gujarati R4 bridge D at positions 144 through 148", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  // HL-C443 moved every position here by 13: the opening runways now open
  // with thirteen anchor words (chapter 1 gains one, chapters 3-7 twelve), each
  // read before the letters it holds. Distances from atoms introduced after
  // chapter 7 are unchanged; distances from chapter 3-7 script atoms grew with
  // the words inserted between them and the bridge, and every window asserted
  // below still closes.
  // Gujarati A1: every position here +2. Chapter 12's two over-budget number
  // lessons split into -more continuations at positions 96 and 98, ahead of
  // this bridge. Atoms introduced before them are two lessons further away;
  // every window asserted below still closes.
  const bridgeIds = [
    "GU-R18-how-are-you-r4",
    "GU-R18-majaa-r4",
    "GU-R18-no-problem-r4",
    "GU-R18-wellbeing-r4",
    "GU-R18-farewell-r4",
  ];
  expect(ordered.slice(144, 149).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(144, 149).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C03-TAMEKEMCHHO-01",
    "GU-CONCEPT-C03-MAJAA-01",
    "GU-CONCEPT-C03-VANDHONAHI-01",
    "GU-CONCEPT-C03-PRACTICE-01",
    "GU-CONCEPT-C04-MALISHUN-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 144 + offset - introducedAt;
    }),
  ).toEqual([63, 63, 63, 63, 63]);

  const farewellR3 = [
    "GU-CONCEPT-C04-KAALE-01",
    "GU-CONCEPT-C04-PACHHA-01",
    "GU-CONCEPT-C04-PACHHAMALISHUN-01",
    "GU-CONCEPT-C04-PRACTICE-01",
  ];
  expect(
    farewellR3.map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 148 - introducedAt;
    }),
  ).toEqual([62, 61, 60, 59]);
  // Gujarati A1: the chapter-12 continuations put kāle and pāchhā at 62 and
  // 61 from this bridge, past R3's 60. GU-C12-naak's warm-up now retrieves
  // both at 39 and 38, so R3 still closes for all four farewell atoms below.

  const continuity = measureContinuity(ordered.slice(0, 149));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  expect(
    continuity.reinforcement.filter(
      (defect) => farewellR3.includes(defect.atom) && defect.missed.includes("R3"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 144));
  const priorTrackEnd = 143;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(266); // Gujarati A1: 268 -> 266. The splits' R1/R3 gains and the new retrievals outweigh windows the +2 prefix newly measures.
  expect(priorWindowMissesAfterBridge).toBe(262);
});
