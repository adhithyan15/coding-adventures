import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "a1");
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

describe("Sanskrit A1 timed mock source pair", () => {
  it("owns two separated learner, examiner, key, and rubric bundles", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-examiner-script.md",
      "mock-2-examiner-script.md", "mock-1-answer-key.md",
      "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name)), name).toBe(true);

    for (const form of ["mock-1", "mock-2"]) {
      const paper = read(`${form}.md`);
      const examiner = read(`${form}-examiner-script.md`);
      for (const heading of [
        "## Reading — 20 minutes", "## Listening — 18 minutes",
        "## Writing — 20 minutes", "## Speaking — 10 minutes",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain(`${form}-examiner-script.md`);
      expect(paper).toContain("five minutes of preparation");
      expect(paper).not.toMatch(/Malayalam/u);

      const reading = between(paper, "## Reading", "## Listening");
      expect(numbered(reading)).toBe(20);
      for (const [part, next, count, min, max] of [
        ["Signs and forms", "Short messages", 7, 70, 90],
        ["Short messages", "Personal descriptions", 7, 90, 120],
        ["Personal descriptions", undefined, 6, 90, 140],
      ] as const) {
        const body = between(reading, `### Part ${part === "Signs and forms" ? "1" : part === "Short messages" ? "2" : "3"} — ${part}`, next === undefined ? undefined : `### Part ${next === "Short messages" ? "2" : "3"} — ${next}`);
        expect(numbered(body), part).toBe(count);
        const source = between(body, "#### Source", "#### Questions");
        expect(devanagariWords(source), `${part} minimum`).toBeGreaterThanOrEqual(min);
        expect(devanagariWords(source), `${part} maximum`).toBeLessThanOrEqual(max);
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(17);
      expect(listening).not.toMatch(/[\u0900-\u097F]/u);
      const speaking = between(paper, "## Speaking");
      expect(numbered(speaking)).toBe(3);
      expect(speaking).not.toMatch(/[\u0900-\u097F]/u);

      expect(examiner).toContain("Do not give this file to the learner");
      const normalizedExaminer = examiner.replace(/\s+/gu, " ");
      expect(normalizedExaminer).toContain("trained speaker of the living spoken register");
      expect(normalizedExaminer).toContain("90–110 words per minute");
      expect(normalizedExaminer.toLowerCase()).toContain("play every named file twice");
      const recording = between(examiner, "## Listening recording master", "## Speaking");
      for (const index of Array.from({ length: 17 }, (_, item) => item + 1)) {
        const marker = `^${index}\\. ` + "`" + `${form}-l${String(index).padStart(2, "0")}` + "`";
        expect(recording).toMatch(new RegExp(marker, "mu"));
      }
      expect(recording).toMatch(/[\u0900-\u097F]/u);
      for (const [part, next, min, max] of [
        ["announcements", "short exchanges", 110, 135],
        ["short exchanges", "personal account", 165, 180],
        ["personal account", undefined, 220, 270],
      ] as const) {
        const script = between(
          recording,
          `### Part ${part === "announcements" ? "1" : part === "short exchanges" ? "2" : "3"} — ${part}`,
          next === undefined ? undefined : `### Part ${next === "short exchanges" ? "2" : "3"} — ${next}`,
        );
        expect(devanagariWords(script), `${part} minimum`).toBeGreaterThanOrEqual(min);
        expect(devanagariWords(script), `${part} maximum`).toBeLessThanOrEqual(max);
      }
      const interview = between(examiner, "### Part 1 — personal interview", "### Part 2");
      expect(interview.match(/^\d+\./gmu)).toHaveLength(5);
      expect(examiner).toContain("Record the speaking paper for independent scoring");
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    expect(read("mock-1.md")).toContain(
      "6. What should students bring? A one book / B one fruit / C one pen",
    );
  });

  it("keys every objective item and keeps readiness evidence unclaimed", () => {
    const rubric = read("rubric.md");
    const normalizedRubric = rubric.replace(/\s+/gu, " ");
    expect(rubric).toMatch(/60\/100 on (?:\*\*)?every(?:\*\*)? paper/u);
    expect(rubric).toContain("30–40 word message");
    expect(rubric).toContain("continuous śirorekhā");
    expect(rubric).toContain("external sandhi");
    expect(rubric).toContain("qualified Sanskrit recordings");
    expect(normalizedRubric).toContain("independent human review");
    expect(normalizedRubric).toContain("standard setting");
    expect(normalizedRubric).toContain("book-only validation");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toMatch(/\[`?rubric\.md`?\]\(rubric\.md\)/u);
      expect(key).toContain("examiner-script.md)");
      for (const [skill, next, count] of [
        ["Reading", "Listening", 20], ["Listening", "Writing", 17],
      ] as const) {
        const answers = between(key, `## ${skill}`, `## ${next}`).match(/\b\d{1,2} [ABC](?=;|\.)/gu) ?? [];
        expect(answers.map((answer) => Number.parseInt(answer, 10))).toEqual(
          Array.from({ length: count }, (_, index) => index + 1),
        );
      }
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("pronunciation tradition");
    }
  });
});
