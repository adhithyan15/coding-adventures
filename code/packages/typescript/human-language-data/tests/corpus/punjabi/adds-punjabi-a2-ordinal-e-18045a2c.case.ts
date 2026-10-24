import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

it("opens Punjabi chapter 193 by ear while retrieving first and second", () => {
  const opener = loadTrackLessons("punjabi").find(
    (lesson) => lesson.realization.lessonId === "PA-C193-ordinal-return",
  );
  expect(opener).toBeDefined();
  const [entry] = orderChapterLessons(lessonModalities([opener!]));
  expect(entry?.coreModality).toBe("voice");
  expect(unstartableCause([entry!])).toBeNull();
  expect(opener?.blocks.some((block) => block.markdown.includes("[YOU HEAR:")))
    .toBe(true);
});

it("adds a slow Punjabi A2 ordinal e prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C193-ordinal-return",
    "PA-C193-pahile",
    "PA-C193-pahile-review",
    "PA-C193-duje",
    "PA-C193-duje-review",
    "PA-C193-ordinal-contrast",
    "PA-C193-ordinal-frames",
    "PA-C193-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 193));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([10470, 10480, 10490, 10500, 10510, 10520, 10530, 10540]);

  const owners = [
    { target: "ਪਹਿਲੇ", atom: "PA-FORM-PAHILE-ORDINAL-193", ownerIndex: 1 },
    { target: "ਦੂਜੇ", atom: "PA-FORM-DUJE-ORDINAL-193", ownerIndex: 3 },
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([
      [],
      [owners[0].atom],
      [],
      [owners[1].atom],
      [],
      [],
      [],
      [],
    ]);

  for (const { target, atom, ownerIndex } of owners) {
    const earlierCopy = chapterLessons.slice(0, ownerIndex)
      .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
      .join("\n");
    expect(earlierCopy.includes(target), `${target} appears before its owner lesson`)
      .toBe(false);

    const rawBeforeOwner = ids.slice(0, ownerIndex).map((id) => {
      const source = readFileSync(new URL(
        `../../../../../../learning/human-languages/punjabi/lessons/${id}.md`,
        import.meta.url,
      ), "utf8");
      return source.replace(/^---[\s\S]*?---\s*/u, "");
    }).join("\n");
    expect(rawBeforeOwner.includes(target), `${target} leaks in raw authored copy before its owner`)
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
    expect(laterPractices.length).toBeGreaterThanOrEqual(2);
  }

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
  expect(learnerCopy).toContain("ਪਹਿਲਾ ਕਮਰਾ");
  expect(learnerCopy).toContain("ਪਹਿਲੇ ਕਮਰੇ ਵਿੱਚ");
  expect(learnerCopy).toContain("ਦੂਜਾ ਕਮਰਾ");
  expect(learnerCopy).toContain("ਦੂਜੇ ਕਮਰੇ ਵਿੱਚ");
  expect(learnerCopy).toMatch(/Gill.*Gleason.*masculine.*singular direct.*oblique.*ਪਹਿਲਾ.*ਪਹਿਲੇ/is);
  expect(learnerCopy).toMatch(/first.*ਪਹਿਲਾ.*second.*ਦੂਜਾ/is);
  expect(learnerCopy).toMatch(/two ordinals.*not.*every Punjabi adjective/is);
  expect(learnerCopy).toMatch(/sixteenth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 10470)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const { target } of owners) supportingPunjabi.delete(target);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 193`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch193-first-and-second-in-a-room.tex",
    import.meta.url,
  ), "utf8");
  for (const { target, ownerIndex } of owners) {
    const ownerLabel = generatedBook.indexOf(`label{lesson:${ids[ownerIndex]}}`);
    expect(ownerLabel).toBeGreaterThan(0);
    const ownerSection = generatedBook.lastIndexOf("\\section[", ownerLabel);
    expect(ownerSection).toBeGreaterThan(0);
    expect(generatedBook.slice(0, ownerSection)).not.toContain(target);
  }
  expect(generatedBook).not.toMatch(/\[(?:YOU HEAR|YOU SAY|YOU READ|YOU WRITE):/u);

  const narration = JSON.parse(readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/narration/ch193.json",
    import.meta.url,
  ), "utf8")) as { lessons: Array<{ id: string }> };
  expect(narration.lessons.map((lesson) => lesson.id)).toEqual(ids);
  for (const { target, ownerIndex } of owners) {
    expect(JSON.stringify(narration.lessons.slice(0, ownerIndex))).not.toContain(target);
  }

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 193)).toMatchObject({
    chapter: 193,
    payoff: {
      lesson: "PA-C193-payoff",
      kind: "production",
      assesses: owners.map(({ atom }) => atom),
    },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-193-A2-ORDINAL-E",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-192-A2-LOKAAN-OBLIQUE");
}, 120_000);
