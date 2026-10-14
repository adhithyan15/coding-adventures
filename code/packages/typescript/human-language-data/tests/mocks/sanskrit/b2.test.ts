import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "b2");
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
const renderedLearnerProse = (text: string) => text.split("\n")
  .filter((line) => !line.startsWith("#"))
  .join("\n")
  .replace(/\[`mock-[12]-examiner-script\.md`\]\(mock-[12]-examiner-script\.md\)/gu, "");

describe("Sanskrit B2 timed mock source pair", () => {
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
        "## Reading — 60 minutes", "## Listening — 45 minutes",
        "## Writing — 60 minutes", "## Speaking — 18 minutes",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain(`${form}-examiner-script.md`);
      const normalizedPaper = paper.replace(/\s+/gu, " ");
      expect(normalizedPaper).toMatch(/लेखनपरीक्षायाः पूर्वं .*पठनश्रवण/u);
      expect(normalizedPaper).toMatch(/दश निमेषाः (?:सज्जतायै|सिद्धतायै)/u);
      expect(paper.match(/उदाहरणम् \(अङ्करहितम्\)/gu)?.length).toBeGreaterThanOrEqual(2);
      expect(renderedLearnerProse(paper)).not.toMatch(/[A-Za-z]{2,}/u);
      expect(paper).not.toMatch(/Hindi|Bengali|Malayalam/u);

      const reading = between(paper, "## Reading", "## Listening");
      expect(numbered(reading)).toBe(32);
      for (const [heading, next, count, minimum, maximum] of [
        ["### Part 1 — argument", "### Part 2", 8, 400, 520],
        ["### Part 2 — reporting", "### Part 3", 8, 420, 560],
        ["### Part 3 — instructions", "### Part 4", 8, 450, 580],
        ["### Part 4 — literary or cultural prose", "## Listening", 8, 530, 640],
      ] as const) {
        const part = between(paper, heading, next);
        expect(numbered(part), heading).toBe(count);
        const source = between(part, "#### Source", "#### Questions");
        expect(devanagariWords(source), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(source), `${heading} maximum`).toBeLessThanOrEqual(maximum);
        expect(source).toContain("ऽ");
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(28);
      expect(listening).toMatch(/[\u0900-\u097F]/u);
      const writing = between(paper, "## Writing", "## Speaking");
      expect(numbered(writing)).toBe(2);
      expect(writing).toContain("१००–१३०");
      expect(writing).toContain("२२०–२८०");
      expect(writing).toContain("बाह्यसन्धिं");
      expect(writing).not.toContain("copyable answer model");
      expect(writing).toMatch(/[\u0900-\u097F]/u);
      const speaking = between(paper, "## Speaking");
      expect(numbered(speaking)).toBe(7);
      expect(speaking).toContain("२१०–२४०");
      expect(speaking).toContain("२४०–३००");
      expect(speaking).toMatch(/[\u0900-\u097F]/u);

      expect(examiner).toContain("Do not give this file to the learner");
      const normalizedExaminer = examiner.replace(/\s+/gu, " ");
      expect(normalizedExaminer).toContain("trained speaker of the living spoken register");
      expect(normalizedExaminer).toContain("150–170 words per minute");
      expect(normalizedExaminer.toLowerCase()).toContain(
        "play every listening part once after one unscored orienting preview",
      );
      const recording = between(examiner, "## Listening recording master", "## Speaking");
      for (const [heading, next, items, minimum, maximum] of [
        ["### Part 1 — multiparty transaction", "### Part 2", 7, 595, 750],
        ["### Part 2 — interview or panel", "### Part 3", 7, 680, 825],
        ["### Part 3 — extended report", "### Part 4", 7, 765, 900],
        ["### Part 4 — cultural narrative", "## Speaking", 7, 765, 975],
      ] as const) {
        const script = between(examiner, heading, next);
        expect(numbered(script), heading).toBe(items);
        expect(devanagariWords(script), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(script), `${heading} maximum`).toBeLessThanOrEqual(maximum);
      }
      expect(normalizedExaminer).toContain("documented regional Sanskrit pronunciation tradition A");
      expect(normalizedExaminer).toContain("documented regional Sanskrit pronunciation tradition B");
      expect(normalizedExaminer).toContain("Before any recording is made");
      expect(normalizedExaminer).toContain("two distinct actual documented traditions");
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
        ["Reading", "Listening", 32], ["Listening", "Writing", 28],
      ] as const) {
        const answers = between(key, `## ${skill}`, `## ${next}`).match(/\b\d{1,2} [ABC](?=;|\.)/gu) ?? [];
        expect(answers.map((answer) => Number.parseInt(answer, 10))).toEqual(
          Array.from({ length: count }, (_, index) => index + 1),
        );
        const labels = answers.map((answer) => answer.at(-1));
        for (const label of ["A", "B", "C"]) {
          expect(labels.filter((candidate) => candidate === label).length, `${name} ${skill} ${label}`)
            .toBeGreaterThanOrEqual(5);
        }
      }
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("speaker provenance and reviewer confirmation");
      expect(key).toContain("once after one unscored orienting preview");
    }

    const semanticRegressions = [
      {
        form: "mock-1",
        question: "17. पुरातनाधिकारिकमानचित्रेष्वभावे स्थानीयमानचित्रैः किं प्रकाशितम्? A सम्पत्तिमूल्यानि / B लघवः ऋतुनिर्भरजलमार्गाः / C नद्याः नाम",
        evidence: "द्वौ मार्गौ पुरातनमानचित्रे सर्वथा नासीताम्",
        item: 17,
        answer: "B",
      },
      {
        form: "mock-1",
        question: "18. प्रतिवेदनं कं निष्कर्षं न वदति? A अनेककारणानि युगपत् परिवर्तितानि / B तृणपुनर्वृद्धिः दृष्टा / C एकेनैव हस्तक्षेपेण सर्वः सुधारो जातः",
        evidence: "कः परिवर्तनः कियत् कारणमिति पृथक्कर्तुं न शक्यते",
        item: 18,
        answer: "C",
      },
      {
        form: "mock-2",
        question: "11. रुक्मिणी कां सुगम्यतासमस्यां निर्दिशति? A आगन्तृभ्यः निःशुल्कोपहाराः आवश्यकाः / B अन्तिमलोकयानम् अतिशीघ्रं निर्गच्छति / C द्विचक्रिकाः निषिद्धाः",
        evidence: "अन्तिमलोकयानं दशवादने निर्गतम्, आपणस्तु एकादशवादने पिहितः",
        item: 11,
        answer: "B",
      },
      {
        form: "mock-2",
        question: "17. प्रथमच्छायामण्डपे किं जातम्? A तत् सद्यः निष्कासितम् / B तेन कापि शीतलता न जाता / C सामग्रीपरिवर्तनात् पूर्वं तस्य कृष्णमावरणं तापं धारयति स्म",
        evidence: "आवरणसामग्री एव प्रथमदोष आसीत्",
        item: 17,
        answer: "C",
      },
    ] as const;
    for (const regression of semanticRegressions) {
      expect(read(`${regression.form}.md`)).toContain(regression.question);
      expect(read(`${regression.form}-examiner-script.md`).replace(/\s+/gu, " "))
        .toContain(regression.evidence.replace(/\s+/gu, " "));
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
    expect(rubric).toContain("100–130 word interaction text");
    expect(normalized).toContain("220–280 word extended text");
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
