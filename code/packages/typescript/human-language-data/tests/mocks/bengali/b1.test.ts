import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "bengali", "mocks", "b1");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const itemCount = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end).match(/^\d+\./gmu)?.length ?? 0;
};
const section = (paper: string, heading: string, nextHeading: string) => {
  const start = paper.indexOf(heading);
  return paper.slice(start, paper.indexOf(nextHeading, start + heading.length));
};
const boldBlockWordCounts = (text: string) =>
  [...text.matchAll(/\*\*([\s\S]*?)\*\*/gu)]
    .map((match) => match[1]!.trim().split(/\s+/u).length);

describe("Bengali B1 timed mock pair", () => {
  it("owns two complete parallel forms, their keys, and a shared rubric", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) {
      expect(existsSync(join(root, name))).toBe(true);
    }

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("Reading — 45 minutes");
      expect(paper).toContain("Listening — 35 minutes");
      expect(paper).toContain("Writing — 45 minutes");
      expect(paper).toContain("Speaking — 15 minutes");
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(28);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(25);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(2);
      expect(itemCount(paper, "## Speaking")).toBe(11);
      expect(paper).toContain("দশ মিনিট");
      expect(paper).toContain("৫০–৭০টি বাংলা শব্দ");
      expect(paper).toContain("১৩০–১৭০টি বাংলা শব্দ");
      expect(paper).toMatch(/[\u0980-\u09FF]/u);
      expect(paper).not.toMatch(/Hindi|Devanagari|Malayalam|Interview:|Follow-up:|Prepared account:/u);

      const readingCounts = boldBlockWordCounts(section(paper, "## Reading", "## Listening"));
      const readingBands = [[220, 280], [260, 330], [280, 360], [340, 430]] as const;
      expect(readingCounts).toHaveLength(4);
      for (const [index, [minimum, maximum]] of readingBands.entries()) {
        expect(readingCounts[index]).toBeGreaterThanOrEqual(minimum);
        expect(readingCounts[index]).toBeLessThanOrEqual(maximum);
      }

      const listeningCounts = boldBlockWordCounts(section(paper, "## Listening", "## Writing"));
      const durationBands = [[120, 180], [180, 240], [210, 300], [240, 330]] as const;
      expect(listeningCounts).toHaveLength(4);
      for (const [index, [minimumSeconds, maximumSeconds]] of durationBands.entries()) {
        const count = listeningCounts[index]!;
        expect(count).toBeGreaterThanOrEqual(Math.ceil(minimumSeconds * 150 / 60));
        expect(count).toBeLessThanOrEqual(Math.floor(maximumSeconds * 130 / 60));
      }
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("pins independent B1 production and all four keyed skill sections", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("every\npaper must independently reach 60/100");
    expect(rubric).toContain("50–70 word functional text");
    expect(rubric).toContain("130–170\nword connected text");
    expect(rubric).toContain("matra line, hasanta");
    expect(rubric).toContain("non-Bengali-script");
    expect(rubric).toContain("transaction and interview twice");
    expect(rubric).toContain("narrative and short talk once");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).toContain("## Reading");
      expect(key).toContain("## Listening");
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("transaction and interview twice");
      expect(key).toContain("narrative and short talk once");
      expect(key).not.toMatch(/Hindi|Devanagari|Malayalam/u);
    }
  });
});
