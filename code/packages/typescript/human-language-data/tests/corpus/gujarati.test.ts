import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
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
    ["3", 9],
    ["4", 6],
    ["5", 5],
    ["6", 5],
    ["7", 2],
    ["8", 6],
    ["9", 4],
    ["10", 4],
    ["11", 4],
    ["12", 4],
    ["13", 4],
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
  ]);
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
  ]);
});
