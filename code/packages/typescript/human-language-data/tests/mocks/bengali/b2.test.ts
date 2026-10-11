import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "bengali", "mocks", "b2");
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
const boldBlocks = (text: string) =>
  [...text.matchAll(/\*\*([\s\S]*?)\*\*/gu)].map((match) => match[1]!.trim());

describe("Bengali B2 timed mock pair", () => {
  it("owns two complete parallel forms at the declared B2 bands", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) {
      expect(existsSync(join(root, name))).toBe(true);
    }

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
      expect(paper).toContain("দশ মিনিট");
      expect(paper).toContain("১০০–১৩০টি বাংলা শব্দ");
      expect(paper).toContain("২২০–২৮০টি বাংলা শব্দ");
      expect(paper).toContain("২১০–২৪০ সেকেন্ড");
      expect(paper).toContain("২৪০–৩০০ সেকেন্ড");
      expect(paper).toMatch(/[\u0980-\u09FF]/u);
      expect(paper).not.toMatch(/Hindi|Devanagari|Malayalam|Presentation:|Defended|defended|discussion|worn borders/u);

      const readingCounts = boldBlockWordCounts(section(paper, "## Reading", "## Listening"));
      const readingBands = [[400, 520], [420, 560], [450, 580], [530, 640]] as const;
      expect(readingCounts).toHaveLength(4);
      for (const [index, [minimum, maximum]] of readingBands.entries()) {
        expect(readingCounts[index]).toBeGreaterThanOrEqual(minimum);
        expect(readingCounts[index]).toBeLessThanOrEqual(maximum);
      }

      const listeningCounts = boldBlockWordCounts(section(paper, "## Listening", "## Writing"));
      const durationBands = [[210, 300], [240, 330], [270, 360], [270, 390]] as const;
      expect(listeningCounts).toHaveLength(4);
      for (const [index, [minimumSeconds, maximumSeconds]] of durationBands.entries()) {
        const count = listeningCounts[index]!;
        expect(count).toBeGreaterThanOrEqual(Math.ceil(minimumSeconds * 170 / 60));
        expect(count).toBeLessThanOrEqual(Math.floor(maximumSeconds * 150 / 60));
      }
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    const form1Blocks = boldBlocks(read("mock-1.md").slice(0, read("mock-1.md").indexOf("## Writing")));
    const form2Blocks = boldBlocks(read("mock-2.md").slice(0, read("mock-2.md").indexOf("## Writing")));
    expect(form1Blocks).toHaveLength(8);
    expect(form2Blocks).toHaveLength(8);
    for (const block of form1Blocks) expect(form2Blocks).not.toContain(block);
  });

  it("pins independent scoring, single delivery, and two regional voices", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("every\npaper must independently reach 60/100");
    expect(rubric).toContain("100–130 word interaction text");
    expect(rubric).toContain("220–280\nword extended text");
    expect(rubric).toContain("matra line, hasanta");
    expect(rubric).toContain("non-Bengali-script");
    expect(rubric).toContain("once after one unscored orienting\npreview");
    expect(rubric).toContain("150–170 words per minute");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).toContain("## Reading");
      expect(key).toContain("## Listening");
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("once after one\nunscored orienting preview");
      expect(key).toContain("Rarh region");
      expect(key).toContain("Varendra region");
      expect(key).toContain("speaker\nprovenance and reviewer confirmation");
      expect(key).not.toMatch(/Hindi|Devanagari|Malayalam/u);
      expect(section(key, "## Reading", "## Listening").match(/\d+\s+[ABC]/gu)).toHaveLength(32);
      expect(section(key, "## Listening", "## Writing").match(/\d+\s+[ABC]/gu)).toHaveLength(28);
    }
  });
});
