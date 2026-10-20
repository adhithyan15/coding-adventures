import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "b1");
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

describe("Sanskrit B1 timed mock source pair", () => {
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
        "## Reading — 45 minutes", "## Listening — 35 minutes",
        "## Writing — 45 minutes", "## Speaking — 15 minutes",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain(`${form}-examiner-script.md`);
      expect(paper).toContain("ten minutes of preparation");
      expect(paper).not.toMatch(/Hindi|Bengali|Malayalam/u);

      const reading = between(paper, "## Reading", "## Listening");
      expect(numbered(reading)).toBe(28);
      for (const [heading, next, count, minimum, maximum] of [
        ["### Part 1 — correspondence", "### Part 2", 7, 220, 280],
        ["### Part 2 — public information", "### Part 3", 7, 260, 330],
        ["### Part 3 — narrative", "### Part 4", 7, 280, 360],
        ["### Part 4 — informational article", "## Listening", 7, 340, 430],
      ] as const) {
        const part = between(paper, heading, next);
        expect(numbered(part), heading).toBe(count);
        const source = between(part, "#### Source", "#### Questions");
        expect(devanagariWords(source), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(source), `${heading} maximum`).toBeLessThanOrEqual(maximum);
        expect(source).toContain("ऽ");
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(25);
      expect(listening).not.toMatch(/[\u0900-\u097F]/u);
      const writing = between(paper, "## Writing", "## Speaking");
      expect(numbered(writing)).toBe(2);
      expect(writing).toContain("50–70 words");
      expect(writing).toContain("130–170 words");
      expect(writing).toContain("apply external sandhi");
      expect(writing).toContain("copyable answer model");
      expect(writing).not.toMatch(/[\u0900-\u097F]/u);
      const speaking = between(paper, "## Speaking");
      expect(numbered(speaking)).toBe(11);
      expect(speaking).not.toMatch(/[\u0900-\u097F]/u);

      expect(examiner).toContain("Do not give this file to the learner");
      const normalizedExaminer = examiner.replace(/\s+/gu, " ");
      expect(normalizedExaminer).toContain("trained speaker of the living spoken register");
      expect(normalizedExaminer).toContain("130–150 words per minute");
      expect(normalizedExaminer.toLowerCase()).toContain("play parts 1 and 2 twice");
      expect(normalizedExaminer.toLowerCase()).toContain("play parts 3 and 4 once");
      const recording = between(examiner, "## Listening recording master", "## Speaking");
      for (const [heading, next, items, minimum, maximum] of [
        ["### Part 1 — transactions", "### Part 2", 7, 300, 390],
        ["### Part 2 — interview", "### Part 3", 6, 450, 520],
        ["### Part 3 — narrative", "### Part 4", 6, 525, 650],
        ["### Part 4 — short talk", "## Speaking", 6, 600, 715],
      ] as const) {
        const script = between(examiner, heading, next);
        expect(numbered(script), heading).toBe(items);
        expect(devanagariWords(script), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(script), `${heading} maximum`).toBeLessThanOrEqual(maximum);
      }
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
        ["Reading", "Listening", 28], ["Listening", "Writing", 25],
      ] as const) {
        const answers = between(key, `## ${skill}`, `## ${next}`).match(/\b\d{1,2} [ABC](?=;|\.)/gu) ?? [];
        expect(answers.map((answer) => Number.parseInt(answer, 10))).toEqual(
          Array.from({ length: count }, (_, index) => index + 1),
        );
        const labels = answers.map((answer) => answer.at(-1));
        for (const label of ["A", "B", "C"]) {
          expect(labels.filter((candidate) => candidate === label).length, `${name} ${skill} ${label}`)
            .toBeGreaterThanOrEqual(4);
        }
      }
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("pronunciation tradition");
      expect(key).toContain("parts 1 and 2 twice");
      expect(key).toContain("parts 3 and 4 once");
    }

    const semanticRegressions = [
      {
        form: "mock-1",
        question: "6. What may the visitor bring inside? A food / B a sealed water bottle / C a large bag",
        evidence: "पिहितं जलपात्रं नेतुं शक्यते",
        item: 6,
        answer: "B",
      },
      {
        form: "mock-1",
        question: "17. Why did the group stop? A heavy rain / B engine trouble / C a festival procession",
        evidence: "उत्सवयात्रा अपेक्षितसमयात्",
        item: 17,
        answer: "C",
      },
      {
        form: "mock-1",
        question: "18. What changed the speaker's attitude? A missing the event / B solving problems together / C receiving money",
        evidence: "सहयोगेन स एव पूर्वाभ्यासस्य अवसरः जातः",
        item: 18,
        answer: "B",
      },
      {
        form: "mock-2",
        question: "16. Who suggested the new approach? A a child / B the organiser / C another speaker",
        evidence: "एका बालिका पृष्टवती",
        item: 16,
        answer: "A",
      },
    ] as const;
    for (const regression of semanticRegressions) {
      expect(read(`${regression.form}.md`)).toContain(regression.question);
      expect(read(`${regression.form}-examiner-script.md`)).toContain(regression.evidence);
      const listeningKey = between(read(`${regression.form}-answer-key.md`), "## Listening", "## Writing");
      expect(listeningKey).toMatch(
        new RegExp(`\\b${regression.item} ${regression.answer}(?=;|\\.)`, "u"),
      );
    }
  });

  it("pins Sanskrit-specific scoring and keeps readiness evidence unclaimed", () => {
    const rubric = read("rubric.md");
    const normalized = rubric.replace(/\s+/gu, " ");
    expect(rubric).toMatch(/60\/100 on (?:\*\*)?every(?:\*\*)? paper/u);
    expect(rubric).toContain("50–70 word functional text");
    expect(rubric).toContain("130–170 word connected text");
    expect(rubric).toContain("external sandhi");
    expect(normalized).toContain("word-final visarga or anusvara");
    expect(normalized).toContain("continuous śirorekhā");
    expect(normalized).toContain("documented pronunciation tradition");
    expect(normalized).toContain("Vedic accent and morphology remain outside");
    expect(normalized).toContain("qualified Sanskrit recordings");
    expect(normalized).toContain("independent human review");
    expect(normalized).toContain("standard setting");
    expect(normalized).toContain("book-only validation");
    expect(normalized).toContain("`assessment.json` is active as a source-artifact contract");
  });
});
