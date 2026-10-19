import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a bounded Punjabi A2 place-reference prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C180-lok",
    "PA-C180-us",
    "PA-C180-thaan",
    "PA-C180-kamre",
    "PA-C180-vadde",
    "PA-C180-place-review",
    "PA-C180-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 180));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9400, 9410, 9420, 9430, 9440, 9450, 9460]);

  const atoms = [
    "PA-PLACE-LOK-PEOPLE-01",
    "PA-PLACE-US-THAT-01",
    "PA-PLACE-THAAN-LOCATION-01",
    "PA-PLACE-KAMRE-ROOM-01",
    "PA-PLACE-VADDE-BIG-01",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([[atoms[0]], [atoms[1]], [atoms[2]], [atoms[3]], [atoms[4]], [], []]);
  // Atom ownership alone would not catch a future form slipped into an
  // earlier lesson's learner copy. Each form must first appear with its owner.
  const newForms = ["ਲੋਕ", "ਉਸ", "ਥਾਂ", "ਕਮਰੇ", "ਵੱਡੇ"];
  for (const [ownerIndex, form] of newForms.entries()) {
    const earlierCopy = chapterLessons.slice(0, ownerIndex)
      .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
      .join("\n");
    expect(earlierCopy.includes(form), `${form} appears before its owner lesson`)
      .toBe(false);
  }
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"]))
      .toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.spine_node).toBe("SPINE-READ-PRACTICAL-TEXTS");
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }
  for (const atom of atoms) {
    const owners = lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(atom),
    );
    expect(owners).toHaveLength(1);
    const ownerSequence = Number(owners[0].frontmatter.sequence);
    const laterPractices = chapterLessons.filter((lesson) =>
      Number(lesson?.frontmatter.sequence) > ownerSequence &&
      lesson?.frontmatter["practises.knowledge"].includes(atom),
    );
    expect(laterPractices.length, `${atom} needs two later retrieval lessons`)
      .toBeGreaterThanOrEqual(2);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n").replace(/\s+/gu, " ");
  for (const form of [...newForms,
    "ਲੋਕ ਉਸ ਥਾਂ ਦੇ ਵੱਡੇ ਕਮਰੇ ਵਿੱਚ ਹਨ।"]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toMatch(/third prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9400)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const newlyTaught of newForms) {
    supportingPunjabi.delete(newlyTaught);
  }
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 180`).toBe(true);
  }

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 180)).toMatchObject({
    chapter: 180,
    payoff: { lesson: "PA-C180-payoff", kind: "production", assesses: atoms },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-180-A2-PLACE-REFERENCE",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "vocabulary" });
  expect(extension?.prerequisites).toContain("PA-EXT-179-A2-SCHEDULE-STATUS");
}, 120_000);
