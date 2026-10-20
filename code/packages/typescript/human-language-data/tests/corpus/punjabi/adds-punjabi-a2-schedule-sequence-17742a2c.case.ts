import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a bounded Punjabi A2 schedule and sequence prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C178-vaje",
    "PA-C178-ton",
    "PA-C178-tak",
    "PA-C178-naal",
    "PA-C178-baad",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );

  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    178, 178, 178, 178, 178,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    9280, 9290, 9300, 9310, 9320,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const atoms = [
    "PA-TIME-VAJE-01",
    "PA-POSTPOSITION-TON-SOURCE-01",
    "PA-POSTPOSITION-TAK-LIMIT-01",
    "PA-POSTPOSITION-NAAL-COMITATIVE-01",
    "PA-TIME-TON-BAAD-SEQUENCE-01",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual(atoms.map((atom) => [atom]));
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
    "ਅੱਠ ਵਜੇ",
    "ਘਰ ਤੋਂ ਸਕੂਲ ਜਾਣਾ",
    "ਅੱਠ ਵਜੇ ਤੋਂ ਦਸ ਵਜੇ ਤੱਕ",
    "ਅਮਨ ਨਾਲ ਕੰਮ ਕਰੋ",
    "ਦਸ ਵਜੇ ਤੋਂ ਬਾਅਦ",
    "ਅਮਨ ਨਾਲ ਅੱਠ ਵਜੇ ਤੋਂ ਦਸ ਵਜੇ ਤੱਕ ਕੰਮ ਕਰੋ। ਦਸ ਵਜੇ ਤੋਂ ਬਾਅਦ ਘਰ ਆਉਣਾ।",
  ]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toMatch(/after its noun|postposition comes after/i);
  expect(learnerCopy).toMatch(/first prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9280)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(
    learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? [],
  );
  for (const newlyTaught of ["ਵਜੇ", "ਤੋਂ", "ਤੱਕ", "ਨਾਲ", "ਬਾਅਦ"]) {
    supportingPunjabi.delete(newlyTaught);
  }
  for (const token of supportingPunjabi) {
    expect(taughtBefore, `${token} must already occur before chapter 178`).toContain(token);
  }

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills).toEqual([
    "listening",
    "speaking",
    "reading",
    "writing",
  ]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:"))).toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 178)).toMatchObject({
    chapter: 178,
    payoff: {
      lesson: "PA-C178-baad",
      kind: "production",
      assesses: atoms,
    },
  });

  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-178-A2-SCHEDULE-SEQUENCE"
  );
  expect(extension).toMatchObject({
    stage: "A2",
    kind: "required",
    category: "grammar",
  });
  expect(extension?.prerequisites).toContain("PA-EXT-177-NOUN-PHRASE-AGREEMENT");
}, 120_000);
