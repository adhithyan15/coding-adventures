import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

// The task shape is the authored contract, not a set of numbers reconstructed
// from the paper. Each mock must prove every part independently.
const trackRoot = join(defaultCurriculumRoot(), "chinese");
const mockRoot = join(trackRoot, "mocks", "a1");
const readMock = (name: string) => readFileSync(join(mockRoot, name), "utf8");
const shape = JSON.parse(readFileSync(join(trackRoot, "task-shapes", "a1.json"), "utf8"));
const han = /\p{Script=Han}/gu;
const taughtA1Characters = new Set(
  readdirSync(join(trackRoot, "lessons"))
    .filter((file) => file.endsWith(".md"))
    .flatMap((file) => {
      const lesson = readFileSync(join(trackRoot, "lessons", file), "utf8");
      const chapter = lesson.match(/^chapter:\s*(\d+)/mu);
      const headword = lesson.match(/^headword:\s*"?([^"\r\n]+)/mu);
      return chapter && Number(chapter[1]) <= 146 && headword
        ? [...headword[1].matchAll(han)].map(([character]) => character)
        : [];
    }),
);

const expectedParts = [
  ["a1-reading-signs-and-form-fields", 10],
  ["a1-reading-short-messages", 10],
  ["a1-listening-familiar-cues", 10],
  ["a1-listening-short-exchanges", 10],
  ["a1-writing-practical-form", 1],
  ["a1-writing-short-message", 1],
  ["a1-speaking-personal-exchange", 5],
  ["a1-speaking-rehearsed-role-play", 1],
] as const;

function partBody(artifact: string, partId: string): string {
  const start = `<!-- mock-part:${partId} -->`;
  const end = `<!-- /mock-part:${partId} -->`;
  expect(artifact).toContain(start);
  expect(artifact).toContain(end);
  return artifact.split(start)[1]!.split(end)[0]!;
}

describe("Chinese A1 timed mock pair", () => {
  it("delivers two distinct complete project-defined four-skill forms", () => {
    expect(shape.administration.speakingPreparationMinutes).toBe(2);
    expect(shape.sections.flatMap((section: { parts: Array<{ id: string; items: number }> }) =>
      section.parts.map((part) => [part.id, part.items]))).toEqual(expectedParts);
    expect(shape.sections.map((section: { parts: Array<{ scoring: { maxRawPoints: number } }> }) =>
      section.parts.reduce((points, part) => points + part.scoring.maxRawPoints, 0)))
      .toEqual([100, 100, 100, 100]);
    const papers: string[] = [];
    for (const form of [1, 2]) {
      const manifest = JSON.parse(readMock(`mock-${form}-manifest.json`));
      expect(manifest).toMatchObject({ version: 1, language: "chinese", level: "A1", mockId: `a1-mock-${form}` });
      expect(manifest.sections.map((section: { skill: string; minutes: number }) =>
        [section.skill, section.minutes])).toEqual(shape.sections.map(
        (section: { skill: string; minutes: number }) => [section.skill, section.minutes],
      ));
      expect(manifest.sections.flatMap((section: { partIds: string[] }) => section.partIds))
        .toEqual(expectedParts.map(([partId]) => partId));

      const paper = readMock(`mock-${form}-paper.md`);
      const key = readMock(`mock-${form}-answer-key.md`);
      const rubric = readMock(`mock-${form}-rubric.md`);
      papers.push(paper);
      expect(paper).toContain("project-defined");
      expect(paper).toContain("not an official HSK paper");
      expect(paper).toContain("60/100 in each skill");
      expect(paper).toContain("17 minutes");
      expect(paper).toContain("15 minutes");
      expect(paper).toContain("8 minutes");
      expect(paper).toContain("two separate minutes of speaking preparation");
      for (const [partId, itemCount] of expectedParts) {
        const paperPart = partBody(paper, partId);
        partBody(key, partId);
        partBody(rubric, partId);
        expect([...paperPart.matchAll(/^\d+\./gmu)], `${form}: ${partId}`).toHaveLength(itemCount);
      }
      expect([...paper.matchAll(/^\d+\./gmu)].map((match) => Number(match[0].slice(0, -1))))
        .toEqual(Array.from({ length: 48 }, (_, index) => index + 1));
      expect(key).toContain("recording script");
      expect(rubric).toContain("independent");
      expect(rubric).toContain("Two trained raters independently score");
      expect(rubric).toContain("four equally weighted dimensions");
      const unintroduced = [...new Set([...paper.matchAll(han), ...key.matchAll(han)]
        .map(([character]) => character))].filter((character) => !taughtA1Characters.has(character));
      expect(unintroduced, `form ${form} uses only pre-A1/A1 introduced characters`).toEqual([]);
      for (const partId of ["a1-reading-signs-and-form-fields", "a1-reading-short-messages"]) {
        const text = partBody(paper, partId);
        const stimuli = [...text.matchAll(/^\d+\. \*\*([^*]+)\*\*/gmu)].map((match) => match[1]);
        expect(stimuli).toHaveLength(10);
        const [minimum, maximum] = partId.endsWith("short-messages") ? [12, 30] : [2, 10];
        for (const stimulus of stimuli) {
          const characters = [...stimulus.matchAll(han)].length;
          expect(characters).toBeGreaterThanOrEqual(minimum);
          expect(characters).toBeLessThanOrEqual(maximum);
        }
      }
    }
    expect(papers[0]).not.toBe(papers[1]);
  });
});
