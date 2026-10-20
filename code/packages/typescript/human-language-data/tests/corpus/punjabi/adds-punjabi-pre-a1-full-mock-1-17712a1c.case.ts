import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

import {
  defaultCurriculumRoot,
  loadAssessmentContracts,
  loadTaskShapeInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  parseMockManifest,
  validateMockAnswerKey,
  validateMockPaper,
  validateMockRubric,
} from "../../../src/mock-manifest.js";
import { readLedgerFile } from "../../../src/shard.js";

function markedPart(markdown: string, partId: string): string {
  const start = `<!-- mock-part:${partId} -->`;
  const end = `<!-- /mock-part:${partId} -->`;
  return markdown.slice(markdown.indexOf(start) + start.length, markdown.indexOf(end));
}

function numberedItems(markdown: string, partId: string): number {
  return markedPart(markdown, partId).match(/^\d+\./gm)?.length ?? 0;
}

it("adds Punjabi pre-A1 full mock 1 without claiming readiness or human validation", () => {
  const root = defaultCurriculumRoot();
  const trackRoot = join(root, "punjabi");
  const contract = loadAssessmentContracts(root)
    .find((entry) => entry.language === "punjabi")?.contract;
  const level = contract?.levels.find((entry) => entry.level === "pre-A1");
  const mock = level?.fullMocks.find((entry) => entry.id === "pre-a1-mock-1");

  expect(mock).toMatchObject({
    id: "pre-a1-mock-1",
    timed: true,
    paper: "mocks/pre-a1/mock-1.md",
    rubric: "mocks/pre-a1/rubric.md",
    answerKey: "mocks/pre-a1/mock-1-answer-key.md",
    manifest: "mocks/pre-a1/mock-1-manifest.json",
  });
  const shape = loadTaskShapeInventory("punjabi", "pre-A1", root);
  const manifest = parseMockManifest(
    readLedgerFile(join(trackRoot, mock!.manifest!)),
    "punjabi",
    "pre-A1",
    "pre-a1-mock-1",
    shape,
  );
  expect(manifest.sections).toEqual([
    {
      skill: "reading",
      minutes: 10,
      partIds: [
        "prea1-reading-signs-and-words",
        "prea1-reading-phrases-and-notices",
        "prea1-reading-personal-details",
      ],
    },
    {
      skill: "listening",
      minutes: 10,
      partIds: [
        "prea1-listening-sounds-and-words",
        "prea1-listening-greeting-responses",
        "prea1-listening-personal-details",
      ],
    },
    {
      skill: "writing",
      minutes: 12,
      partIds: [
        "prea1-writing-delayed-recall",
        "prea1-writing-dictation",
        "prea1-writing-bounded-production",
      ],
    },
    {
      skill: "speaking",
      minutes: 8,
      partIds: [
        "prea1-speaking-greeting-and-identity",
        "prea1-speaking-familiar-questions",
        "prea1-speaking-short-request",
      ],
    },
  ]);

  const paper = readFileSync(join(trackRoot, mock!.paper!), "utf8");
  const rubric = readFileSync(join(trackRoot, mock!.rubric), "utf8");
  const key = readFileSync(join(trackRoot, mock!.answerKey), "utf8");
  validateMockPaper(manifest, paper);
  validateMockRubric(manifest, rubric);
  validateMockAnswerKey(manifest, key);

  expect([
    numberedItems(paper, "prea1-reading-signs-and-words"),
    numberedItems(paper, "prea1-reading-phrases-and-notices"),
    numberedItems(paper, "prea1-reading-personal-details"),
    numberedItems(paper, "prea1-listening-sounds-and-words"),
    numberedItems(paper, "prea1-listening-greeting-responses"),
    numberedItems(paper, "prea1-listening-personal-details"),
    numberedItems(paper, "prea1-writing-delayed-recall"),
    numberedItems(paper, "prea1-writing-dictation"),
    numberedItems(paper, "prea1-writing-bounded-production"),
  ]).toEqual([6, 4, 4, 6, 4, 4, 2, 4, 2]);

  expect(paper).toMatch(/32 written minutes plus 8 speaking\s+minutes/i);
  expect(paper).toMatch(/project-defined practice mock/i);
  expect(paper).toMatch(/does not establish readiness|does not prove readiness/i);
  expect(paper).toMatch(/not printed here/i);
  expect(paper).toMatch(/print the four skill sections as separate packets/i);
  expect(paper).toMatch(
    /collect the Reading packet before\s+giving Listening.*collect Listening before Writing.*collect Writing before\s+Speaking/is,
  );
  expect(paper).not.toMatch(/sat sr[iī] ak[aā]l|mer[aā] n[aā][ṁm]|p[aā]ṇ[iī]|tus[iī]/i);

  const taughtCopy = loadTrackLessons("punjabi", root)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  for (const testedForm of [
    "ਹਾਂ", "ਨਹੀਂ", "ਨਾਂ", "ਪਾਣੀ", "ਰੋਟੀ", "ਦੋਸਤ",
    "ਸਤਿ ਸ੍ਰੀ ਅਕਾਲ", "ਧੰਨਵਾਦ", "ਮਾਫ਼ ਕਰੋ", "ਫਿਰ ਮਿਲਾਂਗੇ",
    "ਮੇਰਾ ਨਾਂ", "ਮੈਂ ਠੀਕ ਹਾਂ", "ਮੈਂ ਪੰਜਾਬੀ ਬੋਲਦੀ ਹਾਂ",
    "ਚਾਹ", "ਦੁੱਧ", "ਭੈਣ", "ਭਰਾ", "ਦਿਲ", "ਨਮਸਤੇ", "ਸ਼ੁਕਰੀਆ",
    "ਤੁਹਾਡਾ ਨਾਂ ਕੀ ਹੈ", "ਮੈਂ ਪੰਜਾਬੀ ਬੋਲਦਾ ਹਾਂ", "ਜੀ", "ਠੀਕ",
    "ਸਲਾਮ", "ਚੰਗਾ ਜੀ", "ਤੁਸੀਂ ਕਿਵੇਂ ਹੋ", "ਕਿਰਪਾ ਕਰਕੇ", "ਮਿਹਰਬਾਨੀ ਕਰਕੇ",
  ]) {
    expect(taughtCopy, `${testedForm} must already be taught`).toContain(testedForm);
  }
  expect(taughtCopy).toContain("maiṁ Amritsar rahindī hāṁ");

  const examiner = readFileSync(join(trackRoot, "mocks/pre-a1/mock-1-examiner-script.md"), "utf8");
  expect(examiner).toMatch(/do not give this file to the learner/i);
  expect(examiner.match(/`mock-1-l\d{2}`/g)).toHaveLength(14);
  expect(examiner.match(/`mock-1-w0[3-6]`/g)).toHaveLength(4);
  expect(examiner.match(/^\d+\./gm)).toHaveLength(26);
  expect(examiner).toContain("ਪਾਣੀ, ਕਿਰਪਾ ਕਰਕੇ।");
  expect(examiner).toContain("Every Part 1 file must last 2–5 seconds");
  expect(examiner).toContain("Every Part 2 file must last 2–8 seconds");
  expect(examiner).toContain("Every Part 3 file must last 2–8 seconds");
  expect(examiner).toMatch(/Every dictation file must last 2–5\s+seconds/);

  const confidentialWritingTargets = ["ਜੀ", "ਸਲਾਮ", "ਚਾਹ", "ਦੁੱਧ", "ਭੈਣ", "ਭਰਾ"];
  for (const target of confidentialWritingTargets) {
    expect(examiner, `${target} must be present in the confidential master`).toContain(`**${target}**`);
    expect(paper, `${target} must not leak into the complete learner paper`).not.toContain(target);
  }

  const greetingListeningPaper = markedPart(paper, "prea1-listening-greeting-responses");
  const greetingListeningKey = markedPart(key, "prea1-listening-greeting-responses");
  expect(examiner).toContain("8. `mock-1-l08`: **ਸ਼ੁਕਰੀਆ।**");
  expect(greetingListeningPaper).toContain("8. A say “you are welcome” / B say “thank you” / C say “no”");
  expect(greetingListeningKey).toContain("7 C; 8 A; 9 B; 10 C.");
  const listeningKey = key.slice(key.indexOf("## Listening"), key.indexOf("## Writing"));
  const listeningAnswers = [...listeningKey.matchAll(/\b\d+ ([ABC])\b/g)].map((match) => match[1]);
  expect(listeningAnswers).toEqual(["B", "C", "B", "A", "C", "B", "C", "A", "B", "C", "A", "B", "C", "B"]);
  expect(Math.max(...["A", "B", "C"].map((letter) => listeningAnswers.filter((answer) => answer === letter).length)))
    .toBeLessThanOrEqual(6);

  expect(rubric).toMatch(/at least 60\/100 on every paper/i);
  expect(rubric).toMatch(/meets\s+this form's scoring threshold/i);
  expect(rubric).not.toMatch(/passes the rung/i);
  expect(rubric).toMatch(/do\s+not by themselves establish.*readiness/is);
  expect(rubric).toMatch(/qualified Punjabi review/i);
  expect(rubric).toMatch(/book-only\s+human validation/i);
}, 120_000);
