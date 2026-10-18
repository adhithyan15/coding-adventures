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

function headingSection(markdown: string, heading: string, nextHeading?: string): string {
  const start = markdown.indexOf(heading);
  expect(start, `missing heading: ${heading}`).toBeGreaterThanOrEqual(0);
  const end = nextHeading === undefined
    ? markdown.length
    : markdown.indexOf(nextHeading, start + heading.length);
  if (nextHeading !== undefined) {
    expect(end, `missing heading after ${heading}: ${nextHeading}`).toBeGreaterThan(start);
  }
  return markdown.slice(start, end);
}

function selectedResponseAnswers(answerKey: string, startHeading: string, endHeading: string): string[] {
  const section = answerKey.slice(answerKey.indexOf(startHeading), answerKey.indexOf(endHeading));
  return [...section.matchAll(/\b\d+ ([ABC])\b/g)].map((match) => match[1]);
}

function numberedItems(markdown: string, partId: string): number {
  return markedPart(markdown, partId).match(/^\d+\./gm)?.length ?? 0;
}

function examinerListeningBodies(markdown: string): string[] {
  return [...markdown.matchAll(/^\d+\. `a1-mock-2-l\d{2}`: \*\*(.+)\*\*$/gm)]
    .map((match) => match[1].trim());
}

function gurmukhiWords(markdown: string): number {
  return markdown.match(/[\u0A00-\u0A7F]+/gu)?.length ?? 0;
}

