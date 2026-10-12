import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "a2");
const read = (name: string) => readFileSync(join(root, name), "utf8");

function between(text: string, start: string, end?: string): string {
  const from = text.indexOf(start);
  expect(from, start).toBeGreaterThanOrEqual(0);
  const to = end === undefined ? text.length : text.indexOf(end, from + start.length);
  expect(to, end).toBeGreaterThan(from);
  return text.slice(from, to);
}

const numbered = (text: string) => text.match(/^\d+\./gmu)?.length ?? 0;
const devanagariWords = (text: string) => text.match(/[\u0900-\u097F]+/gu)?.length ?? 0;

describe("Sanskrit A2 timed mock source pair", () => {
  it("owns two independent learner, confidential, key, and rubric bundles", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-examiner-script.md",
      "mock-2-examiner-script.md", "mock-1-answer-key.md",
      "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name)), name).toBe(true);

    for (const form of ["mock-1", "mock-2"]) {
      const paper = read(`${form}.md`);
      const examiner = read(`${form}-examiner-script.md`);

      for (const heading of [
        "## Reading — 30 minutes", "## Listening — 25 minutes",
        "## Writing — 30 minutes", "## Speaking — 12 minutes",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain(`${form}-examiner-script.md`);
      expect(paper).toContain("five minutes of preparation");
      expect(paper).not.toMatch(/Hindi|Bengali|Malayalam/u);

      const reading = between(paper, "## Reading", "## Listening");
      expect(numbered(reading)).toBe(24);
      for (const [heading, next, count, minimum, maximum] of [
        ["### Part 1 — notices and correspondence", "### Part 2", 8, 150, 220],
        ["### Part 2 — connected everyday account", "### Part 3", 8, 180, 260],
        ["### Part 3 — connected public-information article", "## Listening", 8, 220, 270],
      ] as const) {
        const part = between(paper, heading, next);
        expect(numbered(part), heading).toBe(count);
        const source = between(part, "#### Source", "#### Questions");
        expect(devanagariWords(source), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(source), `${heading} maximum`).toBeLessThanOrEqual(maximum);
        expect(source).toContain("ऽ");
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(20);
      expect(listening).not.toMatch(/[\u0900-\u097F]/u);
      const writing = between(paper, "## Writing", "## Speaking");
      expect(numbered(writing)).toBe(2);
      expect(writing).toContain("25–35 words");
      expect(writing).toContain("70–90 words");
      expect(writing).toContain("apply external sandhi");
      expect(writing).toContain("copyable answer model");
      expect(writing).not.toMatch(/[\u0900-\u097F]/u);
      const speaking = between(paper, "## Speaking");
      expect(numbered(speaking)).toBe(3);
      expect(speaking).not.toMatch(/[\u0900-\u097F]/u);

      expect(examiner).toContain("Do not give this file to the learner");
      const normalizedExaminer = examiner.replace(/\s+/gu, " ");
      expect(normalizedExaminer).toContain("trained speaker of the living spoken register");
      expect(normalizedExaminer).toContain("110–130 words per minute");
      expect(normalizedExaminer.toLowerCase()).toContain("play every named file twice");
      const recording = between(examiner, "## Listening recording master", "## Speaking");
      for (const index of Array.from({ length: 14 }, (_, item) => item + 1)) {
        const marker = `^${index}\\. ` + "`" + `${form}-l${String(index).padStart(2, "0")}` + "`";
        expect(recording).toMatch(new RegExp(marker, "mu"));
      }
      expect(recording).toContain(`${form}-l15-to-l20`);
      for (const [heading, next, minimum, maximum] of [
        ["### Part 1 — announcements", "### Part 2", 195, 275],
        ["### Part 2 — everyday conversations", "### Part 3", 260, 330],
        ["### Part 3 — connected account", "## Speaking", 260, 330],
      ] as const) {
        const script = between(examiner, heading, next);
        expect(devanagariWords(script), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(script), `${heading} maximum`).toBeLessThanOrEqual(maximum);
      }
      const interview = between(examiner, "### Part 1 — personal interview", "### Part 2");
      expect(interview.match(/^\d+\./gmu)).toHaveLength(6);
      expect(normalizedExaminer.toLowerCase()).toContain(
        "record the complete speaking paper for independent scoring",
      );
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("keys every objective item without a one-label answer pattern", () => {
    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toMatch(/\[`?rubric\.md`?\]\(rubric\.md\)/u);
      expect(key).toContain("examiner-script.md)");
      for (const [skill, next, count] of [
        ["Reading", "Listening", 24], ["Listening", "Writing", 20],
      ] as const) {
        const answers = between(key, `## ${skill}`, `## ${next}`).match(/\b\d{1,2} [ABC](?=;|\.)/gu) ?? [];
        expect(answers.map((answer) => Number.parseInt(answer, 10))).toEqual(
          Array.from({ length: count }, (_, index) => index + 1),
        );
        const labels = answers.map((answer) => answer.at(-1));
        for (const label of ["A", "B", "C"]) {
          expect(labels.filter((candidate) => candidate === label).length, `${name} ${skill} ${label}`)
            .toBeGreaterThanOrEqual(3);
        }
      }
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("pronunciation tradition");
    }

    const paper = read("mock-2.md");
    const examiner = read("mock-2-examiner-script.md");
    const key = read("mock-2-answer-key.md");
    expect(paper).toContain(
      "14. When is the summary needed? A tomorrow / B evening after meeting / C three today",
    );
    expect(examiner).toContain("सारांशं त्रिवादनात् पूर्वं प्रेषयतु");
    expect(between(key, "## Listening", "## Writing")).toMatch(/\b14 C(?=;|\.)/u);
  });

  it("pins Sanskrit-specific scoring and keeps readiness evidence unclaimed", () => {
    const rubric = read("rubric.md");
    const normalized = rubric.replace(/\s+/gu, " ");
    expect(rubric).toMatch(/60\/100 on (?:\*\*)?every(?:\*\*)? paper/u);
    expect(rubric).toContain("25–35 word functional response");
    expect(rubric).toContain("70–90 word connected text");
    expect(rubric).toContain("external sandhi");
    expect(normalized).toContain("word-final visarga or anusvara");
    expect(normalized).toContain("continuous śirorekhā");
    expect(normalized).toContain("qualified Sanskrit recordings");
    expect(normalized).toContain("independent human review");
    expect(normalized).toContain("standard setting");
    expect(normalized).toContain("book-only validation");
    expect(normalized).toContain("`assessment.json` remains absent");
  });
});
