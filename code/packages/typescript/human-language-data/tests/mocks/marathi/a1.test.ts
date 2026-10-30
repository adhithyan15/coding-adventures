import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";
import { listMockAssessmentStatuses } from "../../../src/mock-assessment.js";

const curriculumRoot = defaultCurriculumRoot();
const root = join(curriculumRoot, "marathi", "mocks", "a1");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const numberedItems = (text: string, start: string, end?: string) => {
  const from = text.indexOf(start);
  const to = end === undefined ? text.length : text.indexOf(end, from + start.length);
  return text.slice(from, to).match(/^\d+\./gmu)?.length ?? 0;
};

const partIds = [
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
] as const;

describe("Marathi A1 timed mock pair", () => {
  it("binds both timed mocks to every published A1 task-shape part", () => {
    expect(listMockAssessmentStatuses(curriculumRoot).find(
      (status) => status.language === "marathi" && status.level === "A1",
    )).toMatchObject({
      requiredMocks: 2,
      completeMocks: 2,
      missingMocks: [],
      blockedBy: [],
    });

    for (const form of [1, 2]) {
      const manifest = JSON.parse(read(`mock-${form}-manifest.json`)) as {
        language: string;
        level: string;
        mockId: string;
        sections: Array<{ skill: string; minutes: number; partIds: string[] }>;
      };
      expect(manifest).toMatchObject({
        language: "marathi",
        level: "A1",
        mockId: `a1-mock-${form}`,
      });
      expect(manifest.sections.map(({ skill, minutes }) => ({ skill, minutes }))).toEqual([
        { skill: "reading", minutes: 20 },
        { skill: "listening", minutes: 18 },
        { skill: "writing", minutes: 20 },
        { skill: "speaking", minutes: 10 },
      ]);
      expect(manifest.sections.flatMap((section) => section.partIds)).toEqual(partIds);
    }
  });

  it("owns two distinct executable papers, keys, scripts, manifests, and one rubric", () => {
    const names = [
      "mock-1.md",
      "mock-2.md",
      "mock-1-answer-key.md",
      "mock-2-answer-key.md",
      "mock-1-examiner-script.md",
      "mock-2-examiner-script.md",
      "mock-1-manifest.json",
      "mock-2-manifest.json",
      "rubric.md",
    ];
    expect(names.every((name) => existsSync(join(root, name)))).toBe(true);

    for (const form of [1, 2]) {
      const paper = read(`mock-${form}.md`);
      expect(paper).toContain("## Reading — 20 minutes");
      expect(paper).toContain("## Listening — 18 minutes");
      expect(paper).toContain("## Writing — 20 minutes");
      expect(paper).toContain("## Speaking — 10 minutes");
      expect(paper).toMatch(/[\u0900-\u097F]/u);
      expect(numberedItems(paper, "## Reading", "## Listening")).toBe(20);
      expect(numberedItems(paper, "## Listening", "## Writing")).toBe(17);
      expect(numberedItems(paper, "## Writing", "## Speaking")).toBe(2);
      expect(numberedItems(paper, "## Speaking")).toBe(3);

      const script = read(`mock-${form}-examiner-script.md`);
      expect(script).toContain("60–90 second window");
      expect(script).toContain("90–120 second window");
      expect(script).toContain("120–180 second window");
      expect(script).toContain("90–110 words per minute");
      expect(script).toContain("two plays");
      expect(script).toContain("60–120 second response window");
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    expect(read("mock-1-examiner-script.md")).not.toBe(read("mock-2-examiner-script.md"));
  });

  it("pins one visible evidence span per part and preserves the evidence boundary", () => {
    const rubric = read("rubric.md");
    for (const partId of partIds) {
      const open = `<!-- mock-part:${partId} -->`;
      const close = `<!-- /mock-part:${partId} -->`;
      for (const name of ["mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md"]) {
        const artifact = read(name);
        expect(artifact.split(open)).toHaveLength(2);
        expect(artifact.split(close)).toHaveLength(2);
        expect(artifact.indexOf(close)).toBeGreaterThan(artifact.indexOf(open) + open.length);
      }
    }
    expect(rubric).toContain("60/100 on **every** paper");
    expect(rubric).toContain("no compensatory total");
    expect(rubric).toContain("documented regional Marathi form");
    expect(rubric).toContain("does not establish exam readiness");
    expect(rubric).toContain("recording provenance");
    expect(rubric).toContain("independent human validation");
  });
});
