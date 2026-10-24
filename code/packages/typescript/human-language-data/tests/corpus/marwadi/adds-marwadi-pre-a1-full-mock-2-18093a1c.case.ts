import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

import {
  defaultCurriculumRoot,
  loadAssessmentContracts,
  loadTaskShapeInventory,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  parseMockManifest,
  validateMockAnswerKey,
  validateMockPaper,
  validateMockRubric,
} from "../../../src/mock-manifest.js";
import { readLedgerFile } from "../../../src/shard.js";

function part(markdown: string, id: string): string {
  const start = `<!-- mock-part:${id} -->`;
  const end = `<!-- /mock-part:${id} -->`;
  expect(markdown).toContain(start);
  expect(markdown).toContain(end);
  return markdown.slice(markdown.indexOf(start) + start.length, markdown.indexOf(end));
}

it("adds a distinct structural Marwadi pre-A1 mock 2 from floor-taught material", () => {
  const root = defaultCurriculumRoot();
  const trackRoot = join(root, "marwadi");
  const contract = loadAssessmentContracts(root)
    .find((entry) => entry.language === "marwadi")?.contract;
  const level = contract?.levels.find((entry) => entry.level === "pre-A1");
  const mock = level?.fullMocks.find((entry) => entry.id === "pre-a1-mock-2");
  expect(mock).toMatchObject({
    timed: true,
    paper: "mocks/pre-a1/mock-2.md",
    rubric: "mocks/pre-a1/rubric.md",
    answerKey: "mocks/pre-a1/mock-2-answer-key.md",
    manifest: "mocks/pre-a1/mock-2-manifest.json",
  });

  const shape = loadTaskShapeInventory("marwadi", "pre-A1", root);
  const manifest = parseMockManifest(
    readLedgerFile(join(trackRoot, mock!.manifest!)),
    "marwadi", "pre-A1", "pre-a1-mock-2", shape,
  );
  expect(manifest.sections.map(({ skill, minutes, partIds }) =>
    [skill, minutes, partIds.length])).toEqual([
    ["reading", 10, 3], ["listening", 10, 3],
    ["writing", 12, 3], ["speaking", 8, 3],
  ]);

  const paper = readFileSync(join(trackRoot, mock!.paper!), "utf8");
  const key = readFileSync(join(trackRoot, mock!.answerKey), "utf8");
  const rubric = readFileSync(join(trackRoot, mock!.rubric!), "utf8");
  const examiner = readFileSync(join(trackRoot, "mocks/pre-a1/mock-2-examiner-script.md"), "utf8");
  const firstPaper = readFileSync(join(trackRoot, "mocks/pre-a1/mock-1.md"), "utf8");
  validateMockPaper(manifest, paper);
  validateMockAnswerKey(manifest, key);
  validateMockRubric(manifest, rubric);
  expect(paper).not.toEqual(firstPaper);

  const ids = manifest.sections.slice(0, 3).flatMap((section) => section.partIds);
  expect(ids.map((id) => part(paper, id).match(/^\d+\./gm)?.length ?? 0)).toEqual(
    [6, 4, 4, 6, 4, 4, 2, 4, 2],
  );
  for (const section of manifest.sections) {
    for (const id of section.partIds) expect(part(key, id).trim().length).toBeGreaterThan(0);
  }
  const readingTargets = [
    ["घर", "home", "B"], ["भाई", "brother", "C"],
    ["बाजार", "market", "A"], ["मंदिर", "temple", "B"],
    ["बस", "bus", "C"], ["बादल", "cloud", "A"],
  ] as const;
  const readingWords = part(paper, "prea1-reading-signs-and-words");
  const readingKey = part(key, "prea1-reading-signs-and-words");
  readingTargets.forEach(([word, meaning, answer], index) => {
    const item = index + 1;
    const line = readingWords.split("\n").find((candidate) => candidate.startsWith(`${item}. `));
    expect(line).toContain(`**${word}**`);
    expect(line).toContain(`${answer} ${meaning}`);
    expect(readingKey).toMatch(new RegExp(`(?:^|[;\\s])${item} ${answer}(?:[;.\\s]|$)`));
  });
  expect(readingWords).not.toContain("**पाणी**");

  const listeningTargets = [
    ["दादा", "B"], ["बहन", "C"], ["घर", "A"], ["बस", "B"],
    ["बादल", "C"], ["चूरमा", "A"], ["हां सा।", "B"],
    ["आप कैसो हो?", "C"], ["हूं ठीक हूं।", "A"],
    ["पाछे मिलसू।", "B"], ["परिवार: दादी।", "C"],
    ["नाम: राम।", "A"], ["घर: सहर।", "B"], ["परिवार: भाई।", "C"],
  ] as const;
  const listeningKey = manifest.sections[1]!.partIds.map((id) => part(key, id)).join("\n");
  listeningTargets.forEach(([spoken, answer], index) => {
    const item = index + 1;
    const file = `mock-2-l${String(item).padStart(2, "0")}`;
    expect(examiner).toContain(`\`${file}\`: **${spoken}**`);
    expect(listeningKey).toMatch(new RegExp(`(?:^|[;\\s])${item} ${answer}(?:[;.\\s]|$)`));
  });
  const listeningPacket = paper.slice(paper.indexOf("## Listening —"), paper.indexOf("## Writing —"));
  expect(listeningPacket).not.toMatch(/[\u0900-\u097f]/u);
  const writingPacket = paper.slice(paper.indexOf("## Writing —"), paper.indexOf("## Speaking —"));
  for (const dictated of ["बाजार", "दादी", "बस", "सहर"]) {
    expect(writingPacket).not.toContain(dictated);
  }

  expect(paper).toMatch(/32 written minutes plus 8 speaking minutes/i);
  expect(paper).toMatch(/project-defined practice mock/i);
  expect(paper).toMatch(/does not establish readiness/i);
  expect(paper).toMatch(/collect the Reading packet before.*Listening.*collect Listening before Writing.*collect Writing before Speaking/is);
  expect(examiner).toMatch(/do not give this file to the learner/i);
  expect(rubric).toMatch(/at least 60\/100 on every paper/i);
  expect(rubric).not.toMatch(/passes the rung/i);

  // Resolve authored extension stages. A chapter ordinal is not a CEFR level.
  const curriculum = loadTrackLanguageCurriculum("marwadi", root)!;
  const floorIds = new Set(curriculum.extensions
    .filter((extension) => extension.stage === "pre-A1")
    .flatMap((extension) => extension.lessons));
  const floorLessons = loadTrackLessons("marwadi", root)
    .filter((lesson) => floorIds.has(lesson.frontmatter.id));
  const taughtFloor = floorLessons
    .flatMap((lesson) => [String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown)])
    .join("\n");
  for (const target of ["घर", "भाई", "बाजार", "मंदिर", "बस", "बादल",
    "सहर", "दादा", "बहन", "चूरमा", "दादी", "हां सा", "पाछे मिलसू",
    "परिवार", "नाम", "राम", "आप कैसो हो", "हूं ठीक हूं", "पाणी",
    "लावो", "गांव", "ये कपड़ा दिखावो", "पैसा ले लो"]) {
    expect(taughtFloor, `${target} must be taught in pre-A1`).toContain(target);
  }
});
