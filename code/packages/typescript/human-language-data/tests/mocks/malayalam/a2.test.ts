import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "malayalam", "mocks", "a2");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const itemCount = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end).match(/^\d+\./gmu)?.length ?? 0;
};

describe("Malayalam A2 timed mock pair", () => {
  it("owns two complete parallel forms, their keys, and a shared rubric", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) {
      expect(existsSync(join(root, name))).toBe(true);
    }

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("Reading — 30 minutes");
      expect(paper).toContain("Listening — 25 minutes");
      expect(paper).toContain("Writing — 30 minutes");
      expect(paper).toContain("Speaking — 12 minutes");
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(24);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(20);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(2);
      expect(itemCount(paper, "## Speaking")).toBe(3);
      expect(paper).toContain("five minutes");
      expect(paper).toMatch(/[\u0D00-\u0D7F]/u);
      expect(paper).not.toMatch(/Hindi|Devanagari/u);
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("pins independent A2 production and all four keyed skill sections", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("every\npaper must independently reach 60/100");
    expect(rubric).toContain("25–35 word functional response");
    expect(rubric).toContain("70–90 word connected message or description");
    expect(rubric).toMatch(/traditional or\nreformed Malayalam orthography/u);
    expect(rubric).toContain("non-Malayalam-script");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).toContain("## Reading");
      expect(key).toContain("## Listening");
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).not.toMatch(/Hindi|Devanagari/u);
    }
  });
});
