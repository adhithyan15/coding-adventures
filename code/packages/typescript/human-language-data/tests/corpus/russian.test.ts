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
} from "./assert-language-corpus.js";

it("pins Russian continuity", () => expectLanguageContinuity("russian"));
it("pins Russian modality", () => expectLanguageModality("russian"));
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
    lessons: 695,
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
