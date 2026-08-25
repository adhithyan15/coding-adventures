import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../src/continuity.js";
import { loadTrackLessons } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Gujarati continuity", () => expectLanguageContinuity("gujarati"));
it("pins Gujarati modality", () => expectLanguageModality("gujarati"));

it("pins Gujarati's meaning-first opening script spine", () => {
  const ordered = loadTrackLessons("gujarati").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const opening = ordered.slice(0, 11);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "GU-C01-namaste",
    "GU-W01-ha",
    "GU-W01-aa-matra",
    "GU-W01-aa",
    "GU-W01-na",
    "GU-W01-ma",
    "GU-W01-sa",
    "GU-W01-ta",
    "GU-W01-e-matra",
    "GU-W01-virama",
    "GU-W01-namaste-read",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const meaningFirst = opening[0]!;
  expect(meaningFirst.realization.romanization).toBe("namaste");
  expect(meaningFirst.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(meaningFirst.body).not.toMatch(/\p{Script=Gujarati}/u);

  const courtesy = ordered.slice(11, 26);
  expect(courtesy.map((lesson) => lesson.realization.lessonId)).toEqual([
    "GU-W02-ra",
    "GU-W02-da",
    "GU-W02-va",
    "GU-W02-ya",
    "GU-W02-bha",
    "GU-W02-dha",
    "GU-W02-vocalic-r",
    "GU-C01-aabhaar",
    "GU-C01-aavjo",
    "GU-C01-haa-naa",
    "GU-W01-haa-guided-copy",
    "GU-W01-haa-delayed-copy",
    "GU-W01-haa-dictation",
    "GU-C01-saarun",
    "GU-C01-practice",
  ]);
  expect(courtesy.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 11],
    ["2", 15],
    ["3", 10],
    ["4", 10],
    ["5", 6],
    ["6", 5],
    ["7", 5],
    ["8", 2],
    ["9", 6],
    ["10", 4],
    ["11", 4],
    ["12", 4],
    ["13", 5],
    ["14", 4],
    ["15", 6],
  ]);
});

it("pins Gujarati's complete pre-A1 writing runway", () => {
  const gujarati = languageWritingStages("gujarati");
  expect(gujarati.defects).toEqual([]);
  expect(gujarati.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(gujarati.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "delayed-copy",
    "delayed-copy",
    "delayed-copy",
    "dictation-transcription",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
  ]);
});

it("closes R3 for every Gujarati doorway form with one zero-atom checkpoint", () => {
  const doorway = new Set([
    "GU-SCRIPT-JA-01",
    "GU-SCRIPT-O-MATRA-01",
    "GU-SCRIPT-ANUSVARA-01",
    "GU-SCRIPT-II-MATRA-01",
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ]);
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "GU-R13-doorway-nine-r3",
  );
  const checkpoint = ordered[checkpointIndex]!;
  expect(checkpoint.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    [...doorway].map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return checkpointIndex - introducedAt;
    }),
  ).toEqual([60, 59, 58, 57, 56, 55, 54, 53, 52]);

  const stillMissingR3 = measureContinuity(lessons).reinforcement.filter(
    (defect) => doorway.has(defect.atom) && defect.missed.includes("R3"),
  );
  expect(stillMissingR3).toEqual([]);
});

it("pins Gujarati R4 bridge A at positions 91 through 96", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R15-u-matra-r4",
    "GU-R15-chha-r4",
    "GU-R15-ka-r4",
    "GU-R15-nna-r4",
    "GU-R15-sha-r4",
    "GU-R15-name-exchange-r3",
  ];
  expect(ordered.slice(91, 97).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(91, 97).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 91 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

  const stillMissingR4 = measureContinuity(lessons).reinforcement.filter(
    (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
  );
  expect(stillMissingR4).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 91));
  const afterBridge = measureContinuity(ordered);
  const priorTrackEnd = 90;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = afterBridge.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(157);
  expect(priorWindowMissesAfterBridge).toBe(146);
});

it("pins Gujarati-owned objective activities", () => {
  const ids = loadTrackLessons("gujarati")
    .flatMap((lesson) => compileLessonActivities(lesson.blocks))
    .map((activity) => activity.id)
    .sort();
  expect(ids).toEqual([
    "GU-C01-aabhaar-script-retrieval",
    "GU-C01-aavjo-script-retrieval",
    "GU-C01-haa-naa-script-retrieval",
    "GU-C01-practice-four-consonants",
    "GU-C01-practice-two-consonants-one-sign",
    "GU-C01-practice-writing-payoff",
    "GU-C02-shun-da-retrieval",
    "GU-C02-tamarun-naam-shun-chhe-va-retrieval",
    "GU-C02-tu-tame-ra-retrieval",
    "GU-C03-hun-ya-retrieval",
    "GU-C03-kem-bha-retrieval",
    "GU-C03-majaa-vocalic-r-retrieval",
    "GU-C03-tame-kem-chho-dha-retrieval",
    "GU-C06-number-histories-be-source",
    "GU-R03-doorway-three-r1-dictation",
    "GU-R03-doorway-three-r1-reading",
    "GU-R04-doorway-nine-r2-dictation",
    "GU-R04-doorway-nine-r2-reading",
    "GU-R13-doorway-nine-r3-dictation",
    "GU-R13-doorway-nine-r3-reading",
    "GU-R15-chha-r4-dictation",
    "GU-R15-chha-r4-reading",
    "GU-R15-ka-r4-dictation",
    "GU-R15-ka-r4-reading",
    "GU-R15-name-exchange-r3-production",
    "GU-R15-name-exchange-r3-reading",
    "GU-R15-nna-r4-dictation",
    "GU-R15-nna-r4-reading",
    "GU-R15-sha-r4-dictation",
    "GU-R15-sha-r4-reading",
    "GU-R15-u-matra-r4-dictation",
    "GU-R15-u-matra-r4-reading",
    "GU-W01-aa-matra-observe-check",
    "GU-W01-ha-observe-check",
    "GU-W01-haa-delayed-copy-check",
    "GU-W01-haa-delayed-copy-vocalic-r-retrieval",
    "GU-W01-haa-dictation-answer",
    "GU-W01-haa-guided-copy-check",
    "GU-W01-haa-guided-copy-dha-retrieval",
    "GU-W02-bha-recall",
    "GU-W02-da-recall",
    "GU-W02-dha-recall",
    "GU-W02-ra-recall",
    "GU-W02-va-recall",
    "GU-W02-vocalic-r-recall",
    "GU-W02-ya-recall",
    "GU-W03-anusvara-recall",
    "GU-W03-chha-recall",
    "GU-W03-ii-matra-recall",
    "GU-W03-ja-recall",
    "GU-W03-ka-recall",
    "GU-W03-nna-recall",
    "GU-W03-o-matra-recall",
    "GU-W03-sha-doorway-dictation",
    "GU-W03-sha-doorway-reading",
    "GU-W03-sha-recall",
    "GU-W03-u-matra-recall",
  ]);
});
