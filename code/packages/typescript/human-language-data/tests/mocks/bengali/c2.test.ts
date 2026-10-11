import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "bengali", "mocks", "c2");
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
    body = body.replace(/^[^\n]*(?:preview|প্রিভিউ):[\s\S]*?\n\n/u, "");
    body = body.split(/^\d+\./mu)[0]!;
    return body.replace(/^#{1,6} .*$/gmu, "").trim();
  });
const wordCount = (text: string) => text.split(/\s+/u).filter(Boolean).length;

const readingHeadings = [
  "### Specialist argument", "### Public discourse", "### Specialist synthesis", "### Literary intertext",
] as const;
const listeningHeadings = [
  "### Rapid multiparty interaction", "### Extended argument", "### Stylistic narrative", "### Culturally dense media",
] as const;

describe("Bengali C2 timed mock pair", () => {
  it("owns two independent complete forms at every C2 input band", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name))).toBe(true);

    const formStimuli: string[][] = [];
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
      expect(paper).toContain("220–280 words");
      expect(paper).toContain("420–520 words");
      expect(paper).toContain("330–360 seconds");
      expect(paper).toContain("360–450 seconds");
      expect(paper).toContain("20–90 seconds");
      expect(paper).toContain("390–480 seconds");
      expect(paper).toMatch(/[\u0980-\u09FF]/u);
      expect(paper).not.toMatch(
        /[\u0904-\u0963\u0966-\u097F\u0A00-\u0A7F\u0A80-\u0AFF\u0B00-\u0B7F\u0B80-\u0BFF\u0C00-\u0C7F\u0C80-\u0CFF\u0D00-\u0D7F\u0D80-\u0DFF\u0400-\u04FF\u3400-\u9FFF]/u,
      );
      expect(paper).not.toMatch(/Hindi|Devanagari|Malayalam/u);

      const receptiveQuestions = section(paper, "## Reading", "## Writing")
        .match(/^\d+\..*$/gmu) ?? [];
      expect(receptiveQuestions).toHaveLength(72);
      for (const question of receptiveQuestions) {
        expect(question).toMatch(/^\d+\..* A .* \/ B .* \/ C /u);
        const stem = question.match(/^\d+\.\s*(.*?) A /u)?.[1] ?? "";
        expect(stem).toMatch(/[\u0980-\u09FF]/u);
        const options = question.match(/ A (.*) \/ B (.*) \/ C (.*)$/u)?.slice(1) ?? [];
        expect(options).toHaveLength(3);
        for (const option of options) expect(option).toMatch(/[\u0980-\u09FF]/u);
      }

      const readings = partStimuli(paper, readingHeadings, "## Listening");
      const readingBands = [[950, 1200], [1000, 1250], [1000, 1250], [1050, 1300]] as const;
      readings.forEach((stimulus, index) => {
        expect(wordCount(stimulus)).toBeGreaterThanOrEqual(readingBands[index]![0]);
        expect(wordCount(stimulus)).toBeLessThanOrEqual(readingBands[index]![1]);
      });

      const listenings = partStimuli(paper, listeningHeadings, "## Writing");
      const durationBands = [[360, 480], [420, 540], [420, 540], [420, 540]] as const;
      listenings.forEach((stimulus, index) => {
        const [minimumSeconds, maximumSeconds] = durationBands[index]!;
        expect(wordCount(stimulus)).toBeGreaterThanOrEqual(Math.ceil(minimumSeconds * 200 / 60));
        expect(wordCount(stimulus)).toBeLessThanOrEqual(Math.floor(maximumSeconds * 165 / 60));
      });

      const synthesisWords = wordCount(section(paper, "### Synthesis and reframing", "### Contrasting extended text")
        .replace(/^#{1,6} .*$/gmu, ""));
      expect(synthesisWords).toBeGreaterThanOrEqual(750);
      expect(synthesisWords).toBeLessThanOrEqual(1000);
      const extendedBriefWords = wordCount(section(paper, "### Contrasting extended text", "## Speaking")
        .replace(/^#{1,6} .*$/gmu, ""));
      expect(extendedBriefWords).toBeGreaterThanOrEqual(100);
      expect(extendedBriefWords).toBeLessThanOrEqual(180);
      const presentationSourceWords = wordCount(section(
        paper, "### Six-minute synthesis mediation", "### Multiparty negotiation",
      ).replace(/^#{1,6} .*$/gmu, ""));
      expect(presentationSourceWords).toBeGreaterThanOrEqual(550);
      expect(presentationSourceWords).toBeLessThanOrEqual(750);
      const collaborativeBriefWords = wordCount(section(
        paper, "### Multiparty negotiation", "### Defence and second-audience reformulation",
      ).replace(/^#{1,6} .*$/gmu, ""));
      expect(collaborativeBriefWords).toBeGreaterThanOrEqual(80);
      expect(collaborativeBriefWords).toBeLessThanOrEqual(130);
      formStimuli.push([...readings, ...listenings]);
    }

    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    for (const stimulus of formStimuli[0]!) expect(formStimuli[1]).not.toContain(stimulus);
  });

  it("pins single delivery, regional voices, independent scoring, and readiness limits", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("every paper must independently reach 60/100");
    expect(rubric).toContain("220–280 word synthesis/reframing");
    expect(rubric).toMatch(/420–520 word\s+contrasting-genre text/u);
    expect(rubric).toContain("matra line");
    expect(rubric).toContain("hasanta");
    expect(rubric).toContain("vowel signs");
    expect(rubric).toContain("non-Bengali-script");
    expect(rubric).toContain("not a proven C2 pass");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      expect(key).toContain("once after one unscored orienting preview");
      expect(key).toContain("Rarh-region");
      expect(key).toContain("Varendra-region");
      expect(key).toMatch(/speaker\s+provenance/u);
      expect(key).toContain("alternatives used consistently receive full credit");
      expect(section(key, "## Reading", "## Listening").match(/\d+\s+[ABC]/gu)).toHaveLength(40);
      expect(section(key, "## Listening", "## Writing").match(/\d+\s+[ABC]/gu)).toHaveLength(32);
      expect(key).not.toMatch(/[\u0904-\u0939]|Hindi|Devanagari|Malayalam/u);
    }
  });
});
