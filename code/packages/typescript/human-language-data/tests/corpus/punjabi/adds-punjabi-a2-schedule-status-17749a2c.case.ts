import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a reinforced Punjabi A2 schedule-status prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C179-samaa",
    "PA-C179-savere",
    "PA-C179-khullha",
    "PA-C179-dupahir",
    "PA-C179-band",
    "PA-C179-status-review",
    "PA-C179-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );

  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 179));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9330, 9340, 9350, 9360, 9370, 9380, 9390]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.spine_node).toBe("SPINE-READ-PRACTICAL-TEXTS");
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const atoms = [
    "PA-TIME-SAMAA-01",
    "PA-TIME-SAVERE-DAYPART-01",
    "PA-STATUS-KHULLHA-OPEN-01",
    "PA-TIME-DUPAHIR-DAYPART-01",
    "PA-STATUS-BAND-CLOSED-01",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([
      [atoms[0]], [atoms[1]], [atoms[2]], [atoms[3]], [atoms[4]], [], [],
    ]);
  for (const atom of atoms) {
    const owners = lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(atom)
    );
    expect(owners).toHaveLength(1);
    const ownerSequence = Number(owners[0].frontmatter.sequence);
    const laterPractices = chapterLessons.filter((lesson) =>
      Number(lesson?.frontmatter.sequence) > ownerSequence &&
      lesson?.frontmatter["practises.knowledge"].includes(atom)
    );
    expect(laterPractices.length, `${atom} must have at least two later lesson retrievals`)
      .toBeGreaterThanOrEqual(2);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of [
    "ਸਮਾਂ",
    "ਸਵੇਰੇ ਅੱਠ ਵਜੇ",
    "ਬਾਗ਼ ਸਵੇਰੇ ਖੁੱਲ੍ਹਾ ਹੈ",
    "ਦੁਪਹਿਰ ਦੋ ਵਜੇ",
    "ਬਾਗ਼ ਬੰਦ ਹੈ",
    "ਬਾਗ਼ ਸਵੇਰੇ ਅੱਠ ਵਜੇ ਖੁੱਲ੍ਹਾ ਹੈ। ਦੁਪਹਿਰ ਦੋ ਵਜੇ ਬੰਦ ਹੈ।",
  ]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toMatch(/second prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9330)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const newlyTaught of ["ਸਮਾਂ", "ਸਵੇਰੇ", "ਖੁੱਲ੍ਹਾ", "ਦੁਪਹਿਰ", "ਬੰਦ"]) {
    supportingPunjabi.delete(newlyTaught);
  }
  for (const token of supportingPunjabi) {
    expect(taughtBefore, `${token} must already occur before chapter 179`).toContain(token);
  }

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills).toEqual([
    "listening", "speaking", "reading", "writing",
  ]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:"))).toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 179)).toMatchObject({
    chapter: 179,
    payoff: {
      lesson: "PA-C179-payoff",
      kind: "production",
      assesses: atoms,
    },
  });

  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-179-A2-SCHEDULE-STATUS"
  );
  expect(extension).toMatchObject({
    stage: "A2",
    kind: "required",
    category: "vocabulary",
  });
  expect(extension?.prerequisites).toContain("PA-EXT-178-A2-SCHEDULE-SEQUENCE");
}, 120_000);
