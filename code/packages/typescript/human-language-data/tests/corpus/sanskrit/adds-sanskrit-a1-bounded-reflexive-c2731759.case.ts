import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import {
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";
import { sanskritA1PointCoverage } from "../sanskrit-a1-point-coverage.js";

it("teaches one bounded attested Sanskrit reflexive relation twice", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C273-meet-one-reflexive",
    "SA-C273-reflexive-first-retrieval",
    "SA-C273-reflexive-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    273, 273, 273,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    16090, 16100, 16110,
  ]);

  const reflexiveAtom = "SA-PATTERN-C273-REFLEXIVE-01";
  const knownSelfAtom = "SA-LEX-C212-WEL212-03";
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }
  expect(chapterLessons[0]?.frontmatter["introduces.knowledge"]).toEqual([
    reflexiveAtom,
  ]);
  expect(chapterLessons[1]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(chapterLessons[2]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(reflexiveAtom),
    ),
  ).toHaveLength(1);
  expect(
    lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(knownSelfAtom),
    ),
  ).toHaveLength(1);

  const teachingLesson = chapterLessons[0];
  expect(teachingLesson?.frontmatter["requires.knowledge"]).toContain(knownSelfAtom);
  expect(teachingLesson?.body).toContain("अहमेव स्वात्मानं न वेद्मि।");
  expect(teachingLesson?.body).toContain("ahameva svātmānaṃ na vedmi");
  expect(teachingLesson?.body).toContain(
    "https://www.sanskrit.nic.in/ASSP/Vikramcharitam/6frame7.htm",
  );
  expect(teachingLesson?.body).toMatch(/complete attested sentence/u);
  expect(teachingLesson?.body).toMatch(/points back to the first-person subject/u);
  expect(teachingLesson?.body).toMatch(/known आत्मा/u);
  expect(teachingLesson?.body).toMatch(/one whole line/u);
  expect(teachingLesson?.body).not.toMatch(/complete reflexive paradigm/u);

  for (const lesson of chapterLessons.slice(1)) {
    expect(lesson?.frontmatter["requires.knowledge"]).toContain(reflexiveAtom);
    expect(lesson?.frontmatter["practises.knowledge"]).toContain(reflexiveAtom);
    const responseIndex =
      lesson?.blocks.findIndex((block) =>
        block.markdown.includes("No Sanskrit model is shown before you answer"),
      ) ?? -1;
    const checkIndex =
      lesson?.blocks.findIndex((block) => block.markdown.includes("Now check:")) ?? -1;
    expect(responseIndex).toBeGreaterThanOrEqual(0);
    expect(checkIndex).toBeGreaterThan(responseIndex);
    const responseBlock = lesson?.blocks[responseIndex];
    const checkBlock = lesson?.blocks[checkIndex];
    expect(responseBlock?.markdown).toContain("[PAUSE 8s]");
    expect(responseBlock?.markdown.indexOf("Say the Sanskrit")).toBeLessThan(
      responseBlock?.markdown.indexOf("[PAUSE 8s]") ?? -1,
    );
    const authoredPrefix =
      lesson?.blocks
        .slice(0, checkIndex)
        .map((block) => block.markdown)
        .join(" ") ?? "";
    expect(authoredPrefix).not.toMatch(
      /अहमेव स्वात्मानं न वेद्मि|ahameva svātmānaṃ na vedmi/u,
    );
    expect(checkBlock?.markdown).toContain("अहमेव स्वात्मानं न वेद्मि।");
    expect(checkBlock?.markdown).toMatch(/ahameva\s+svātmānaṃ na vedmi/u);
  }

  const narrationPath = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/sanskrit/narration/ch273.json",
  );
  const narration = JSON.parse(readFileSync(narrationPath, "utf8")) as {
    lessons: Array<{
      id: string;
      blocks: Array<{
        type: string;
        segments: Array<{ kind: string; text?: string; seconds?: number }>;
      }>;
    }>;
  };
  for (const id of ids.slice(1)) {
    const spoken = narration.lessons.find((lesson) => lesson.id === id);
    const guidedIndex =
      spoken?.blocks.findIndex((block) => block.type === "guided-production") ?? -1;
    const checkIndex = spoken?.blocks.findIndex((block) => block.type === "notice") ?? -1;
    expect(guidedIndex).toBeGreaterThanOrEqual(0);
    expect(checkIndex).toBeGreaterThan(guidedIndex);
    const guided = spoken?.blocks[guidedIndex];
    expect(guided?.segments.map((segment) => segment.kind)).toEqual([
      "speech",
      "speech",
      "pause",
    ]);
    expect(guided?.segments[1]?.text).toContain("Say the Sanskrit");
    expect(guided?.segments[2]).toMatchObject({ kind: "pause", seconds: 8 });
    const spokenPrefix =
      spoken?.blocks
        .slice(0, checkIndex)
        .flatMap((block) => block.segments)
        .filter((segment) => segment.kind === "speech")
        .map((segment) => segment.text ?? "")
        .join(" ") ?? "";
    expect(spokenPrefix).not.toMatch(
      /अहमेव स्वात्मानं न वेद्मि|ahameva\s+svātmānaṃ na vedmi/u,
    );
    const spokenCheck =
      spoken?.blocks[checkIndex]?.segments
        .filter((segment) => segment.kind === "speech")
        .map((segment) => segment.text ?? "")
        .join(" ") ?? "";
    expect(spokenCheck).toContain("अहमेव स्वात्मानं न वेद्मि।");
    expect(spokenCheck).toMatch(/ahameva\s+svātmānaṃ na vedmi/u);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);
  expect(learnerCopy).toContain(
    "not evidence of middle-voice mastery, a reflexive paradigm, productive case or sandhi mastery, human validation, learner attainment, or exam readiness",
  );

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 273)).toMatchObject({
    chapter: 273,
    payoff: {
      lesson: "SA-C273-reflexive-final-retrieval",
      kind: "production",
      assesses: [reflexiveAtom],
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-PRON-05")?.probe).toEqual([
    reflexiveAtom,
  ]);
  expect(sanskritA1PointCoverage("SA-A1-PRON-05")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
