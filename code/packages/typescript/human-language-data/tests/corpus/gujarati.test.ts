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
    ["4", 5],
    ["5", 5],
    ["6", 10],
    ["7", 6],
    ["8", 5],
    ["9", 5],
    ["10", 2],
    ["11", 6],
    ["12", 4],
    ["13", 4],
    ["14", 4],
    ["15", 5],
    ["16", 4],
    ["17", 6],
    ["18", 6],
    ["19", 6],
    ["20", 5],
    ["21", 1],
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
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
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

it("closes R3 for every Gujarati doorway form after the first name exchange", () => {
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
    (lesson) => lesson.realization.lessonId === "GU-R04-doorway-nine-r2",
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
  ).toEqual([28, 27, 26, 25, 24, 23, 22, 21, 20]);

  const stillMissingR3 = measureContinuity(lessons).reinforcement.filter(
    (defect) => doorway.has(defect.atom) && defect.missed.includes("R3"),
  );
  expect(stillMissingR3).toEqual([]);
});

it("pins Gujarati R4 bridge A at positions 101 through 106", () => {
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
  expect(ordered.slice(101, 107).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(101, 107).every((lesson) =>
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
      return 101 + offset - introducedAt;
    }),
  ).toEqual([71, 71, 71, 71, 71]);

  const stillMissingR4 = measureContinuity(ordered.slice(0, 107)).reinforcement.filter(
    (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
  );
  expect(stillMissingR4).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 101));
  const afterBridge = measureContinuity(ordered.slice(0, 107));
  const priorTrackEnd = 100;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = afterBridge.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(182);
  expect(priorWindowMissesAfterBridge).toBe(171);
});

it("pins Gujarati R4 bridge B at positions 107 through 112", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R16-naam-r4",
    "GU-R16-maarun-r4",
    "GU-R16-chhe-r4",
    "GU-R16-my-name-is-r4",
    "GU-R16-anand-r4",
    "GU-R16-wellbeing-r3",
  ];
  expect(ordered.slice(107, 113).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(107, 113).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C02-NAAM-01",
    "GU-CONCEPT-C02-MAARUN-01",
    "GU-CONCEPT-C02-CHHE-01",
    "GU-CONCEPT-C02-MAARUNNAAMCHHE-01",
    "GU-CONCEPT-C02-ANAND-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 107 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

  const wellbeingR3 = new Set([
    "GU-CONCEPT-C03-HUN-01",
    "GU-CONCEPT-C03-KEM-01",
    "GU-CONCEPT-C03-MAJAA-01",
    "GU-CONCEPT-C03-PRACTICE-01",
    "GU-CONCEPT-C03-TAMEKEMCHHO-01",
    "GU-CONCEPT-C03-VANDHONAHI-01",
  ]);
  const continuity = measureContinuity(ordered.slice(0, 113));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  expect(
    continuity.reinforcement.filter(
      (defect) => wellbeingR3.has(defect.atom) && defect.missed.includes("R3"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 107));
  const priorTrackEnd = 106;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(201);
  expect(priorWindowMissesAfterBridge).toBe(195);
});

it("pins Gujarati R4 bridge C at positions 113 through 118", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R17-you-r4",
    "GU-R17-shun-r4",
    "GU-R17-whats-your-name-r4",
    "GU-R17-introduction-r4",
    "GU-R17-hun-r4",
    "GU-R17-kem-r4",
  ];
  expect(ordered.slice(113, 119).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(113, 119).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C02-TUTAME-01",
    "GU-CONCEPT-C02-SHUN-01",
    "GU-CONCEPT-C02-TAMARUNNAAMSHUNCHHE-01",
    "GU-CONCEPT-C02-PRACTICE-01",
    "GU-CONCEPT-C03-HUN-01",
    "GU-CONCEPT-C03-KEM-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 113 + offset - introducedAt;
    }),
  ).toEqual([62, 62, 62, 61, 61, 61]);

  const continuity = measureContinuity(ordered.slice(0, 119));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 113));
  const priorTrackEnd = 112;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(217);
  expect(priorWindowMissesAfterBridge).toBe(214);
});

