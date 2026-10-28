import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

const ids = [
  "PA-C208-go-return",
  "PA-C208-pia",
  "PA-C208-pia-review",
  "PA-C208-had-to-go",
  "PA-C208-payoff",
];

it("opens Punjabi chapter 208 by ear without leaking the new necessity form", () => {
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

it("adds one bounded Punjabi A2 past-necessity prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 208));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([11280, 11290, 11300, 11310, 11320]);

  const target = "ਪਿਆ";
  const base = "ਪੈਣਾ";
  const atom = "PA-FORM-PIA-208";
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
  expect(rawBeforeOwner.includes(base), "base leaks in raw authored copy before its owner")
    .toBe(false);

  const directOwners = lessons.filter((lesson) =>
    lesson.frontmatter["introduces.knowledge"].includes(atom),
  );
  expect(directOwners).toHaveLength(1);
  expect(directOwners[0].frontmatter.headword).toContain(base);
  expect(directOwners[0].frontmatter.headword).toContain(target);
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
  expect(learnerCopy).toContain("ਮੈਨੂੰ ਜਾਣਾ ਪਿਆ");
  expect(learnerCopy).toMatch(/I had to go/is);
  expect(learnerCopy).toMatch(/ਪੈਣਾ.*ਪਿਆ|ਪਿਆ.*ਪੈਣਾ/us);
  expect(learnerCopy).toMatch(/masculine singular.*perfective|perfective.*masculine singular/is);
  expect(learnerCopy).toMatch(/keep.*whole|whole.*form/is);
  expect(learnerCopy).toContain(
    "https://pt.learnpunjabi.org/assets/A%20Reference%20Grammar_Final.pdf",
  );
  expect(learnerCopy).toContain(
    "https://www.aqa.org.uk/subjects/panjabi/a-level/panjabi-7682/specification/" +
      "subject-content/grammar",
  );
  expect(learnerCopy).not.toMatch(/ਪਈ|ਪਏ|ਪਈਆਂ|ਪੈਂਦਾ|ਪਵੇਗਾ/u);
  expect(learnerCopy).toMatch(/not.*full agreement paradigm/is);
  expect(learnerCopy).not.toMatch(/means? .*fall|means? .*lie/is);
  expect(learnerCopy).toMatch(/thirty-first prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 11280)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  for (const familiar of ["ਮੈਨੂੰ", "ਜਾਣਾ", "ਪੈਰ", "ਪਿਆਸ", "ਪਿਆਰ"]) {
    expect(taughtBefore.includes(familiar), `${familiar} must occur before chapter 208`)
      .toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch208-i-had-to-go.tex",
    import.meta.url,
  ), "utf8");
  const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
  expect(ownerLabel).toBeGreaterThan(0);
  const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
  expect(ownerSection).toBeGreaterThan(0);
  expect(generatedBook.slice(0, ownerSection)).not.toContain(target);
  expect(generatedBook.slice(0, ownerSection)).not.toContain(base);
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch208.json",
    import.meta.url,
  ), "utf8")) as { lessons: Array<{ id: string }> };
  expect(narration.lessons.map((lesson) => lesson.id)).toEqual(ids);
  expect(JSON.stringify(narration.lessons.slice(0, ownerIndex))).not.toContain(target);
  expect(JSON.stringify(narration.lessons.slice(0, ownerIndex))).not.toContain(base);

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 208)).toMatchObject({
    chapter: 208,
    payoff: {
      lesson: "PA-C208-payoff",
      kind: "production",
      assesses: [atom],
    },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-208-A2-PIA",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-207-A2-HUNDA");
}, 120_000);
