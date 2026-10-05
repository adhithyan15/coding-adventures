import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "malayalam", "mocks", "b2");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const itemCount = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end).match(/^\d+\./gmu)?.length ?? 0;
};

describe("Malayalam B2 timed mock pair", () => {
  it("owns two complete parallel forms, their keys, and a shared rubric", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name))).toBe(true);

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("Reading — 60 minutes");
      expect(paper).toContain("Listening — 45 minutes");
      expect(paper).toContain("Writing — 60 minutes");
      expect(paper).toContain("Speaking — 18 minutes");
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(32);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(28);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(2);
      expect(itemCount(paper, "## Speaking")).toBe(7);
      expect(paper).toContain("ten minutes");
      expect(paper).toMatch(/[\u0D00-\u0D7F]/u);
      expect(paper).not.toMatch(/Hindi|Devanagari/u);
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("pins independent B2 production and all four keyed skill sections", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("every\npaper must independently reach 60/100");
    expect(rubric).toContain("100–130 word interaction text");
    expect(rubric).toContain("220–280 word extended text");
    expect(rubric).toMatch(/traditional or reformed Malayalam orthography/u);
    expect(rubric).toContain("non-Malayalam-script");
    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      for (const heading of ["## Reading", "## Listening", "## Writing", "## Speaking"])
        expect(key).toContain(heading);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).not.toMatch(/Hindi|Devanagari/u);
    }
  });
});
