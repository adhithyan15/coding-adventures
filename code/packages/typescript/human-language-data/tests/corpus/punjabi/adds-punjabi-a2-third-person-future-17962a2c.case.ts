import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

it("opens Punjabi chapter 186 by ear while retrieving its two familiar infinitives", () => {
  const opener = loadTrackLessons("punjabi").find(
    (lesson) => lesson.realization.lessonId === "PA-C186-future-return",
  );
  expect(opener).toBeDefined();
  const [entry] = orderChapterLessons(lessonModalities([opener!]));
  expect(entry?.coreModality).toBe("voice");
  expect(unstartableCause([entry!])).toBeNull();
  expect(opener?.blocks.some((block) => block.markdown.includes("[YOU HEAR:")))
    .toBe(true);
});

it("adds a slow Punjabi A2 third-person future prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C186-future-return",
    "PA-C186-hovega",
    "PA-C186-hovegi",
    "PA-C186-hovega-hovegi-review",
    "PA-C186-aavega",
    "PA-C186-aavegi",
    "PA-C186-future-review",
    "PA-C186-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 186));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9910, 9920, 9930, 9940, 9950, 9960, 9970, 9980]);

  const targets = [
    { form: "ਹੋਵੇਗਾ", atom: "PA-FORM-HOVEGA-186", ownerIndex: 1 },
    { form: "ਹੋਵੇਗੀ", atom: "PA-FORM-HOVEGI-186", ownerIndex: 2 },
    { form: "ਆਵੇਗਾ", atom: "PA-FORM-AAVEGA-186", ownerIndex: 4 },
    { form: "ਆਵੇਗੀ", atom: "PA-FORM-AAVEGI-186", ownerIndex: 5 },
  ] as const;
  const forms = targets.map((target) => target.form);
  const atoms = targets.map((target) => target.atom);
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([[], [atoms[0]], [atoms[1]], [], [atoms[2]], [atoms[3]], [], []]);

  for (const { form, ownerIndex } of targets) {
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

  for (const { atom } of targets) {
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
  for (const form of forms) expect(learnerCopy).toContain(form);
  expect(learnerCopy).toContain("ਕੰਮ ਹੋਵੇਗਾ");
  expect(learnerCopy).toContain("ਯਾਤਰਾ ਹੋਵੇਗੀ");
  expect(learnerCopy).toContain("ਅਮਨ ਆਵੇਗਾ");
  expect(learnerCopy).toContain("ਰੇਲ ਆਵੇਗੀ");
  expect(learnerCopy).toMatch(/ninth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9910)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const { form } of targets) supportingPunjabi.delete(form);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 186`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch186-what-will-happen-who-will-come.tex",
    import.meta.url,
  ), "utf8");
  for (const { form, ownerIndex } of targets) {
    const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
    expect(ownerLabel).toBeGreaterThan(0);
    const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
    expect(ownerSection).toBeGreaterThan(0);
    expect(generatedBook.slice(0, ownerSection)).not.toContain(form);
  }
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch186.json",
    import.meta.url,
  ), "utf8")) as { lessons: Array<{ id: string }> };
  expect(narration.lessons.map((lesson) => lesson.id)).toEqual(ids);
  for (const { form, ownerIndex } of targets) {
    const narrationBeforeOwner = JSON.stringify(narration.lessons.slice(0, ownerIndex));
    expect(
      narrationBeforeOwner.includes(form),
      `${form} leaks in generated narration before ${ids[ownerIndex]}`,
    ).toBe(false);
  }

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 186)).toMatchObject({
    chapter: 186,
    payoff: { lesson: "PA-C186-payoff", kind: "production", assesses: atoms },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-186-A2-THIRD-PERSON-FUTURE",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-185-A2-POSSESSIVE-AGREEMENT");
}, 120_000);
