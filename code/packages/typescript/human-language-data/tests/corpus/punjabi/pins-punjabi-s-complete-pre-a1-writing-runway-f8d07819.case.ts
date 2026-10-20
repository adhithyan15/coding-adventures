import { expect, it } from "vitest";
import { languageWritingStages } from "../assert-language-corpus.js";

it("pins Punjabi's complete pre-A1 writing runway", () => {
  const punjabi = languageWritingStages("punjabi");
  expect(punjabi.defects).toEqual([]);
  expect(punjabi.levels[0]).toMatchObject({
    level: "pre-A1",
    complete: true,
    missingStages: [],
  });

  // Later chapters may repeat any stage. Pin the stable first owner of each
  // pre-A1 requirement instead of copying the whole ever-growing evidence list.
  const firstEvidenceByStage = new Map<string, string>();
  for (const evidence of punjabi.validEvidence) {
    if (!firstEvidenceByStage.has(evidence.stage)) {
      firstEvidenceByStage.set(evidence.stage, evidence.lessonId);
    }
  }
  expect(Object.fromEntries(firstEvidenceByStage)).toMatchObject({
    "observe-trace": "PA-C01-sat-sri-akal",
    "guided-copy": "PA-W01-haan-guided-copy",
    "delayed-copy": "PA-W01-haan-delayed-copy",
    "dictation-transcription": "PA-W01-haan-dictation",
  });
});
