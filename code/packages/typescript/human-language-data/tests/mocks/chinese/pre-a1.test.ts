import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

// A mock is only book-bounded when its assessed words are already headwords
// before this rung. Body prose can mention an untaught word in passing, so the
// check deliberately reads the lesson front matter and curriculum membership.
const trackRoot = join(defaultCurriculumRoot(), "chinese");
const mockRoot = join(trackRoot, "mocks", "pre-a1");
const readMock = (name: string) => readFileSync(join(mockRoot, name), "utf8");

const taught = [
  ["ZH-C01-ni", "你"],
  ["ZH-C01-hao", "好"],
  ["ZH-C01-nihao", "你好"],
  ["ZH-C04-bu", "不"],
  ["ZH-C05-shi", "是"],
  ["ZH-C06-zaijian", "再见"],
  ["ZH-C07-zaoshanghao", "早上好"],
  ["ZH-C08-wo", "我"],
  ["ZH-C08-mingzi", "名字"],
  ["ZH-C09-shenme", "什么"],
  ["ZH-C10-xiexie", "谢谢"],
  ["ZH-C10-buxie", "不谢"],
  ["ZH-C11-qing", "请"],
] as const;

const expectedParts = [
  "prea1-reading-pinyin-signs-and-doorway-words",
  "prea1-listening-tones-and-doorway-turns",
  "prea1-writing-delayed-character-recall",
  "prea1-writing-dictation-and-transcription",
  "prea1-writing-bounded-production",
  "prea1-speaking-tone-and-doorway-exchange",
];

describe("Chinese pre-A1 timed mock pair", () => {
  it("keeps two complete, independent four-skill forms on the task-shape timing", () => {
    const shape = JSON.parse(readFileSync(join(trackRoot, "task-shapes", "pre-a1.json"), "utf8"));
    const papers: string[] = [];

    for (const form of [1, 2]) {
      const manifest = JSON.parse(readMock(`mock-${form}-manifest.json`));
      expect(manifest.mockId).toBe(`pre-a1-mock-${form}`);
      expect(manifest.sections.map((section: { skill: string; minutes: number }) =>
        [section.skill, section.minutes])).toEqual(shape.sections.map(
        (section: { skill: string; minutes: number }) => [section.skill, section.minutes],
      ));
      expect(manifest.sections.flatMap((section: { partIds: string[] }) => section.partIds))
        .toEqual(expectedParts);

      const paper = readMock(`mock-${form}-paper.md`);
      const key = readMock(`mock-${form}-answer-key.md`);
      const rubric = readMock(`mock-${form}-rubric.md`);
      papers.push(paper);
      for (const part of expectedParts) {
        for (const artifact of [paper, key, rubric]) {
          expect(artifact, `${form}: ${part}`).toContain(`<!-- mock-part:${part} -->`);
          expect(artifact, `${form}: ${part}`).toContain(`<!-- /mock-part:${part} -->`);
        }
      }
      expect([...paper.matchAll(/^(\d+)\./gmu)].map((match) => Number(match[1])))
        .toEqual(Array.from({ length: 20 }, (_, index) => index + 1));
      expect(paper).toContain("ten seconds");
      expect(key).toContain("ten seconds");
      expect(rubric).toContain("ten seconds");
      expect(paper).toContain("15/25");
      expect(paper).toContain("60/100");
      expect(paper).not.toContain("**mǎ** (third tone; isolated syllable)");
      const picturePart = paper.split("<!-- mock-part:prea1-writing-bounded-production -->")[1]
        .split("<!-- /mock-part:prea1-writing-bounded-production -->")[0];
      const pictureFiles = [...picturePart.matchAll(/!\[[^\]]+\]\(([^)]+\.svg)\)/gu)]
        .map((match) => match[1]);
      expect(pictureFiles).toHaveLength(2);
      for (const file of pictureFiles) {
        const svg = readMock(file);
        expect(svg).toContain("<svg");
        expect(svg).toContain("<title>");
      }
    }
    expect(papers[0]).not.toBe(papers[1]);
  });

  it("bounds assessed vocabulary to taught Chinese headwords", () => {
    for (const [lessonId, headword] of taught) {
      const lesson = readFileSync(join(trackRoot, "lessons", `${lessonId}.md`), "utf8");
      expect(lesson, lessonId).toContain(`headword: ${headword}\n`);
      expect(existsSync(join(trackRoot, "curriculum-membership.d", `${lessonId}.json`)), lessonId)
        .toBe(true);
    }

    for (const form of [1, 2]) {
      const paper = readMock(`mock-${form}-paper.md`);
      const key = readMock(`mock-${form}-answer-key.md`);
      // Objective answers, options, and examiner audio are all drawn from this
      // finite taught set; a new lexical item needs a new explicit mapping.
      const knownCharacters = new Set([...taught.flatMap(([, word]) => [...word]), "谢"]);
      for (const glyph of (paper + key).match(/[\p{Script=Han}]/gu) ?? []) {
        expect(knownCharacters.has(glyph), `${form}: untaught glyph ${glyph}`).toBe(true);
      }
      for (const [, lessonId] of key.matchAll(/`(ZH-C\d\d-[a-z-]+)`/gu)) {
        expect(existsSync(join(trackRoot, "lessons", `${lessonId}.md`)), lessonId).toBe(true);
      }
      expect(key).toMatch(/Reading, 5 points each:/u);
      expect(key).toMatch(/Each correct selection/u);
    }
  });
});
