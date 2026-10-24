import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

const ids = [
  "PA-C195-window-room-return",
  "PA-C195-vala",
  "PA-C195-vala-review",
  "PA-C195-vale",
  "PA-C195-vale-review",
  "PA-C195-direct-oblique-contrast",
  "PA-C195-payoff",
];

it("opens Punjabi chapter 195 by ear without leaking either new form", () => {
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

it("adds a bounded Punjabi A2 vala direct-oblique prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 195));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([10600, 10610, 10620, 10630, 10640, 10650, 10660]);

  const owners = [
    { target: "ਵਾਲਾ", atom: "PA-GRAMMAR-VALA-ASSOCIATIVE-195", ownerIndex: 1 },
    { target: "ਵਾਲੇ", atom: "PA-FORM-VALE-MASC-OBLIQUE-195", ownerIndex: 3 },
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
    ]);

  for (const { target, atom, ownerIndex } of owners) {
    const rawBeforeOwner = ids.slice(0, ownerIndex).map((id) => {
      const source = readFileSync(new URL(
        `../../../../../../learning/human-languages/punjabi/lessons/${id}.md`,
        import.meta.url,
      ), "utf8");
      return source.replace(/^---[\s\S]*?---\s*/u, "");
    }).join("\n");
    expect(rawBeforeOwner.includes(target), `${target} leaks before its owner`).toBe(false);

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
    expect(lesson?.frontmatter["requires.knowledge"])
      .not.toContain("PA-LEX-C121-THINGS121-01");
    expect(lesson?.frontmatter["practises.knowledge"])
      .not.toContain("PA-LEX-C121-THINGS121-01");
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n").replace(/\s+/gu, " ");
  expect(learnerCopy).toContain("ਖਿੜਕੀ ਵਾਲਾ ਕਮਰਾ");
  expect(learnerCopy).toContain("ਖਿੜਕੀ ਵਾਲੇ ਕਮਰੇ ਵਿੱਚ");
  expect(learnerCopy).toMatch(/ਵਾਲੇ.*masculine singular oblique/is);
  expect(learnerCopy).toMatch(/noun.*ਵਾਲਾ.*adjective phrase/is);
  expect(learnerCopy).toMatch(/not.*plural.*honorific/is);
  expect(learnerCopy).toMatch(/not.*every noun.*postposition/is);
  expect(learnerCopy).not.toContain("ਵਾਲਿਆਂ");
  expect(learnerCopy).not.toMatch(/seller|occupation|about to|the ones|infinitive/is);
  expect(learnerCopy).toMatch(/earring.*separate lexical homograph/is);
  expect(learnerCopy).toContain("https://pt.learnpunjabi.org/assets/");
  expect(learnerCopy).toMatch(/Gill.*Gleason.*pp\. 71–72.*107/is);
  expect(learnerCopy).toMatch(/eighteenth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 10600)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const { target } of owners) supportingPunjabi.delete(target);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 195`).toBe(true);
  }

  const generatedBook = readFileSync(new URL(
    "../../../../../../learning/human-languages/punjabi/book/chapters/" +
      "ch195-a-room-with-a-window.tex",
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
    "../../../../../../learning/human-languages/punjabi/narration/ch195.json",
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
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 195)).toMatchObject({
    chapter: 195,
    payoff: {
      lesson: "PA-C195-payoff",
      kind: "production",
      assesses: owners.map(({ atom }) => atom),
    },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-195-A2-VALA-FORMS",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "grammar" });
  expect(extension?.prerequisites).toContain("PA-EXT-194-A2-VIDIARTHI");
}, 120_000);
