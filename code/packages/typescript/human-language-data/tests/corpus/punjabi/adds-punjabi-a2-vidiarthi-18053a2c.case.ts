import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

it("opens Punjabi chapter 194 by ear with familiar identity words", () => {
  const opener = loadTrackLessons("punjabi").find(
    (lesson) => lesson.realization.lessonId === "PA-C194-identity-return",
  );
  expect(opener).toBeDefined();
  const [entry] = orderChapterLessons(lessonModalities([opener!]));
  expect(entry?.coreModality).toBe("voice");
  expect(unstartableCause([entry!])).toBeNull();
  expect(opener?.blocks.some((block) => block.markdown.includes("[YOU HEAR:")))
    .toBe(true);
});

it("adds a slow Punjabi A2 student-noun prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C194-identity-return",
    "PA-C194-vidiarthi",
    "PA-C194-vidiarthi-review",
    "PA-C194-student-frame",
    "PA-C194-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 194));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([10550, 10560, 10570, 10580, 10590]);

  const target = "ਵਿਦਿਆਰਥੀ";
  const atom = "PA-LEX-VIDIARTHI-194";
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
  expect(learnerCopy).toContain("ਉਹ ਵਿਦਿਆਰਥੀ ਹੈ");
  expect(learnerCopy).not.toContain("ਵਿਦਿਆਰਥੀਆਂ");
  expect(learnerCopy).toMatch(/gender-neutral|does not mark.*gender/is);
  expect(learnerCopy).toMatch(/vidiārthī/is);
  expect(learnerCopy).toMatch(/source history separate.*Punjabi pronunciation.*vidiārthī/is);
  expect(learnerCopy).toMatch(/borrowing from Sanskrit.*Keep the source history separate/is);
  expect(learnerCopy).toContain("https://en.wiktionary.org/wiki/");
  expect(learnerCopy).toContain("https://dic.learnpunjabi.org/default.aspx");
  expect(learnerCopy).toMatch(/learn the Punjabi form as one whole/is);
  expect(learnerCopy).not.toMatch(/root means|suffix means|combine these parts/is);
  expect(learnerCopy).not.toMatch(/YOU HEAR:\s*\*vidyārthī\*/is);
  expect(learnerCopy).toMatch(/seventeenth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 10550)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  supportingPunjabi.delete(target);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 194`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch194-he-or-she-is-a-student.tex",
    import.meta.url,
  ), "utf8");
  const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
  expect(ownerLabel).toBeGreaterThan(0);
  const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
  expect(ownerSection).toBeGreaterThan(0);
  expect(generatedBook.slice(0, ownerSection)).not.toContain(target);
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch194.json",
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
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 194)).toMatchObject({
    chapter: 194,
    payoff: {
      lesson: "PA-C194-payoff",
      kind: "production",
      assesses: [atom],
    },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-194-A2-VIDIARTHI",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "vocabulary" });
  expect(extension?.prerequisites).toContain("PA-EXT-193-A2-ORDINAL-E");
}, 120_000);
