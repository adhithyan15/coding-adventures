import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { parseAssessmentContract } from "../../src/assessment.js";
import {
  defaultCurriculumRoot,
  listAssessmentContracts,
  loadAssessmentPolicy,
} from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Russian continuity", () => expectLanguageContinuity("russian"));
it("pins Russian modality", () => expectLanguageModality("russian"));

it("pins Russian's writing ladder IN ORDER, pre-A1 through A1", () => {
  const russian = languageWritingStages("russian");
  expect(russian.defects).toEqual([]);
  // The pre-A1 runway is привет, observed, copied, copied blind and dictated;
  // the chapter-one checkpoint is a second dictation block. Then chapters
  // 258-260, and chapter 270's B1 letter. ORDER is the point: `missing-stage-prerequisite` rejects a timed
  // or connected paper that sits before a controlled one, so a set-equality
  // assertion would pass on a ladder whose rungs prove nothing.
  expect(russian.validEvidence.map(({ lessonId, stage }) => [lessonId, stage])).toEqual([
    ["RU-W05-privet-observe", "observe-trace"],
    ["RU-W05-privet-guided-copy", "guided-copy"],
    ["RU-W05-privet-delayed-copy", "delayed-copy"],
    ["RU-W05-privet-dictation", "dictation-transcription"],
    ["RU-C01-checkpoint", "dictation-transcription"],
    ["RU-W258-own-line", "controlled-composition"],
    ["RU-W259-with-a-clock", "timed-assessment-production"],
    ["RU-W260-four-lines", "connected-composition"],
    // Chapter 270's six lines about a trip, the first B1 connected writing.
    ["RU-W270-pismo", "connected-composition"],
  ]);
  expect(russian.levels[0]).toMatchObject({ level: "pre-A1", missingStages: [], complete: true });
  expect(russian.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
it("pins Russian lesson-content budgets", () =>
  expectLanguageLessonBudgets("russian", {
    // 88 -> 123: the joining and repair tranche, chapters 16-22, seven chapters
    // of five lessons each. The single new culture claim is that простите is
    // the heavier of the two apologies -- a fact about when a Russian reaches
    // for which word, which is a claim about people rather than about grammar.
    //
    // 142 -> 145: chapter 27, the reading rung -- six words off a board, six
    // lines you would say to a stranger, and a 27-word passage. No new word and
    // no new letter: Russian had already taught its whole alphabet.
    //
    // 145 -> 148: HL-C431, three `review` lessons closing the track's
    // reinforcement debt -- the four opening letters against the verb Russian
    // leaves out, the question family, and ли retrieved four chapters on. The
    // third exists because ли had NO later revisit at all and the gate wants
    // two. No new atoms and no new headwords.
    //
    // 148 -> 151: HL-C443: the letter chapter adds one letter lesson per letter the reader had read in words and never written, plus two reviews. RE-MEASURED against the tree; a letter lesson introduces one script atom and no idiom, sense or culture claim.
    // 151 -> 418: the pre-A1 vocabulary tranche, chapters 29-80. 260 word
    // lessons (twenty-three verbs) and four reviews; each word lesson introduces
    // one lexical atom and no idiom, sense or culture claim. Chapter 81 adds the
    // ё letter lesson and its two reviews (one script atom). Re-measured.
    // 418 -> 695: the A1 vocabulary tranche, chapters 82-135: 265 word lessons,
    // eight reviews, and chapter 93's two letter lessons and two reviews. No idiom,
    // sense or culture claim.
    // 695 -> 696: RU-C03-govorit-false-friend, the atom-budget continuation that
    // carries the govorit/govern etymology. No idiom, sense or culture claim.
    // 696 -> 718: the A2 spine chapters 136-139 (saying no, the past, the
    // future, reading practical texts): twenty word lessons and two reviews. No
    // idiom, sense or culture claim.
    // 718 -> 891: the first A2 vocabulary tranche, chapters 140-172: 165 word
    // lessons (thirty-five verbs) and eight reviews. No idiom, sense or culture
    // claim.
    // 891 -> 1054: the second A2 vocabulary tranche, chapters 173-203: 155 word
    // lessons (thirty verbs) and eight reviews. No idiom, sense or culture
    // claim.
    // 1054 -> 1202: the third A2 vocabulary tranche, chapters 204-231: 140 word
    // lessons (thirty verbs) and eight reviews. No idiom, sense or culture
    // claim.
    // 1202 -> 1338: the fourth A2 vocabulary tranche, chapters 232-257: 130 word
    // lessons (thirty verbs) and six reviews. No idiom, sense or culture claim.
    // 1338 -> 1339: RU-C01-checkpoint, chapter 1's closing checkpoint after the
    // writing runway, so the payoff can assess the ten letter atoms. It
    // introduces nothing: no idiom, sense or culture claim.
    // 1339 -> 1342: chapters 258-260, the A1 writing stages Russian had not
    // proved. No new word or atom.
    // 1342 -> 1376: chapters 261-270, the B1 core that realizes all five B1
    // spine nodes (aspect in the past, reasons, travel, experience, бы): 34
    // lessons, one reading and one writing. No idiom, sense or culture claim.
    lessons: 1376,
    idioms: 0,
    senses: 4,
    cultureClaims: 10,
    unitPrefix: "RU",
  }));

it("pins Russia's project pre-A1 bridge and external A1-to-C2 TORFL targets", () => {
  const policy = loadAssessmentPolicy();
  const russian = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "russian", "assessment.json"), "utf8")),
    "russian",
    policy,
  );

  expect(listAssessmentContracts()).toContain("russian");
  expect(russian.levels.map((level) => level.level)).toEqual(policy.levels);
  expect(russian.levels[0]?.target.basis).toBe("project-defined");
  expect(russian.levels.slice(1).every((level) => level.target.basis === "external")).toBe(true);
  expect(russian.levels[0]?.skills.reading.passThreshold).toBe(0.6);
  expect(russian.levels[0]?.additionalComponents).toEqual({});
  expect(russian.levels.slice(1).every((level) =>
    Object.values(level.skills).every((skill) => skill.passThreshold === 0.66)
  )).toBe(true);
  expect(russian.levels.slice(1).every((level) =>
    level.additionalComponents["lexis-grammar"]?.passThreshold === 0.66
  )).toBe(true);
  expect(russian.levels[0]?.writingStages).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
  expect(russian.levels.at(-1)?.writingStages).toEqual(policy.writingStages.map((stage) => stage.id));
  expect(russian.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
});
