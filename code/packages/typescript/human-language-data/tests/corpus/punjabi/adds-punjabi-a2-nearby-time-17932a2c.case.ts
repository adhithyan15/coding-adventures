import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a slow Punjabi A2 nearby-time prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C184-time-return",
    "PA-C184-hafte",
    "PA-C184-mahine",
    "PA-C184-agli",
    "PA-C184-agle",
    "PA-C184-pichhle",
    "PA-C184-time-review",
    "PA-C184-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 184));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9730, 9740, 9750, 9760, 9770, 9780, 9790, 9800]);

  const forms = ["ਹਫ਼ਤੇ", "ਮਹੀਨੇ", "ਅਗਲੀ", "ਅਗਲੇ", "ਪਿਛਲੇ"];
  const atoms = [
    "PA-FORM-HAFTE-184",
    "PA-FORM-MAHINE-184",
    "PA-FORM-AGLI-184",
    "PA-FORM-AGLE-184",
    "PA-FORM-PICHHLE-184",
  ];
  const ownerIndexes = [1, 2, 3, 4, 5];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([[], [atoms[0]], [atoms[1]], [atoms[2]], [atoms[3]], [atoms[4]], [], []]);

  for (const [formIndex, form] of forms.entries()) {
    const ownerIndex = ownerIndexes[formIndex];
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
  for (const form of forms) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("ਦੋ ਹਫ਼ਤੇ");
  expect(learnerCopy).toContain("ਹਰ ਮਹੀਨੇ");
  expect(learnerCopy).toContain("ਅਗਲੀ ਯਾਤਰਾ");
  expect(learnerCopy).toContain("ਅਗਲੇ ਮਹੀਨੇ");
  expect(learnerCopy).toContain("ਪਿਛਲੇ ਹਫ਼ਤੇ");
  expect(learnerCopy).toMatch(/seventh prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9730)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const form of forms) supportingPunjabi.delete(form);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 184`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch184-next-week-last-week.tex",
    import.meta.url,
  ), "utf8");
  for (const [formIndex, form] of forms.entries()) {
    const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndexes[formIndex]]}}`);
    expect(ownerLabel).toBeGreaterThan(0);
    const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
    expect(ownerSection).toBeGreaterThan(0);
    expect(generatedBook.slice(0, ownerSection)).not.toContain(form);
  }
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 184)).toMatchObject({
    chapter: 184,
    payoff: { lesson: "PA-C184-payoff", kind: "production", assesses: atoms },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-184-A2-NEARBY-TIME",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-183-A2-HABITUAL-ROUTINE");
}, 120_000);
