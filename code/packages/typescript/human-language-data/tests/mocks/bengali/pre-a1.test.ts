import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "bengali", "mocks", "pre-a1");
const read = (name: string) => readFileSync(join(root, name), "utf8");

function between(text: string, startHeading: string, endHeading?: string): string {
  const start = text.indexOf(startHeading);
  expect(start, startHeading).toBeGreaterThanOrEqual(0);
  const end = endHeading === undefined ? text.length : text.indexOf(endHeading, start + startHeading.length);
  expect(end, endHeading).toBeGreaterThan(start);
  return text.slice(start, end);
}

const itemCount = (section: string) => section.match(/^\d+\./gmu)?.length ?? 0;

describe("Bengali pre-A1 timed mock pair", () => {
  it("has distinct complete papers with every task-shape part", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name)), name).toBe(true);

    for (const name of ["mock-1.md", "mock-2.md"]) {
      const paper = read(name);
      expect(paper).toContain("## Reading — 10 minutes");
      expect(paper).toContain("## Listening — 10 minutes");
      expect(paper).toContain("## Writing — 12 minutes");
      expect(paper).toContain("## Speaking — 8 minutes");
      expect(paper).toMatch(/[\u0980-\u09FF]/u);
      expect(paper).not.toMatch(/romanized Bengali|Devanagari|Hindi/u);

      for (const skill of ["Reading", "Listening"] as const) {
        const next = skill === "Reading" ? "## Listening" : "## Writing";
        const section = between(paper, `## ${skill}`, next);
        expect(itemCount(section)).toBe(14);
        const parts = skill === "Reading"
          ? ["Signs and words", "Phrases and notices", "Personal details"]
          : ["Sounds and words", "Greeting responses", "Personal details"];
        expect(itemCount(between(section, `### ${parts[0]}`, `### ${parts[1]}`))).toBe(6);
        expect(itemCount(between(section, `### ${parts[1]}`, `### ${parts[2]}`))).toBe(4);
        expect(itemCount(between(section, `### ${parts[2]}`))).toBe(4);
      }

      const writing = between(paper, "## Writing", "## Speaking");
      expect(itemCount(writing)).toBe(8);
      expect(itemCount(between(writing, "### Delayed recall", "### Dictation"))).toBe(2);
      expect(itemCount(between(writing, "### Dictation", "### Independent production"))).toBe(4);
      expect(itemCount(between(writing, "### Independent production"))).toBe(2);
      expect(itemCount(between(paper, "## Speaking"))).toBe(6);
      expect(paper).toContain("ten seconds");
      expect(paper).toMatch(/70–90\s+words per minute/u);
      expect(paper).toContain("twice");
      expect(paper).toContain("Print Reading, Listening, and Writing as separate skill sheets");
      expect(paper).toContain("Collect each completed sheet before giving the next");
      expect(paper).toContain("remove the Reading and Listening sheets before Writing");
      const examinerScript = between(paper, "## Examiner-only script and cards");
      for (let item = 1; item <= 14; item += 1) {
        expect(examinerScript).toMatch(new RegExp(`^- L${item}: \\*\\*[^\\n]+\\*\\*$`, "mu"));
      }
      for (const card of ["W1", "W2", "D3", "D4", "D5", "D6"]) {
        expect(examinerScript).toMatch(new RegExp(`^- ${card}: \\*\\*[^\\n]+\\*\\*$`, "mu"));
      }
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
  });

  it("keys every objective item and preserves independent Bengali-script scoring", () => {
    const rubric = read("rubric.md");
    expect(rubric).toContain("60/100 on every paper");
    expect(rubric).toContain("matra line");
    expect(rubric).toContain("vowel-sign side");
    expect(rubric).toContain("hasanta");
    expect(rubric).toContain("No book-only readiness claim");
    expect(rubric).toContain("Collect Reading and Listening before");

    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toContain("[rubric.md](rubric.md)");
      for (const [skill, next] of [["Reading", "Listening"], ["Listening", "Writing"]]) {
        const answers = between(key, `## ${skill}`, `## ${next}`).match(/\b\d{1,2} [ABC](?=;|\.)/gu) ?? [];
        expect(answers.map((answer) => Number.parseInt(answer, 10))).toEqual(
          Array.from({ length: 14 }, (_, index) => index + 1),
        );
      }
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
    }
  });
});
