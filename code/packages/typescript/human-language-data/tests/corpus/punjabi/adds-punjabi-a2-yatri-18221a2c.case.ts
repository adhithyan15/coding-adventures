import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

const ids = [
  "PA-C202-identity-return",
  "PA-C202-yatri",
  "PA-C202-yatri-review",
  "PA-C202-traveller-label",
  "PA-C202-payoff",
];

it("opens Punjabi chapter 202 by ear without leaking the new word", () => {
  const opener = loadTrackLessons("punjabi").find(
    (lesson) => lesson.realization.lessonId === ids[0],
  );
  expect(opener).toBeDefined();
  const [entry] = orderChapterLessons(lessonModalities([opener!]));
  expect(entry?.coreModality).toBe("voice");
  expect(unstartableCause([entry!])).toBeNull();
  expect(opener?.blocks.some((block) => block.markdown.includes("[YOU HEAR:")))
    .toBe(true);
});

it("adds a slow Punjabi A2 traveller prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 202));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([10970, 10980, 10990, 11000, 11010]);

  const target = "ਯਾਤਰੀ";
  const atom = "PA-LEX-YATRI-202";
  const ownerIndex = 1;
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([[], [atom], [], [], []]);

  const rawBeforeOwner = ids.slice(0, ownerIndex).map((id) => {
    const source = readFileSync(new URL(
      `../../../../../../learning/human-languages/punjabi/lessons/${id}.md`,
      import.meta.url,
    ), "utf8");
    return source.replace(/^---[\s\S]*?---\s*/u, "");
  }).join("\n");
  expect(rawBeforeOwner.includes(target), "target leaks in raw authored copy before its owner")
    .toBe(false);

  const directOwners = lessons.filter((lesson) =>
    lesson.frontmatter["introduces.knowledge"].includes(atom),
  );
  expect(directOwners).toHaveLength(1);
  const ownerSequence = Number(directOwners[0].frontmatter.sequence);
  const laterPractices = chapterLessons.filter((lesson) =>
    Number(lesson?.frontmatter.sequence) > ownerSequence &&
    lesson?.frontmatter["practises.knowledge"].includes(atom),
  );
  expect(laterPractices).toHaveLength(3);

  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"]))
      .toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.spine_node).toBe("SPINE-READ-PRACTICAL-TEXTS");
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n").replace(/\s+/gu, " ");
  expect(learnerCopy).toContain("ਇਹ ਯਾਤਰੀ ਹੈ");
  expect(learnerCopy).toMatch(/person.*noun|noun.*person/is);
  expect(learnerCopy).toMatch(/person who travels|traveller.*person/is);
  expect(learnerCopy).toMatch(/keep.*whole|whole word/is);
  expect(learnerCopy).toContain("https://static.pseb.ac.in/media/1749801072_");
  expect(learnerCopy).toContain("https://static.pseb.ac.in/media/1687845902_");
  expect(learnerCopy).toMatch(/current.*educational Punjabi|educational Punjabi.*current/is);
  expect(learnerCopy).toMatch(/exact origin.*remains unknown/is);
  expect(learnerCopy).not.toMatch(/feminine|masculine|plural|oblique|inflection/is);
  expect(learnerCopy).not.toMatch(/ਪੁਲਾੜ-ਯਾਤਰੀ|passenger|synonym|derived from ਯਾਤਰਾ/is);
  expect(learnerCopy).toMatch(/twenty-fifth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 10970)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  supportingPunjabi.delete(target);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 202`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch202-this-person-is-a-traveller.tex",
    import.meta.url,
  ), "utf8");
  const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
  expect(ownerLabel).toBeGreaterThan(0);
  const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
  expect(ownerSection).toBeGreaterThan(0);
  expect(generatedBook.slice(0, ownerSection)).not.toContain(target);
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch202.json",
    import.meta.url,
  ), "utf8")) as { lessons: Array<{ id: string }> };
  expect(narration.lessons.map((lesson) => lesson.id)).toEqual(ids);
  expect(JSON.stringify(narration.lessons.slice(0, ownerIndex))).not.toContain(target);

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 202)).toMatchObject({
    chapter: 202,
    payoff: {
      lesson: "PA-C202-payoff",
      kind: "production",
      assesses: [atom],
    },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-202-A2-YATRI",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "vocabulary" });
  expect(extension?.prerequisites).toContain("PA-EXT-201-A2-CLASS");
}, 120_000);
