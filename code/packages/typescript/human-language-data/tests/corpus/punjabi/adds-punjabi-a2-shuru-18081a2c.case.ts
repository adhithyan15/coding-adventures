import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

const ids = [
  "PA-C196-work-command-return",
  "PA-C196-shuru",
  "PA-C196-shuru-review",
  "PA-C196-start-work-frame",
  "PA-C196-payoff",
];

it("opens Punjabi chapter 196 by ear without leaking the new word", () => {
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

it("adds a slow Punjabi A2 start-word prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 196));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([10670, 10680, 10690, 10700, 10710]);

  const target = "ਸ਼ੁਰੂ";
  const atom = "PA-LEX-SHURU-196";
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
  expect(learnerCopy).toContain("ਕੰਮ ਸ਼ੁਰੂ ਕਰੋ");
  expect(learnerCopy).toMatch(/noun.*beginning|noun.*start/is);
  expect(learnerCopy).toMatch(/ਕਰੋ.*polite command|polite command.*ਕਰੋ/is);
  expect(learnerCopy).toMatch(/noun-plus-.*ਕਰਨਾ.*pattern/is);
  expect(learnerCopy).toMatch(/keep.*whole|whole word/is);
  expect(learnerCopy).toContain("https://dic.learnpunjabi.org/default.aspx");
  expect(learnerCopy).toMatch(/Arabic.*through Persian.*Urdu|Arabic-origin.*Persian.*Urdu/is);
  expect(learnerCopy).toMatch(/source history separate.*Punjabi use/is);
  expect(learnerCopy).toMatch(/Gill.*Gleason.*pp\. 122.*124/is);
  expect(learnerCopy).toMatch(/not every possible combination.*tense.*function/is);
  expect(learnerCopy).not.toContain("ਸ਼ੁਰੂਆਤ");
  expect(learnerCopy).not.toContain("ਸ਼ੁਰੂ ਵਿੱਚ");
  expect(learnerCopy).not.toContain("ਸ਼ੁਰੂ ਹੋਣਾ");
  expect(learnerCopy).not.toMatch(/root means|prefix means|suffix means|combine these parts/is);
  expect(learnerCopy).toMatch(/nineteenth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 10670)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  supportingPunjabi.delete(target);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 196`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch196-start-the-work.tex",
    import.meta.url,
  ), "utf8");
  const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
  expect(ownerLabel).toBeGreaterThan(0);
  const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
  expect(ownerSection).toBeGreaterThan(0);
  expect(generatedBook.slice(0, ownerSection)).not.toContain(target);
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch196.json",
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
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 196)).toMatchObject({
    chapter: 196,
    payoff: {
      lesson: "PA-C196-payoff",
      kind: "production",
      assesses: [atom],
    },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-196-A2-SHURU",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "vocabulary" });
  expect(extension?.prerequisites).toContain("PA-EXT-195-A2-VALA-FORMS");
}, 120_000);
