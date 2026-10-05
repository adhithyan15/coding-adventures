import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "malayalam", "mocks", "pre-a1");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const itemCount = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end).match(/^\d+\./gmu)?.length ?? 0;
};

describe("Malayalam pre-A1 timed mock pair", () => {
  it("owns two executable papers, their keys, and one shared rubric", () => {
    const names = [
      "mock-1.md",
      "mock-2.md",
      "mock-1-answer-key.md",
      "mock-2-answer-key.md",
      "rubric.md",
    ];
    expect(names.every((name) => existsSync(join(root, name)))).toBe(true);

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("## Reading — 10 minutes");
      expect(paper).toContain("## Listening — 10 minutes");
      expect(paper).toContain("## Writing — 12 minutes");
      expect(paper).toContain("## Speaking — 8 minutes");
      expect(paper).toMatch(/[\u0D00-\u0D7F]/u);
      expect(paper).not.toMatch(/Hindi|Devanagari/u);
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(14);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(14);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(8);
      expect(itemCount(paper, "## Speaking")).toBe(6);
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("keeps the four papers independent and pins Malayalam-specific writing evidence", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("60/100 on **every** paper");
    expect(rubric).toContain("chandrakkala placement");
    expect(rubric).toContain("word-final chandrakkala or chillu");
    expect(rubric).toContain("internally consistent conjunct and orthography choices");
    expect(rubric).toContain("Either reformed or traditional orthography");

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
