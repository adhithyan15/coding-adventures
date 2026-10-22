import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

it("opens Punjabi chapter 187 by ear while retrieving singular past ਸੀ", () => {
  const opener = loadTrackLessons("punjabi").find(
    (lesson) => lesson.realization.lessonId === "PA-C187-si-return",
  );
  expect(opener).toBeDefined();
  const [entry] = orderChapterLessons(lessonModalities([opener!]));
  expect(entry?.coreModality).toBe("voice");
  expect(unstartableCause([entry!])).toBeNull();
  expect(opener?.blocks.some((block) => block.markdown.includes("[YOU HEAR:")))
    .toBe(true);
});

it("adds a slow Punjabi A2 plural past-copula prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C187-si-return",
    "PA-C187-san",
    "PA-C187-si-san-contrast",
    "PA-C187-san-description",
    "PA-C187-san-habitual",
    "PA-C187-san-review",
    "PA-C187-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 187));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9990, 10000, 10010, 10020, 10030, 10040, 10050]);

  const form = "ਸਨ";
  const atom = "PA-FORM-SAN-187";
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([[], [atom], [], [], [], [], []]);

  const ownerIndex = 1;
  const earlierCopy = chapterLessons.slice(0, ownerIndex)
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n");
  expect(earlierCopy.includes(form), `${form} appears before its owner lesson`)
    .toBe(false);

  const rawBeforeOwner = ids.slice(0, ownerIndex).map((id) => {
    const source = readFileSync(new URL(
      `../../../../../../learning/human-languages/punjabi/lessons/${id}.md`,
      import.meta.url,
    ), "utf8");
    return source.replace(/^---[\s\S]*?---\s*/u, "");
  }).join("\n");
  expect(rawBeforeOwner.includes(form), `${form} leaks in raw authored copy before its owner`)
    .toBe(false);

  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"]))
      .toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.spine_node).toBe("SPINE-READ-PRACTICAL-TEXTS");
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const owners = lessons.filter((lesson) =>
    lesson.frontmatter["introduces.knowledge"].includes(atom),
  );
  expect(owners).toHaveLength(1);
  const ownerSequence = Number(owners[0].frontmatter.sequence);
  const laterPractices = chapterLessons.filter((lesson) =>
    Number(lesson?.frontmatter.sequence) > ownerSequence &&
    lesson?.frontmatter["practises.knowledge"].includes(atom),
  );
  expect(laterPractices.length).toBeGreaterThanOrEqual(2);

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n").replace(/\s+/gu, " ");
  expect(learnerCopy).toContain("ਫੁੱਲ ਲਾਲ ਸਨ");
  expect(learnerCopy).toContain("ਉਹ ਪੰਜਾਬੀ ਬੋਲਦੇ ਸਨ");
  expect(learnerCopy).toMatch(/tenth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9990)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  supportingPunjabi.delete(form);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 187`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch187-they-were-there.tex",
    import.meta.url,
  ), "utf8");
  const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
  expect(ownerLabel).toBeGreaterThan(0);
  const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
  expect(ownerSection).toBeGreaterThan(0);
  expect(generatedBook.slice(0, ownerSection)).not.toContain(form);
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch187.json",
    import.meta.url,
  ), "utf8")) as { lessons: Array<{ id: string }> };
  expect(narration.lessons.map((lesson) => lesson.id)).toEqual(ids);
  expect(JSON.stringify(narration.lessons.slice(0, ownerIndex))).not.toContain(form);

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 187)).toMatchObject({
    chapter: 187,
    payoff: { lesson: "PA-C187-payoff", kind: "production", assesses: [atom] },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-187-A2-PLURAL-PAST-COPULA",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-186-A2-THIRD-PERSON-FUTURE");
}, 120_000);
