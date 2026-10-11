import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "malayalam", "mocks", "c2");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const itemCount = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end).match(/^\d+\./gmu)?.length ?? 0;
};

describe("Malayalam C2 timed mock pair", () => {
  it("owns two complete parallel forms, their keys, and a shared rubric", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name))).toBe(true);

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("Reading — 90 minutes");
      expect(paper).toContain("Listening — 60 minutes");
      expect(paper).toContain("Writing — 90 minutes");
      expect(paper).toContain("Speaking — 25 minutes");
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(40);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(32);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(2);
      expect(itemCount(paper, "## Speaking")).toBe(8);
      expect(paper).toContain("15 minutes preparation");
      expect(paper).toContain("165–200 words per minute");
      expect(paper).toContain("documented regional Malayalam voice");
      expect(paper).toMatch(/[\u0D00-\u0D7F]/u);
      expect(paper).not.toMatch(/Hindi|Devanagari/u);
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("pins independent C2 production and all four keyed skill sections", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("paper must independently reach 60/100");
    expect(rubric).toContain("220–280 word three-source synthesis");
    expect(rubric).toContain("420–520\nword contrasting extended response");
    expect(rubric).toMatch(/traditional or reformed Malayalam orthography/u);
    expect(rubric).toContain("non-Malayalam-script");
    expect(rubric).toContain("fifteen-minute preparation notes");
    expect(rubric).toContain("second-audience reformulation");
    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      for (const heading of ["## Reading", "## Listening", "## Writing", "## Speaking"])
        expect(key).toContain(heading);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).not.toMatch(/Hindi|Devanagari/u);
    }
  });
});
