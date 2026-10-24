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

function part(markdown: string, id: string): string {
  const start = `<!-- mock-part:${id} -->`;
  const end = `<!-- /mock-part:${id} -->`;
  expect(markdown).toContain(start);
  expect(markdown).toContain(end);
  return markdown.slice(markdown.indexOf(start) + start.length, markdown.indexOf(end));
}

it("adds only a structural Marwadi pre-A1 mock 1 from taught material", () => {
  const root = defaultCurriculumRoot();
  const trackRoot = join(root, "marwadi");
  const contract = loadAssessmentContracts(root)
    .find((entry) => entry.language === "marwadi")?.contract;
  const level = contract?.levels.find((entry) => entry.level === "pre-A1");
  const mock = level?.fullMocks.find((entry) => entry.id === "pre-a1-mock-1");
  expect(mock).toMatchObject({
    timed: true,
    paper: "mocks/pre-a1/mock-1.md",
    rubric: "mocks/pre-a1/rubric.md",
    answerKey: "mocks/pre-a1/mock-1-answer-key.md",
    manifest: "mocks/pre-a1/mock-1-manifest.json",
  });

  const shape = loadTaskShapeInventory("marwadi", "pre-A1", root);
  const manifest = parseMockManifest(
    readLedgerFile(join(trackRoot, mock!.manifest!)),
    "marwadi", "pre-A1", "pre-a1-mock-1", shape,
  );
  expect(manifest.sections).toEqual([
    {
      skill: "reading", minutes: 10,
      partIds: ["prea1-reading-signs-and-words", "prea1-reading-phrases-and-notices", "prea1-reading-personal-details"],
    },
    {
      skill: "listening", minutes: 10,
      partIds: ["prea1-listening-sounds-and-words", "prea1-listening-greeting-responses", "prea1-listening-personal-details"],
    },
    {
      skill: "writing", minutes: 12,
      partIds: ["prea1-writing-delayed-recall", "prea1-writing-dictation", "prea1-writing-bounded-production"],
    },
    {
      skill: "speaking", minutes: 8,
      partIds: ["prea1-speaking-greeting-and-identity", "prea1-speaking-familiar-questions", "prea1-speaking-short-request"],
    },
  ]);

  const paper = readFileSync(join(trackRoot, mock!.paper!), "utf8");
  const key = readFileSync(join(trackRoot, mock!.answerKey), "utf8");
  const rubric = readFileSync(join(trackRoot, mock!.rubric), "utf8");
  const examiner = readFileSync(join(trackRoot, "mocks/pre-a1/mock-1-examiner-script.md"), "utf8");
  validateMockPaper(manifest, paper);
  validateMockAnswerKey(manifest, key);
  validateMockRubric(manifest, rubric);

  const counts = [6, 4, 4, 6, 4, 4, 2, 4, 2];
  const ids = manifest.sections.slice(0, 3).flatMap((section) => section.partIds);
  expect(ids.map((id) => part(paper, id).match(/^\d+\./gm)?.length ?? 0)).toEqual(counts);
  for (const section of manifest.sections) {
    for (const id of section.partIds) {
      expect(part(key, id)).not.toHaveLength(0);
    }
  }
  const listeningTargets = [
    ["पाणी", "A"], ["चाय", "B"], ["रोटी", "C"], ["मां", "A"],
    ["बाप", "C"], ["कोनी", "B"], ["राम-राम सा।", "C"],
    ["आभार।", "A"], ["थारो नाम कांई है?", "C"], ["पाछे मिलसू।", "C"],
    ["म्हारो नाम राम है।", "A"], ["नाम: राम।", "B"],
    ["परिवार: मां।", "A"], ["घर: गांव।", "B"],
  ] as const;
  const listeningIds = manifest.sections[1]!.partIds;
  const listeningKey = listeningIds.map((id) => part(key, id)).join("\n");
  listeningTargets.forEach(([spoken, answer], index) => {
    const item = index + 1;
    const file = `mock-1-l${String(item).padStart(2, "0")}`;
    expect(examiner).toContain(`\`${file}\`: **${spoken}**`);
    expect(listeningKey).toMatch(new RegExp(`(?:^|[;\\s])${item} ${answer}(?:[;.\\s]|$)`));
  });
  const listeningPacket = paper.slice(paper.indexOf("## Listening —"), paper.indexOf("## Writing —"));
  expect(listeningPacket).not.toMatch(/[\u0900-\u097f]/u);
  const writingPacket = paper.slice(paper.indexOf("## Writing —"), paper.indexOf("## Speaking —"));
  for (const dictated of ["पाणी", "चाय", "रोटी", "मां"]) {
    expect(writingPacket).not.toContain(dictated);
  }

  expect(paper).toMatch(/32 written minutes plus 8 speaking minutes/i);
  expect(paper).toMatch(/project-defined practice mock/i);
  expect(paper).toMatch(/does not prove readiness|does not establish readiness/i);
  expect(paper).toMatch(/collect the Reading packet before.*Listening.*collect Listening before Writing.*collect Writing before Speaking/is);
  expect(examiner).toMatch(/do not give this file to the learner/i);
  expect(rubric).toMatch(/at least 60\/100 on every paper/i);
  expect(rubric).not.toMatch(/passes the rung/i);

  const taughtCopy = loadTrackLessons("marwadi", root)
    .flatMap((lesson) => [String(lesson.frontmatter.headword ?? ""), ...lesson.blocks.map((block) => block.markdown)])
    .join("\n");
  for (const target of ["राम-राम सा", "आभार", "हां सा", "पाणी", "म्हारो", "नाम", "पाछे मिलसू", "चाय", "रोटी"]) {
    expect(taughtCopy, `${target} must precede the paper`).toContain(target);
  }
  for (const confidentialTarget of ["म्हारो", "पाणी"]) {
    expect(examiner).toContain(confidentialTarget);
    expect(part(paper, "prea1-writing-dictation")).not.toContain(confidentialTarget);
  }
});
