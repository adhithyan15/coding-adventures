import { expect, it } from "vitest";
import {
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";
import { formatExamCoverage, measureExamCoverage } from "../../../src/exam-inventory.js";

it("adds one bounded Punjabi A1 pointing-and-place tranche without readiness credit", () => {
  const lessons = loadTrackLessons("punjabi");
  const ids = ["PA-C176-eh", "PA-C176-vich", "PA-C176-location-frame"];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );

  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    176,
    176,
    176,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    9200,
    9210,
    9220,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const eh = "PA-DEM-EH-01";
  const vich = "PA-POSTPOSITION-VICH-01";
  const locationFrame = "PA-GRAMMAR-EXISTENTIAL-LOCATION-01";
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [eh],
    [vich],
    [locationFrame],
  ]);
  for (const atom of [eh, vich, locationFrame]) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of ["ਇਹ", "ਉਹ", "ਵਿੱਚ", "ਕਿੱਥੇ", "ਇੱਥੇ", "ਉੱਥੇ", "ਹੈ"]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("ਕਿਤਾਬ ਘਰ ਵਿੱਚ ਹੈ");
  expect(learnerCopy).toContain("ਦੁਕਾਨ ਉੱਥੇ ਹੈ");
  expect(learnerCopy).toMatch(/not an A1 mock|not A1 readiness/i);

  const ledger = loadTrackChapters().find((track) => track.language === "punjabi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 176)).toMatchObject({
    chapter: 176,
    payoff: {
      lesson: "PA-C176-location-frame",
      kind: "production",
      assesses: [
        "PA-DEM-EH-01",
        "PA-PRON-OH-01",
        "PA-LEX-C89-PLACE89-03",
        "PA-LEX-C89-PLACE89-04",
        "PA-POSTPOSITION-VICH-01",
        "PA-LEX-HAI-02",
        "PA-GRAMMAR-EXISTENTIAL-LOCATION-01",
      ],
    },
  });

  const inventory = loadExamInventory("punjabi", "A1");
  expect(inventory.points.find((point) => point.id === "PA-A1-DEM-01")?.probe).toEqual([
    eh,
    "PA-PRON-OH-01",
  ]);
  for (const id of ["PA-A1-DEM-02", "PA-A1-ADV-05"]) {
    expect(inventory.points.find((point) => point.id === id)?.probe).toEqual([
      "PA-LEX-C89-PLACE89-03",
      "PA-LEX-C89-PLACE89-04",
    ]);
  }
  expect(inventory.points.find((point) => point.id === "PA-A1-CASE-04")?.probe).toEqual([
    vich,
  ]);
  expect(inventory.points.find((point) => point.id === "PA-A1-S-06")?.probe).toEqual([
    locationFrame,
  ]);

  const coverage = measureExamCoverage(inventory, lessons);
  expect(coverage.enumerated).toBe(227);
  expect(coverage.covered).toBe(142);
  expect(coverage.unmapped).toBe(85);
  expect(coverage.partial).toBe(0);
  for (const id of [
    "PA-A1-DEM-01",
    "PA-A1-DEM-02",
    "PA-A1-CASE-04",
    "PA-A1-ADV-05",
    "PA-A1-S-06",
  ]) {
    expect(coverage.points.find((point) => point.id === id)).toMatchObject({
      covered: true,
      missingAtoms: [],
      unmapped: false,
    });
  }
  expect(formatExamCoverage(coverage)).toContain(
    "punjabi A1 (partial inventory): 142/227 points covered (63%)",
  );
}, 120_000);
