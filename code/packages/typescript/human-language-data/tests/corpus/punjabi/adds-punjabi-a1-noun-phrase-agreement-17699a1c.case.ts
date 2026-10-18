import { expect, it } from "vitest";
import {
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";
import { measureExamCoverage } from "../../../src/exam-inventory.js";

it("adds one bounded Punjabi A1 noun-phrase agreement tranche without readiness credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C177-noun-gender",
    "PA-C177-adjective-position",
    "PA-C177-adjective-agreement",
    "PA-C177-bahut",
    "PA-C177-noun-phrase",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );

  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    177, 177, 177, 177, 177,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    9230, 9240, 9250, 9260, 9270,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const atoms = [
    "PA-GRAMMAR-NOUN-GENDER-01",
    "PA-GRAMMAR-ADJ-PRENOMINAL-01",
    "PA-GRAMMAR-ADJ-GENDER-AGREEMENT-01",
    "PA-ADV-BAHUT-DEGREE-01",
    "PA-GRAMMAR-MULTIWORD-NP-01",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual(
    atoms.map((atom) => [atom]),
  );
  for (const atom of atoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of [
    "ਮੁੰਡਾ",
    "ਕੁੜੀ",
    "ਚੰਗਾ ਮੁੰਡਾ",
    "ਚੰਗੀ ਕੁੜੀ",
    "ਬਹੁਤ ਚੰਗਾ ਮੁੰਡਾ",
    "ਬਹੁਤ ਚੰਗੀ ਕੁੜੀ",
    "ਇਹ ਬਹੁਤ ਚੰਗੀ ਕੁੜੀ ਹੈ",
  ]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toMatch(/singular direct|this one bounded contrast/i);
  expect(learnerCopy).toMatch(/not an A1 mock|not A1 readiness/i);

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills).toEqual([
    "listening",
    "speaking",
    "reading",
    "writing",
  ]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:"))).toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 177)).toMatchObject({
    chapter: 177,
    payoff: {
      lesson: "PA-C177-noun-phrase",
      kind: "production",
      assesses: atoms,
    },
  });

  const pointAtoms: Record<string, string> = {
    "PA-A1-N-04": atoms[0],
    "PA-A1-ADJ-03": atoms[1],
    "PA-A1-ADJ-02": atoms[2],
    "PA-A1-ADJ-05": atoms[3],
    "PA-A1-S-08": atoms[4],
  };
  const inventory = loadExamInventory("punjabi", "A1");
  for (const [id, atom] of Object.entries(pointAtoms)) {
    expect(inventory.points.find((point) => point.id === id)?.probe).toEqual([atom]);
  }

  const coverage = measureExamCoverage(inventory, lessons);
  // This direct owner pins only the five points it introduced. Aggregate
  // Punjabi coverage belongs to tests/exam-inventories/punjabi.test.ts so a
  // later independent tranche never has to rewrite this historical owner.
  for (const id of Object.keys(pointAtoms)) {
    expect(coverage.points.find((point) => point.id === id)).toMatchObject({
      covered: true,
      missingAtoms: [],
      unmapped: false,
    });
  }
}, 120_000);
