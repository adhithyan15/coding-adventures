import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "bengali", "mocks", "a1");
const read = (name: string) => readFileSync(join(root, name), "utf8");

function between(text: string, start: string, end?: string): string {
  const from = text.indexOf(start);
  expect(from, start).toBeGreaterThanOrEqual(0);
  const to = end === undefined ? text.length : text.indexOf(end, from + start.length);
  expect(to, end).toBeGreaterThan(from);
  return text.slice(from, to);
}

const numbered = (text: string) => text.match(/^\d+\./gmu)?.length ?? 0;
const bengaliWords = (text: string) => text.match(/[\u0980-\u09FF]+/gu)?.length ?? 0;

describe("Bengali A1 timed mock pair", () => {
  it("owns two distinct complete four-skill forms, keys, and a shared rubric", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name)), name).toBe(true);

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      for (const heading of [
        "## Reading — 20 minutes", "## Listening — 18 minutes",
        "## Writing — 20 minutes", "## Speaking — 10 minutes",
        "## Examiner-only listening scripts",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain("five minutes of preparation");
      expect(paper).toContain("Collect the Reading and Listening sheets before Writing");
      expect(paper).toMatch(/never hand the combined source file to\s+the learner/u);
      expect(paper).toMatch(/90–110 words per minute/u);
      expect(paper).toContain("twice");
      expect(paper).not.toMatch(/romanized Bengali|Devanagari|Hindi/u);

      const reading = between(paper, "## Reading", "## Listening");
      const readingParts = [
        ["Signs and forms", "Short messages", 7, 70, 90],
        ["Short messages", "Personal descriptions", 7, 90, 120],
        ["Personal descriptions", undefined, 6, 90, 140],
      ] as const;
      expect(numbered(reading)).toBe(20);
      for (const [part, next, count, min, max] of readingParts) {
        const section = between(reading, `### ${part}`, next === undefined ? undefined : `### ${next}`);
        expect(numbered(section), part).toBe(count);
        const source = between(section, "#### Source", "#### Questions");
        expect(bengaliWords(source), part).toBeGreaterThanOrEqual(min);
        expect(bengaliWords(source), part).toBeLessThanOrEqual(max);
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(17);
      for (const [part, next, count] of [
        ["Announcements", "Short exchanges", 6],
        ["Short exchanges", "Personal account", 6],
        ["Personal account", undefined, 5],
      ] as const) {
        const section = between(listening, `### ${part}`, next === undefined ? undefined : `### ${next}`);
        expect(numbered(section), part).toBe(count);
      }
      const examiner = between(paper, "## Examiner-only listening scripts");
      for (const [part, next, min, max] of [
        // Both the slowest and fastest permitted delivery must fit the time window.
        ["Announcements", "Short exchanges", 110, 135],
        ["Short exchanges", "Personal account", 165, 180],
        ["Personal account", undefined, 220, 270],
      ] as const) {
        const section = between(examiner, `### ${part}`, next === undefined ? undefined : `### ${next}`);
        const script = between(section, "#### Script");
        expect(bengaliWords(script), part).toBeGreaterThanOrEqual(min);
        expect(bengaliWords(script), part).toBeLessThanOrEqual(max);
      }
      for (let index = 1; index <= 17; index += 1) {
        expect(examiner).toMatch(new RegExp(`^- L${index}:`, "mu"));
      }

      const writing = between(paper, "## Writing", "## Speaking");
      expect(numbered(writing)).toBe(2);
      const form = between(writing, "1. Practical form", "2. Named-reader message");
      expect(form.match(/^   \| [^|]+ \| __________ \|$/gmu), "seven blank form fields").toHaveLength(7);
      for (const [task, next, min, max] of [
        ["1. Practical form", "2. Named-reader message", 40, 70],
        ["2. Named-reader message", undefined, 25, 50],
      ] as const) {
        const section = between(writing, task, next);
        const prompt = between(section, "**Bengali prompt:**");
        expect(bengaliWords(prompt), task).toBeGreaterThanOrEqual(min);
        expect(bengaliWords(prompt), task).toBeLessThanOrEqual(max);
      }
      const speaking = between(paper, "## Speaking", "## Examiner-only");
      expect(numbered(speaking)).toBe(3);
      const interview = between(speaking, "1. Personal interview", "2. Prepared description");
      expect(interview.match(/\*\*[^*]+\?\*\*/gu)?.length).toBe(5);
      const description = between(speaking, "2. Prepared description", "3. Transactional role-play");
      const card = between(description, "**Prompt card:**");
      expect(bengaliWords(card)).toBeGreaterThanOrEqual(10);
      expect(bengaliWords(card)).toBeLessThanOrEqual(30);
      const rolePlay = between(speaking, "3. Transactional role-play");
      const opening = between(rolePlay, "Examiner opens orally:");
      expect(bengaliWords(opening)).toBeGreaterThanOrEqual(17);
      expect(bengaliWords(opening)).toBeLessThanOrEqual(50);
      expect(paper).toContain("30–40 Bengali words");
      expect(paper).toMatch(/six to eight fields/iu);
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("keys every objective item and preserves independent script scoring", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("60/100 on every skill");
    expect(rubric).toContain("matra line");
    expect(rubric).toContain("vowel-sign side");
    expect(rubric).toContain("hasanta");
    expect(rubric).toContain("No book-only readiness claim");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
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
    }
  });
});
