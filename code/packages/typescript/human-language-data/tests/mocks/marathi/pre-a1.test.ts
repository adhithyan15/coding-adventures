import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";
import { listMockAssessmentStatuses } from "../../../src/mock-assessment.js";

const curriculumRoot = defaultCurriculumRoot();
const root = join(curriculumRoot, "marathi", "mocks", "pre-a1");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const itemCount = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end).match(/^\d+\./gmu)?.length ?? 0;
};

const partIds = [
  "prea1-reading-signs-and-words",
  "prea1-reading-phrases-and-notices",
  "prea1-reading-personal-details",
  "prea1-listening-sounds-and-words",
  "prea1-listening-greeting-responses",
  "prea1-listening-personal-details",
  "prea1-writing-delayed-recall",
  "prea1-writing-dictation",
  "prea1-writing-bounded-production",
  "prea1-speaking-greeting-and-identity",
  "prea1-speaking-familiar-questions",
  "prea1-speaking-short-request",
] as const;

describe("Marathi pre-A1 timed mock pair", () => {
  it("binds both timed mocks to every published task-shape part", () => {
    expect(listMockAssessmentStatuses(curriculumRoot).find(
      (status) => status.language === "marathi" && status.level === "pre-A1",
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
        sections: Array<{ partIds: string[] }>;
      };
      expect(manifest).toMatchObject({
        language: "marathi",
        level: "pre-A1",
        mockId: `pre-a1-mock-${form}`,
      });
      expect(manifest.sections.flatMap((section) => section.partIds)).toEqual(partIds);
    }
  });

  it("owns two executable papers, keys, examiner scripts, manifests, and one shared rubric", () => {
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

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("## Reading — 10 minutes");
      expect(paper).toContain("## Listening — 10 minutes");
      expect(paper).toContain("## Writing — 12 minutes");
      expect(paper).toContain("## Speaking — 8 minutes");
      expect(paper).toMatch(/[\u0900-\u097F]/u);
      expect(paper).not.toMatch(/romanization|transcript of the listening script/iu);
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(14);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(14);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(8);
      expect(itemCount(paper, "## Speaking")).toBe(6);
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    expect(read("mock-1-examiner-script.md")).not.toBe(read("mock-2-examiner-script.md"));

    for (const name of ["mock-1-examiner-script.md", "mock-2-examiner-script.md"]) {
      const script = read(name);
      expect(script).toContain("items 1–6, put");
      expect(script).toContain("inside one 2–5 second window");
      expect(script).toContain("items 7–14");
      expect(script).toContain("inside one 2–8 second window");
      expect(script).toContain("dictation\nitems W3–W6, use one 2–5 second window");
      expect(script).toContain("deliver each prompt in 2–8 seconds");
      expect(script).toContain("allow a 2–15 second response\nwindow");
      expect(script).toContain("single replay\ndeclared by the task shape");
    }
  });

  it("keeps the four papers independent and pins Marathi-specific evidence", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("60/100 on **every** paper");
    expect(rubric).toContain("matra and anusvara placement");
    expect(rubric).toContain("internally consistent gender agreement");
    expect(rubric).toContain("documented regional Marathi form");
    expect(rubric).toContain("does not establish exam readiness");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).toContain("## Reading");
      expect(key).toContain("## Listening");
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toMatch(/[\u0900-\u097F]/u);
    }
  });
});