function itemPromptWords(markdown: string, partId: string): number {
  const part = markedPart(markdown, partId);
  const item = part.slice(part.search(/^\d+\./m)).replace(/^\d+\.\s*/, "");
  return item.match(/[\p{L}\p{M}\p{N}]+(?:[’'-][\p{L}\p{M}\p{N}]+)*/gu)?.length ?? 0;
}

function expectInRange(value: number, minimum: number, maximum: number): void {
  expect(value).toBeGreaterThanOrEqual(minimum);
  expect(value).toBeLessThanOrEqual(maximum);
}

it("adds a distinct Punjabi A1 full mock 2 without changing the shared rubric", () => {
  const root = defaultCurriculumRoot();
  const trackRoot = join(root, "punjabi");
  const contract = loadAssessmentContracts(root)
    .find((entry) => entry.language === "punjabi")?.contract;
  const level = contract?.levels.find((entry) => entry.level === "A1");
  const mock = level?.fullMocks.find((entry) => entry.id === "a1-mock-2");

  expect(mock).toMatchObject({
    id: "a1-mock-2",
    timed: true,
    paper: "mocks/a1/mock-2.md",
    rubric: "mocks/a1/rubric.md",
    answerKey: "mocks/a1/mock-2-answer-key.md",
    manifest: "mocks/a1/mock-2-manifest.json",
  });
  const shape = loadTaskShapeInventory("punjabi", "A1", root);
  const manifest = parseMockManifest(
    readLedgerFile(join(trackRoot, mock!.manifest!)),
    "punjabi",
    "A1",
    "a1-mock-2",
    shape,
  );
  expect(manifest.sections.map((section) => [section.skill, section.minutes, section.partIds.length]))
    .toEqual([
      ["reading", 20, 3],
      ["listening", 18, 3],
      ["writing", 20, 2],
      ["speaking", 10, 3],
    ]);

  const paper = readFileSync(join(trackRoot, mock!.paper!), "utf8");
  const rubric = readFileSync(join(trackRoot, mock!.rubric), "utf8");
  const key = readFileSync(join(trackRoot, mock!.answerKey), "utf8");
  const examiner = readFileSync(join(trackRoot, "mocks/a1/mock-2-examiner-script.md"), "utf8");
  const form1Paper = readFileSync(join(trackRoot, "mocks/a1/mock-1.md"), "utf8");
  const form1Examiner = readFileSync(join(trackRoot, "mocks/a1/mock-1-examiner-script.md"), "utf8");
  const form1Key = readFileSync(join(trackRoot, "mocks/a1/mock-1-answer-key.md"), "utf8");
  validateMockPaper(manifest, paper);
  validateMockRubric(manifest, rubric);
  validateMockAnswerKey(manifest, key);

  expect([
    numberedItems(paper, "a1-reading-signs-and-forms"),
    numberedItems(paper, "a1-reading-short-messages"),
    numberedItems(paper, "a1-reading-personal-descriptions"),
    numberedItems(paper, "a1-listening-announcements"),
    numberedItems(paper, "a1-listening-short-exchanges"),
    numberedItems(paper, "a1-listening-personal-account"),
    numberedItems(paper, "a1-writing-practical-form"),
    numberedItems(paper, "a1-writing-reader-purpose-message"),
    numberedItems(paper, "a1-speaking-personal-interview"),
    numberedItems(paper, "a1-speaking-prepared-description"),
    numberedItems(paper, "a1-speaking-transactional-role-play"),
  ]).toEqual([7, 7, 6, 6, 6, 5, 1, 1, 5, 1, 1]);

  expectInRange(
    gurmukhiWords(markedPart(paper, "a1-reading-signs-and-forms").split("#### Questions")[0]),
    70,
    90,
  );
  expectInRange(
    gurmukhiWords(markedPart(paper, "a1-reading-short-messages").split("#### Questions")[0]),
    90,
    120,
  );
  expectInRange(
    gurmukhiWords(markedPart(paper, "a1-reading-personal-descriptions").split("#### Questions")[0]),
    90,
    140,
  );
  expectInRange(itemPromptWords(paper, "a1-writing-practical-form"), 40, 70);
  expectInRange(itemPromptWords(paper, "a1-writing-reader-purpose-message"), 25, 50);

  expect(paper).toMatch(/58 written minutes plus 10 speaking\s+minutes/i);
  expect(paper).toMatch(/five-minute speaking preparation/i);
  expect(paper).toMatch(/does not establish readiness/i);
  expect(paper).toMatch(/not\s+printed here/i);
  expect(paper).toMatch(/print the four skill sections as separate packets/i);
  expect(paper).toMatch(
    /collect the Reading packet before\s+giving Listening.*collect Listening before Writing.*collect Writing before\s+Speaking/is,
  );
  expect(paper).not.toMatch(/[A-Za-z][āīūṛṅñṭḍṇśṣṁḥ]/i);
  expect([...markedPart(paper, "a1-reading-signs-and-forms")
    .matchAll(/^\*\*([^\s.]+)\.\*\*/gm)]
    .map((match) => match[1])).toEqual(["ਕ", "ਖ", "ਗ", "ਘ", "ਚ", "ਛ", "ਜ"]);

  const listeningBodies = examinerListeningBodies(examiner);
  expect(listeningBodies).toHaveLength(17);
  for (const body of listeningBodies) {
    expect(paper, `examiner-only listening body leaked: ${body}`).not.toContain(body);
    expect(form1Examiner, `form 2 reused a form 1 listening body: ${body}`).not.toContain(body);
  }
  expect(examiner).toMatch(/do not give this file to the learner/i);
  expect(examiner).toMatch(/announcements part must last 60–90 seconds/i);
  expect(examiner).toMatch(/short-exchanges\s+part must last 90–120 seconds/i);
  expect(examiner).toMatch(/personal-account part must last\s+120–180 seconds/i);
  expect(examiner).toMatch(/90–110 words per minute/i);
  expect(examiner).toMatch(/play every named\s+file twice/i);
  expectInRange(listeningBodies.slice(0, 6).reduce((sum, body) => sum + gurmukhiWords(body), 0), 90, 165);
  expectInRange(listeningBodies.slice(6, 12).reduce((sum, body) => sum + gurmukhiWords(body), 0), 135, 220);
  expectInRange(listeningBodies.slice(12).reduce((sum, body) => sum + gurmukhiWords(body), 0), 180, 330);

  expect(examiner).toMatch(/five prompts in 10–45 seconds total.*60–120 seconds of candidate response/is);
  expect(examiner).toMatch(/exactly 60-second response/i);
  expect(examiner).toMatch(/opening and necessary follow-up in 10–30 seconds total.*60–120 seconds/is);
  const descriptionCue = examiner.match(/exactly 60-second response:\s*\n\*\*(.+?)\*\*/s)?.[1] ?? "";
  expectInRange(gurmukhiWords(descriptionCue), 10, 30);
  expect(markedPart(paper, "a1-speaking-personal-interview"))
    .toMatch(/five questions over 60–120 seconds in total/i);
  expect(markedPart(paper, "a1-speaking-prepared-description"))
    .toMatch(/Speak for one minute/i);
  expect(markedPart(paper, "a1-speaking-transactional-role-play"))
    .toMatch(/exchange for 60–120 seconds/i);

  const taughtCopy = loadTrackLessons("punjabi", root)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const testedPunjabi = `${paper}\n${examiner}`;
  const tokens = new Set(testedPunjabi.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const token of tokens) {
    expect(taughtCopy, `${token} must already occur in a Punjabi lesson`).toContain(token);
  }

  const form = markedPart(paper, "a1-writing-practical-form");
  for (const field of ["ਨਾਂ", "ਭਾਸ਼ਾ", "ਰਿਹਾਇਸ਼", "ਕੰਮ", "ਉਮਰ", "ਫ਼ੋਨ", "ਤਾਰੀਖ਼"]) {
    expect(form).toContain(`**${field}:**`);
  }
  const message = markedPart(paper, "a1-writing-reader-purpose-message");
  expect(message).toMatch(/30–40 Punjabi words/i);
  expect(message).toContain("**ਅਮਨ**");
  expect(message).not.toMatch(/possible response|answer model/i);

  const readingAnswers = selectedResponseAnswers(key, "## Reading", "## Listening");
  const listeningAnswers = selectedResponseAnswers(key, "## Listening", "## Writing");
  const form1ReadingAnswers = selectedResponseAnswers(form1Key, "## Reading", "## Listening");
  const form1ListeningAnswers = selectedResponseAnswers(form1Key, "## Listening", "## Writing");
  expect(readingAnswers).toEqual([
    "B", "C", "A", "B", "C", "A", "B", "C", "A", "B",
    "C", "A", "B", "C", "A", "B", "C", "A", "B", "C",
  ]);
  expect(listeningAnswers).toEqual([
    "C", "A", "B", "C", "A", "B", "C", "A", "B",
    "C", "A", "B", "B", "C", "A", "B", "C",
  ]);
  expect(paper).toContain("5. What is at school? A book / B bag / C key");
  expect(paper).toContain("13. Where does the speaker live? A a city / B a village / C a school");
  expect(examiner).toContain("ਰਿਹਾਇਸ਼: ਪਿੰਡ।");
  expect(Math.max(...["A", "B", "C"].map((letter) => readingAnswers.filter((answer) => answer === letter).length)))
    .toBeLessThanOrEqual(8);
  expect(Math.max(...["A", "B", "C"].map((letter) => listeningAnswers.filter((answer) => answer === letter).length)))
    .toBeLessThanOrEqual(7);

  const markedPartIds = [
    "a1-reading-signs-and-forms",
    "a1-reading-short-messages",
    "a1-reading-personal-descriptions",
    "a1-listening-announcements",
    "a1-listening-short-exchanges",
    "a1-listening-personal-account",
    "a1-writing-practical-form",
    "a1-writing-reader-purpose-message",
    "a1-speaking-personal-interview",
    "a1-speaking-prepared-description",
    "a1-speaking-transactional-role-play",
  ];
  for (const partId of markedPartIds) {
    expect(markedPart(paper, partId), `mock 2 learner part ${partId} must differ from mock 1`)
      .not.toBe(markedPart(form1Paper, partId));
  }

  const examinerSpeakingHeadings = [
    "### Part 1 — personal interview",
    "### Part 2 — prepared description",
    "### Part 3 — transactional role-play",
  ];
  for (const [index, heading] of examinerSpeakingHeadings.entries()) {
    const nextHeading = examinerSpeakingHeadings[index + 1];
    expect(
      headingSection(examiner, heading, nextHeading),
      `mock 2 examiner section ${heading} must differ from mock 1`,
    ).not.toBe(headingSection(form1Examiner, heading, nextHeading));
  }

  expect(readingAnswers).not.toEqual(form1ReadingAnswers);
  expect(listeningAnswers).not.toEqual(form1ListeningAnswers);
  expect(markedPart(paper, "a1-speaking-personal-interview")).toMatch(/familiar object/i);
  expect(markedPart(paper, "a1-speaking-prepared-description")).toMatch(/familiar person/i);
  expect(markedPart(form1Paper, "a1-speaking-prepared-description")).toMatch(/familiar place/i);
  expect(markedPart(paper, "a1-speaking-transactional-role-play"))
    .toMatch(/small shop[\s\S]*book or pen/i);
  expect(markedPart(form1Paper, "a1-speaking-transactional-role-play"))
    .toMatch(/small café[\s\S]*tea or water/i);
  expect(rubric).toMatch(/at least 60\/100 on every paper/i);
  expect(rubric).toMatch(/35 points.*35 points.*30 points/is);
  expect(rubric).toMatch(/40 points.*60 points/is);
  expect(rubric).toMatch(/30 points.*30 points.*40 points/is);
  expect(rubric).not.toMatch(/passes the rung/i);
  expect(rubric).toMatch(/do\s+not by themselves establish.*readiness/is);
  expect(rubric).toMatch(/qualified Punjabi review/i);
  expect(rubric).toMatch(/book-only\s+human validation/i);
}, 120_000);
