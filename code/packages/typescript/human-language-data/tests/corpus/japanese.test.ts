import { expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  defaultCurriculumRoot,
  loadAssessmentContracts,
  loadTaskShapeInventory,
} from "../../src/loader.js";
import {
  parseMockManifest,
  validateMockAnswerKey,
  validateMockPaper,
  validateMockRubric,
} from "../../src/mock-manifest.js";
import { readLedgerFile } from "../../src/shard.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Japanese continuity", () => expectLanguageContinuity("japanese"));
it("pins Japanese modality", () => expectLanguageModality("japanese"));

it("pins Japanese's writing ladder IN ORDER, pre-A1 through A1", () => {
  const japanese = languageWritingStages("japanese");
  expect(japanese.defects).toEqual([]);
  expect(japanese.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(japanese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    // The three A1 stages, proved by the appended writing chapters. ORDER is
    // the point: a timed or connected paper before a controlled one is invalid.
    "controlled-composition",
    "timed-assessment-production",
    "connected-composition",
  ]);
});

it("adds Japanese pre-A1 full mock 1 with separate secure four-skill packets", () => {
  const root = defaultCurriculumRoot();
  const trackRoot = join(root, "japanese");
  const level = loadAssessmentContracts(root)
    .find((entry) => entry.language === "japanese")?.contract.levels
    .find((entry) => entry.level === "pre-A1");
  const mock = level?.fullMocks.find((entry) => entry.id === "pre-a1-mock-1");
  expect(mock).toMatchObject({
    timed: true,
    paper: "mocks/pre-a1/mock-1.md",
    rubric: "mocks/pre-a1/rubric.md",
    answerKey: "mocks/pre-a1/mock-1-answer-key.md",
    manifest: "mocks/pre-a1/mock-1-manifest.json",
  });
  const shape = loadTaskShapeInventory("japanese", "pre-A1", root);
  const manifest = parseMockManifest(
    readLedgerFile(join(trackRoot, mock!.manifest!)),
    "japanese", "pre-A1", "pre-a1-mock-1", shape,
  );
  expect(manifest.sections).toEqual([
    { skill: "reading", minutes: 5, partIds: ["prea1-reading-kana-and-doorway-words"] },
    { skill: "listening", minutes: 5, partIds: ["prea1-listening-greetings-and-one-turn-details"] },
    { skill: "writing", minutes: 10, partIds: [
      "prea1-writing-delayed-kana-recall", "prea1-writing-dictation", "prea1-writing-bounded-production",
    ] },
    { skill: "speaking", minutes: 5, partIds: ["prea1-speaking-doorway-exchange"] },
  ]);

  const paper = readFileSync(join(trackRoot, mock!.paper!), "utf8");
  const rubric = readFileSync(join(trackRoot, mock!.rubric), "utf8");
  const key = readFileSync(join(trackRoot, mock!.answerKey), "utf8");
  const examiner = readFileSync(join(trackRoot, "mocks/pre-a1/mock-1-examiner-script.md"), "utf8");
  validateMockPaper(manifest, paper);
  validateMockRubric(manifest, rubric);
  validateMockAnswerKey(manifest, key);

  const part = (source: string, id: string) => source.split(`<!-- mock-part:${id} -->`)[1]?.split(`<!-- /mock-part:${id} -->`)[0] ?? "";
  const count = (id: string) => part(paper, id).match(/^\d+\./gm)?.length ?? 0;
  expect(count("prea1-reading-kana-and-doorway-words")).toBe(5);
  expect(count("prea1-listening-greetings-and-one-turn-details")).toBe(5);
  expect(count("prea1-writing-delayed-kana-recall")).toBe(1);
  expect(count("prea1-writing-dictation")).toBe(2);
  expect(count("prea1-writing-bounded-production")).toBe(2);
  expect(count("prea1-speaking-doorway-exchange")).toBe(5);
  expect(part(key, "prea1-reading-kana-and-doorway-words")).toMatch(/1 B; 2 A; 3 C; 4 A; 5 B/);
  expect(part(key, "prea1-listening-greetings-and-one-turn-details")).toMatch(/1 A; 2 B; 3 B; 4 C; 5 A/);
  for (const [file, utterance, keyedChoice] of [
    ["l01", "こんにちは", "A"], ["l02", "はい", "B"],
    ["l03", "いいえ", "B"], ["l04", "なまえ", "C"],
    ["l05", "おねがいします", "A"],
  ]) {
    const line = examiner.split("\n").find((entry) => entry.includes(`mock-1-${file}`));
    expect(line).toContain(`**${utterance}**`);
    expect(line).toContain(`key ${keyedChoice}`);
  }
  expect(paper).toMatch(/20 written minutes.*5 speaking minutes/is);
  expect(paper).toMatch(/collect.*Reading.*before.*Listening.*collect.*Listening.*before.*Writing.*collect.*Writing.*before.*Speaking/is);
  expect(paper).toMatch(/project-defined.*precursor/is);
  expect(paper).toMatch(/does not (establish|prove) readiness/i);
  expect(examiner).toMatch(/do not give this file to the learner/i);
  expect(examiner).toMatch(/ten.second.*card.*view/is);
  expect(examiner).toMatch(/2.5 seconds.*twice/is);
  expect(rubric).toMatch(/60%.*each.*skill/is);
  expect(rubric).toMatch(/qualified Japanese review/i);
  expect(rubric).toMatch(/human validation/i);
  expect(paper).not.toMatch(/\b(konnichiwa|onegaishimasu|arigatou|iie|hai)\b/i);
  for (const hiddenTarget of ["みみ", "はな", "て"]) {
    expect(examiner).toContain(hiddenTarget);
    expect(paper).not.toContain(hiddenTarget);
  }
}, 120_000);
