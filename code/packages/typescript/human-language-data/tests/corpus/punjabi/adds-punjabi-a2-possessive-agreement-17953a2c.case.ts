import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

it("opens Punjabi chapter 185 by ear while retaining its printed base review", () => {
  const opener = loadTrackLessons("punjabi").find(
    (lesson) => lesson.realization.lessonId === "PA-C185-possessive-return",
  );
  expect(opener).toBeDefined();
  const [entry] = orderChapterLessons(lessonModalities([opener!]));
  expect(entry?.coreModality).toBe("voice");
  expect(unstartableCause([entry!])).toBeNull();
  expect(opener?.blocks.some((block) => block.markdown.includes("[YOU READ: **ਮੇਰਾ")))
    .toBe(true);
});

it("adds a slow Punjabi A2 possessive-agreement prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C185-possessive-return",
    "PA-C185-meri",
    "PA-C185-mere",
    "PA-C185-tuhadi",
    "PA-C185-apni",
    "PA-C185-apne",
    "PA-C185-sada",
    "PA-C185-sade",
    "PA-C185-possessive-review",
    "PA-C185-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 185));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9810, 9820, 9830, 9840, 9850, 9860, 9870, 9880, 9890, 9900]);

  const scaffoldAtom = "PA-FORM-SADA-185";
  const targets = [
    { form: "ਮੇਰੀ", atom: "PA-FORM-MERI-185", ownerIndex: 1, measured: true },
    { form: "ਮੇਰੇ", atom: "PA-FORM-MERE-185", ownerIndex: 2, measured: true },
    { form: "ਤੁਹਾਡੀ", atom: "PA-FORM-TUHADI-185", ownerIndex: 3, measured: true },
    { form: "ਆਪਣੀ", atom: "PA-FORM-APNI-185", ownerIndex: 4, measured: true },
    { form: "ਆਪਣੇ", atom: "PA-FORM-APNE-185", ownerIndex: 5, measured: true },
    { form: "ਸਾਡਾ", atom: scaffoldAtom, ownerIndex: 6, measured: false },
    { form: "ਸਾਡੇ", atom: "PA-FORM-SADE-185", ownerIndex: 7, measured: true },
  ] as const;
  const measuredTargets = targets.filter((target) => target.measured);
  const forms = measuredTargets.map((target) => target.form);
  const atoms = measuredTargets.map((target) => target.atom);
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([
      [],
      [atoms[0]],
      [atoms[1]],
      [atoms[2]],
      [atoms[3]],
      [atoms[4]],
      [scaffoldAtom],
      [atoms[5]],
      [],
      [],
    ]);

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
  expect(learnerCopy).toContain("ਸਾਡਾ");
  expect(learnerCopy).toContain("ਮੇਰੀ ਕਿਤਾਬ");
  expect(learnerCopy).toContain("ਮੇਰੇ ਦੋਸਤ");
  expect(learnerCopy).toContain("ਤੁਹਾਡੀ ਯਾਤਰਾ");
  expect(learnerCopy).toContain("ਆਪਣੀ ਕਿਤਾਬ");
  expect(learnerCopy).toContain("ਆਪਣੇ ਦੋਸਤ");
  expect(learnerCopy).toContain("ਸਾਡੇ ਘਰ");
  expect(learnerCopy).toMatch(/eighth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9810)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const { form } of targets) supportingPunjabi.delete(form);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 185`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch185-whose-book-whose-home.tex",
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
    "../../../../../../learning/human-languages/punjabi/narration/ch185.json",
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
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 185)).toMatchObject({
    chapter: 185,
    payoff: { lesson: "PA-C185-payoff", kind: "production", assesses: atoms },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-185-A2-POSSESSIVE-AGREEMENT",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-184-A2-NEARBY-TIME");
}, 120_000);