it("pins Gujarati R4 bridge D at positions 119 through 123", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R18-how-are-you-r4",
    "GU-R18-majaa-r4",
    "GU-R18-no-problem-r4",
    "GU-R18-wellbeing-r4",
    "GU-R18-farewell-r4",
  ];
  expect(ordered.slice(119, 124).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(119, 124).every((lesson) =>
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
      return 119 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

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
      return 123 - introducedAt;
    }),
  ).toEqual([60, 59, 58, 57]);

  const continuity = measureContinuity(ordered.slice(0, 124));
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

  const beforeBridge = measureContinuity(ordered.slice(0, 119));
  const priorTrackEnd = 118;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(231);
  expect(priorWindowMissesAfterBridge).toBe(225);
});

it("closes Gujarati doorway R4 at position 124", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const doorway = [
    "GU-SCRIPT-JA-01",
    "GU-SCRIPT-O-MATRA-01",
    "GU-SCRIPT-ANUSVARA-01",
    "GU-SCRIPT-II-MATRA-01",
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ];
  const checkpoint = ordered[124]!;
  expect(checkpoint.realization.lessonId).toBe("GU-R19-doorway-nine-r4");
  expect(checkpoint.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    doorway.map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 124 - introducedAt;
    }),
  ).toEqual([98, 97, 96, 95, 94, 93, 92, 91, 90]);

  const beforeCheckpoint = measureContinuity(ordered.slice(0, 124));
  const afterCheckpoint = measureContinuity(lessons);
  expect(beforeCheckpoint.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(234);
  expect(afterCheckpoint.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(226);
  expect(
    afterCheckpoint.reinforcement.filter(
      (defect) => doorway.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
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
    "GU-R04-first-four-r1-dictation",
    "GU-R04-first-four-r1-reading",
    "GU-R05-second-four-r1-dictation",
    "GU-R05-second-four-r1-doorway-three-r2-dictation",
    "GU-R05-second-four-r1-doorway-three-r2-reading",
    "GU-R05-second-four-r1-reading",
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
    "GU-R16-anand-r4-listening",
    "GU-R16-anand-r4-writing",
    "GU-R16-chhe-r4-listening",
    "GU-R16-chhe-r4-writing",
    "GU-R16-maarun-r4-listening",
    "GU-R16-maarun-r4-writing",
    "GU-R16-my-name-is-r4-listening",
    "GU-R16-my-name-is-r4-writing",
    "GU-R16-naam-r4-listening",
    "GU-R16-naam-r4-writing",
    "GU-R16-wellbeing-r3-listening",
    "GU-R16-wellbeing-r3-writing",
    "GU-R17-hun-r4-listening",
    "GU-R17-hun-r4-writing",
    "GU-R17-introduction-r4-listening",
    "GU-R17-introduction-r4-writing",
    "GU-R17-kem-r4-listening",
    "GU-R17-kem-r4-writing",
    "GU-R17-shun-r4-listening",
    "GU-R17-shun-r4-writing",
    "GU-R17-whats-your-name-r4-listening",
    "GU-R17-whats-your-name-r4-writing",
    "GU-R17-you-r4-listening",
    "GU-R17-you-r4-writing",
    "GU-R18-farewell-r4-listening",
    "GU-R18-farewell-r4-writing",
    "GU-R18-how-are-you-r4-listening",
    "GU-R18-how-are-you-r4-writing",
    "GU-R18-majaa-r4-listening",
    "GU-R18-majaa-r4-writing",
    "GU-R18-no-problem-r4-listening",
    "GU-R18-no-problem-r4-writing",
    "GU-R18-wellbeing-r4-listening",
    "GU-R18-wellbeing-r4-writing",
    "GU-R19-doorway-nine-r4-dictation",
    "GU-R19-doorway-nine-r4-reading",
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
    "GU-W04-i-matra-recall",
    "GU-W04-independent-a-recall",
    "GU-W04-lla-recall",
    "GU-W04-tha-recall",
    "GU-W05-ba-recall",
    "GU-W05-cha-recall",
    "GU-W05-la-recall",
    "GU-W05-pa-recall",
  ]);
});
