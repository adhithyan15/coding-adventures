import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "pre-a1");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const section = (paper: string, heading: string, nextHeading?: string) => {
  const start = paper.indexOf(heading);
  const end = nextHeading === undefined ? paper.length : paper.indexOf(nextHeading, start + heading.length);
  return paper.slice(start, end);
};
const itemCount = (text: string) => text.match(/^\d+\./gmu)?.length ?? 0;
const keyedCount = (text: string) => text.match(/\b\d+ [ABC]\b/gu)?.length ?? 0;

describe("Sanskrit pre-A1 timed mock source pair", () => {
  it("owns two separated learner, examiner, key, and rubric bundles", () => {
    const names = [
      "mock-1.md",
      "mock-2.md",
      "mock-1-examiner-script.md",
      "mock-2-examiner-script.md",
      "mock-1-answer-key.md",
      "mock-2-answer-key.md",
      "rubric.md",
    ];
    expect(names.every((name) => existsSync(join(root, name)))).toBe(true);

    for (const form of ["mock-1", "mock-2"]) {
      const paper = read(`${form}.md`);
      const examiner = read(`${form}-examiner-script.md`);
      const listeningPaper = section(paper, "## Listening", "## Writing");
      const speakingPaper = section(paper, "## Speaking");
      const listeningScript = section(examiner, "## Listening recording master", "## Writing prompt master");
      const writingScript = section(examiner, "## Writing prompt master", "## Speaking");
      const speakingScript = section(examiner, "## Speaking");

      expect(paper).toContain("## Reading — 10 minutes");
      expect(paper).toContain("## Listening — 10 minutes");
      expect(paper).toContain("## Writing — 12 minutes");
      expect(paper).toContain("## Speaking — 8 minutes");
      expect(paper).toMatch(/[\u0900-\u097F]/u);
      expect(paper).not.toMatch(/Hindi|Bengali|Malayalam/u);
      expect(itemCount(section(paper, "## Reading", "## Listening"))).toBe(14);
      expect(itemCount(listeningPaper)).toBe(14);
      expect(itemCount(section(paper, "## Writing", "## Speaking"))).toBe(8);

      expect(listeningPaper).not.toMatch(/[\u0900-\u097F]/u);
      expect(speakingPaper).not.toMatch(/[\u0900-\u097F]/u);
      expect(paper).toContain(`${form}-examiner-script.md`);

      expect(examiner).toContain("Do not give this file to the learner");
      expect(examiner).toContain("a trained speaker");
      expect(examiner).toContain("70–90 words per minute");
      expect(examiner).toContain("play every named file twice");
      expect(examiner).toContain("eight-second");
      expect(itemCount(section(listeningScript, "### Part 1", "### Part 2"))).toBe(6);
      expect(itemCount(section(listeningScript, "### Part 2", "### Part 3"))).toBe(4);
      expect(itemCount(section(listeningScript, "### Part 3"))).toBe(4);
      expect(itemCount(writingScript)).toBe(6);
      expect(itemCount(speakingScript)).toBe(6);
      expect(examiner.match(new RegExp(`${form}-l\\d{2}`, "gu"))?.length).toBe(14);
      expect(examiner.match(new RegExp(`${form}-w0[3-6]`, "gu"))?.length).toBe(4);

      const hiddenWritingTargets = [...writingScript.matchAll(/\*\*([^*]+)\*\*/gu)].map((match) => match[1]);
      expect(hiddenWritingTargets).toHaveLength(6);
      for (const target of hiddenWritingTargets) {
        expect(paper).not.toContain(target);
      }
    }

    expect(read("mock-1.md")).toContain("दिल्ली");
    expect(read("mock-1.md")).not.toContain("उज्जयिनी");
    expect(read("mock-2.md")).toContain("उज्जयिनी");
    expect(read("mock-2.md")).not.toContain("दिल्ली");
  });

  it("pins complete keys, independent passes, and Sanskrit-specific evidence", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("60/100 on **every** paper");
    expect(rubric).toContain("continuous śirorekhā");
    expect(rubric).toContain("correct absence of visarga");
    expect(rubric.replace(/\s+/g, " ")).toContain("external sandhi");
    expect(rubric).toContain("qualified Sanskrit review");
    expect(rubric).toContain("book-only human validation");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("(rubric.md)");
      expect(key).toContain("examiner-script.md)");
      expect(key).toContain("## Reading");
      expect(key).toContain("## Listening");
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(keyedCount(section(key, "## Reading", "## Listening"))).toBe(14);
      expect(keyedCount(section(key, "## Listening", "## Writing"))).toBe(14);
      expect(key).toContain("pronunciation variants");
      expect(key).not.toMatch(/Hindi|Bengali|Malayalam/u);
    }
  });
});
