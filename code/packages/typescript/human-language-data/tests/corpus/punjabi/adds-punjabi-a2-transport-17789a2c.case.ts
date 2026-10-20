import { expect, it } from "vitest";
import {
  loadTrackChapters,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a bounded Punjabi A2 transport prerequisite without mock credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = [
    "PA-C181-rel",
    "PA-C181-ticket",
    "PA-C181-yatra",
    "PA-C181-yayya",
    "PA-C181-vapas",
    "PA-C181-paidal",
    "PA-C181-travel-review",
    "PA-C181-payoff",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter)))
    .toEqual(ids.map(() => 181));
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence)))
    .toEqual([9470, 9480, 9490, 9500, 9510, 9520, 9530, 9540]);

  const atoms = [
    "PA-TRAVEL-RAIL-01",
    "PA-TRAVEL-TICKET-01",
    "PA-TRAVEL-JOURNEY-01",
    "PA-TRAVEL-BACK-01",
    "PA-TRAVEL-ON-FOOT-01",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"]))
    .toEqual([
      [atoms[0]],
      [atoms[1]],
      [atoms[2]],
      ["PA-SCRIPT-RECOG-YA-01"],
      [atoms[3]],
      [atoms[4]],
      [],
      [],
    ]);
  const newForms = ["ਰੇਲ", "ਟਿਕਟ", "ਯਾਤਰਾ", "ਵਾਪਸ", "ਪੈਦਲ"];
  const ownerIndexes = [0, 1, 2, 4, 5];
  for (const [formIndex, form] of newForms.entries()) {
    const ownerIndex = ownerIndexes[formIndex]!;
    const earlierCopy = chapterLessons.slice(0, ownerIndex)
      .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
      .join("\n");
    expect(earlierCopy.includes(form), `${form} appears before its owner lesson`)
      .toBe(false);
  }
  const yatra = chapterLessons[2]!;
  expect(yatra.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(yatra.blocks.map((block) => block.markdown).join("\n"))
    .not.toContain("[YOU READ:");
  const yayya = chapterLessons[3]!;
  expect(yayya.frontmatter.headword).toBe("ਯ");
  expect(yayya.frontmatter.delivery).toBe("script");
  expect(yayya.blocks.map((block) => block.markdown).join("\n"))
    .toMatch(/No stroke order is taught here/i);
  expect(yayya.frontmatter["requires.knowledge"])
    .toContain("PA-TRAVEL-JOURNEY-01");
  expect(chapterLessons[4]?.frontmatter["requires.knowledge"])
    .toContain("PA-SCRIPT-RECOG-YA-01");
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
  const payoffText = "ਰੇਲ ਦੀ ਟਿਕਟ ਇੱਥੇ ਹੈ। ਮੈਨੂੰ ਰੇਲ ਨਾਲ ਯਾਤਰਾ ਪਸੰਦ ਹੈ। " +
    "ਮੈਨੂੰ ਪੈਦਲ ਵਾਪਸ ਘਰ ਜਾਣਾ ਪਸੰਦ ਹੈ।";
  for (const form of [...newForms, payoffText]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toMatch(/fourth prerequisite tranche|does not unblock.*mock 1/is);
  expect(learnerCopy).toMatch(/22 measured unsupported occurrences/is);
  expect(learnerCopy).toMatch(/not.*A2 mock|not A2 readiness/is);

  const taughtBefore = lessons
    .filter((lesson) => Number(lesson.frontmatter.sequence) < 9470)
    .flatMap((lesson) => [
      String(lesson.frontmatter.headword ?? ""),
      ...lesson.blocks.map((block) => block.markdown),
    ])
    .join("\n");
  const supportingPunjabi = new Set(learnerCopy.match(/[\u0A00-\u0A7F]+/gu) ?? []);
  for (const newlyTaught of [...newForms, "ਯ"]) supportingPunjabi.delete(newlyTaught);
  for (const token of supportingPunjabi) {
    expect(taughtBefore.includes(token), `${token} must occur before chapter 181`).toBe(true);
  }

  const payoff = chapterLessons.at(-1);
  expect(payoff?.frontmatter.skills)
    .toEqual(["listening", "speaking", "reading", "writing"]);
  expect(payoff?.blocks.some((block) => block.markdown.includes("[YOU WRITE:")))
    .toBe(true);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 181)).toMatchObject({
    chapter: 181,
    payoff: { lesson: "PA-C181-payoff", kind: "production", assesses: atoms },
  });
  const curriculum = loadTrackLanguageCurriculum("punjabi");
  const extension = curriculum?.extensions.find((candidate) =>
    candidate.id === "PA-EXT-181-A2-TRANSPORT",
  );
  expect(extension).toMatchObject({ stage: "A2", kind: "required", category: "vocabulary" });
  expect(extension?.prerequisites).toContain("PA-EXT-180-A2-PLACE-REFERENCE");
}, 120_000);
