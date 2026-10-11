import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "bengali", "mocks", "c1");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const section = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end);
};
const itemCount = (paper: string, heading: string, nextHeading?: string) =>
  section(paper, heading, nextHeading).match(/^\d+\./gmu)?.length ?? 0;

const partStimuli = (paper: string, headings: readonly string[], finalHeading: string) =>
  headings.map((heading, index) => {
    const nextHeading = headings[index + 1] ?? finalHeading;
    let body = section(paper, heading, nextHeading).replace(/^.*\n/u, "");
    body = body.replace(/^(?:Orienting preview|অস্কোরকৃত পরিচিতি):[\s\S]*?\n\n/u, "");
    body = body.split(/^\d+\./mu)[0]!;
    return body.replace(/^#{1,6} .*$/gmu, "").trim();
  });
const wordCount = (text: string) => text.split(/\s+/u).filter(Boolean).length;

const readingHeadings = [
  "### Dense public text", "### Professional text", "### Academic text", "### Literary text",
] as const;
const listeningHeadings = [
  "### Unscripted discussion", "### Lecture", "### Interview", "### Narrative",
] as const;

describe("Bengali C1 timed mock pair", () => {
  it("owns two independent complete forms at every C1 input band", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name))).toBe(true);

    const formStimuli: string[][] = [];
    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("Reading — 75 minutes");
      expect(paper).toContain("Listening — 50 minutes");
      expect(paper).toContain("Writing — 75 minutes");
      expect(paper).toContain("Speaking — 22 minutes");
      expect(itemCount(paper, "## Reading", "## Listening")).toBe(36);
      expect(itemCount(paper, "## Listening", "## Writing")).toBe(28);
      expect(itemCount(paper, "## Writing", "## Speaking")).toBe(2);
      expect(itemCount(paper, "## Speaking")).toBe(8);
      expect(paper).toContain("15 minutes preparation");
      expect(paper).toContain("160–185 words per minute");
      expect(paper).toContain("180–220 words");
      expect(paper).toContain("300–380 words");
      expect(paper).toContain("270–300 seconds");
      expect(paper).toContain("300–360 seconds");
      expect(paper).toMatch(/[\u0980-\u09FF]/u);
      expect(paper).not.toMatch(/[\u0904-\u0939]|Hindi|Devanagari|Malayalam/u);

      const receptiveQuestions = section(paper, "## Reading", "## Writing")
        .match(/^\d+\..*$/gmu) ?? [];
      expect(receptiveQuestions).toHaveLength(64);
      for (const question of receptiveQuestions) {
        expect(question).toMatch(/[\u0980-\u09FF]/u);
        expect(question).toMatch(/^\d+\..* A .* \/ B .* \/ C /u);
      }

      const readings = partStimuli(paper, readingHeadings, "## Listening");
      const readingBands = [[650, 800], [700, 850], [700, 880], [750, 970]] as const;
      readings.forEach((stimulus, index) => {
        expect(wordCount(stimulus)).toBeGreaterThanOrEqual(readingBands[index]![0]);
        expect(wordCount(stimulus)).toBeLessThanOrEqual(readingBands[index]![1]);
      });

      const listenings = partStimuli(paper, listeningHeadings, "## Writing");
      const durationBands = [[300, 420], [360, 480], [330, 450], [330, 450]] as const;
      listenings.forEach((stimulus, index) => {
        const [minimumSeconds, maximumSeconds] = durationBands[index]!;
        expect(wordCount(stimulus)).toBeGreaterThanOrEqual(Math.ceil(minimumSeconds * 185 / 60));
        expect(wordCount(stimulus)).toBeLessThanOrEqual(Math.floor(maximumSeconds * 160 / 60));
      });

      const synthesisWords = wordCount(section(paper, "### Source synthesis", "### Extended response")
        .replace(/^#{1,6} .*$/gmu, ""));
      expect(synthesisWords).toBeGreaterThanOrEqual(450);
      expect(synthesisWords).toBeLessThanOrEqual(650);
      const extendedBriefWords = wordCount(section(paper, "### Extended response", "## Speaking")
        .replace(/^#{1,6} .*$/gmu, ""));
      expect(extendedBriefWords).toBeGreaterThanOrEqual(90);
      expect(extendedBriefWords).toBeLessThanOrEqual(150);
      const presentationSourceWords = wordCount(section(
        paper, "### Five-minute source-based presentation", "### Collaborative synthesis",
      ).replace(/^#{1,6} .*$/gmu, ""));
      expect(presentationSourceWords).toBeGreaterThanOrEqual(350);
      expect(presentationSourceWords).toBeLessThanOrEqual(500);
      const collaborativeBriefWords = wordCount(section(
        paper, "### Collaborative synthesis", "### Challenge and defence",
      ).replace(/^#{1,6} .*$/gmu, ""));
      expect(collaborativeBriefWords).toBeGreaterThanOrEqual(60);
      expect(collaborativeBriefWords).toBeLessThanOrEqual(100);
      expect(paper).toContain("২০–৭৫ সেকেন্ড");
      formStimuli.push([...readings, ...listenings]);
    }

    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    for (const stimulus of formStimuli[0]!) expect(formStimuli[1]).not.toContain(stimulus);
  });

  it("pins single delivery, regional voices, independent scoring, and readiness limits", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("every paper must independently reach 60/100");
    expect(rubric).toContain("180–220 word source synthesis");
    expect(rubric).toContain("300–380 word extended response");
    expect(rubric).toContain("matra line");
    expect(rubric).toContain("hasanta");
    expect(rubric).toContain("vowel signs");
    expect(rubric).toContain("non-Bengali-script");
    expect(rubric).toContain("not a proven C1 pass");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).toContain("once after one unscored orienting preview");
      expect(key).toContain("Rarh-region");
      expect(key).toContain("Varendra-region");
      expect(key).toMatch(/speaker\s+provenance/u);
      expect(key).toContain("alternatives used consistently receive full credit");
      expect(section(key, "## Reading", "## Listening").match(/\d+\s+[ABC]/gu)).toHaveLength(36);
      expect(section(key, "## Listening", "## Writing").match(/\d+\s+[ABC]/gu)).toHaveLength(28);
      expect(key).not.toMatch(/[\u0904-\u0939]|Hindi|Devanagari|Malayalam/u);
    }
  });
});
