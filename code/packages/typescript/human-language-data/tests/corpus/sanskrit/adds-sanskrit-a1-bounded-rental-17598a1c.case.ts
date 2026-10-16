import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import {
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";
import { sanskritA1PointCoverage } from "../sanskrit-a1-point-coverage.js";

it("teaches one bounded cross-checked Sanskrit rental exchange twice", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C274-meet-one-rental-exchange",
    "SA-C274-rental-first-retrieval",
    "SA-C274-rental-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    274, 274, 274,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    16120, 16130, 16140,
  ]);

  const rentalAtom = "SA-PATTERN-C274-RENTAL-01";
  const knownFieldAtoms = [
    "SA-LEX-C16-PLACE-01",
    "SA-VERB-C266-KRINAMI-01",
    "SA-LEX-C72-HOUSEWORK-01",
    "SA-LEX-C72-HOUSEWORK-03",
    "SA-LEX-C136-ACT136-04",
  ];
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }
  expect(chapterLessons[0]?.frontmatter["introduces.knowledge"]).toEqual([
    rentalAtom,
  ]);
  expect(chapterLessons[1]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(chapterLessons[2]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(rentalAtom),
    ),
  ).toHaveLength(1);
  for (const atom of knownFieldAtoms) {
    expect(
      lessons.filter((lesson) =>
        lesson.frontmatter["introduces.knowledge"].includes(atom),
      ),
    ).toHaveLength(1);
  }

  const teachingLesson = chapterLessons[0];
  for (const atom of knownFieldAtoms) {
    expect(teachingLesson?.frontmatter["requires.knowledge"]).toContain(atom);
  }
  expect(teachingLesson?.body).toContain("किं तत् भाटकितं गृहम्?");
  expect(teachingLesson?.body).toContain("न, अहं भाटकं न ददामि।");
  expect(teachingLesson?.body).toContain("kiṃ tat bhāṭakitaṃ gṛham");
  expect(teachingLesson?.body).toContain("na, ahaṃ bhāṭakaṃ na dadāmi");
  expect(teachingLesson?.body).toContain(
    "https://sanskritkalp.com/samskrit-vaartaalaap/kim",
  );
  expect(teachingLesson?.body).toContain(
    "https://sanskritdocuments.org/doc_z_misc_major_works/vyavahAraprashnottarI.html",
  );
  expect(teachingLesson?.body).toContain(
    "https://www.cstt.education.gov.in/sites/default/files/fundamintal-administrative-terminology-english-sanskrit.pdf",
  );
  expect(teachingLesson?.body).toMatch(/complete attested exchange/u);
  expect(teachingLesson?.body).toMatch(/house rent/u);
  expect(teachingLesson?.body).toMatch(/known house, buying, and household-work field/u);
  expect(teachingLesson?.body).toMatch(/apparent dropped syllable/u);

  for (const lesson of chapterLessons.slice(1)) {
    expect(lesson?.frontmatter["requires.knowledge"]).toContain(rentalAtom);
    expect(lesson?.frontmatter["practises.knowledge"]).toContain(rentalAtom);
    const responseIndex =
      lesson?.blocks.findIndex((block) =>
        block.markdown.includes("No Sanskrit model is shown before you answer"),
      ) ?? -1;
    const checkIndex =
      lesson?.blocks.findIndex((block) => block.markdown.includes("Now check:")) ?? -1;
    expect(responseIndex).toBeGreaterThanOrEqual(0);
    expect(checkIndex).toBeGreaterThan(responseIndex);
    const responseBlock = lesson?.blocks[responseIndex];
    const checkBlock = lesson?.blocks[checkIndex];
    expect(responseBlock?.markdown).toContain("[PAUSE 8s]");
    expect(responseBlock?.markdown.indexOf("Say the Sanskrit rental exchange")).toBeLessThan(
      responseBlock?.markdown.indexOf("[PAUSE 8s]") ?? -1,
    );
    const authoredPrefix =
      lesson?.blocks
        .slice(0, checkIndex)
        .map((block) => block.markdown)
        .join(" ") ?? "";
    expect(authoredPrefix).not.toMatch(
      /किं तत् भाटकितं गृहम्|न, अहं भाटकं न ददामि|kiṃ tat bhāṭakitaṃ gṛham|na, ahaṃ bhāṭakaṃ na dadāmi/u,
    );
    expect(checkBlock?.markdown).toContain("किं तत् भाटकितं गृहम्?");
    expect(checkBlock?.markdown).toContain("न, अहं भाटकं न ददामि।");
    expect(checkBlock?.markdown).toMatch(/kiṃ tat bhāṭakitaṃ gṛham/u);
    expect(checkBlock?.markdown).toMatch(/na, ahaṃ bhāṭakaṃ na dadāmi/u);
  }

  const narrationPath = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/sanskrit/narration/ch274.json",
  );
  const narration = JSON.parse(readFileSync(narrationPath, "utf8")) as {
    lessons: Array<{
      id: string;
      blocks: Array<{
        type: string;
        segments: Array<{ kind: string; text?: string; seconds?: number }>;
      }>;
    }>;
  };
  for (const id of ids.slice(1)) {
    const spoken = narration.lessons.find((lesson) => lesson.id === id);
    const guidedIndex =
      spoken?.blocks.findIndex((block) => block.type === "guided-production") ?? -1;
    const checkIndex = spoken?.blocks.findIndex((block) => block.type === "notice") ?? -1;
    expect(guidedIndex).toBeGreaterThanOrEqual(0);
    expect(checkIndex).toBeGreaterThan(guidedIndex);
    const guided = spoken?.blocks[guidedIndex];
    expect(guided?.segments.map((segment) => segment.kind)).toEqual([
      "speech",
      "speech",
      "pause",
    ]);
    expect(guided?.segments[1]?.text).toContain("Say the Sanskrit rental exchange");
    expect(guided?.segments[2]).toMatchObject({ kind: "pause", seconds: 8 });
    const spokenPrefix =
      spoken?.blocks
        .slice(0, checkIndex)
        .flatMap((block) => block.segments)
        .filter((segment) => segment.kind === "speech")
        .map((segment) => segment.text ?? "")
        .join(" ") ?? "";
    expect(spokenPrefix).not.toMatch(
      /किं तत् भाटकितं गृहम्|न, अहं भाटकं न ददामि|kiṃ tat bhāṭakitaṃ gṛham|na, ahaṃ bhāṭakaṃ na dadāmi/u,
    );
    const spokenCheck =
      spoken?.blocks[checkIndex]?.segments
        .filter((segment) => segment.kind === "speech")
        .map((segment) => segment.text ?? "")
        .join(" ") ?? "";
    expect(spokenCheck).toContain("किं तत् भाटकितं गृहम्?");
    expect(spokenCheck).toContain("न, अहं भाटकं न ददामि।");
    expect(spokenCheck).toMatch(/kiṃ tat bhāṭakitaṃ gṛham/u);
    expect(spokenCheck).toMatch(/na, ahaṃ bhāṭakaṃ na dadāmi/u);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);
  expect(learnerCopy).toContain(
    "not evidence of productive participle, case, or sandhi mastery, a general renting repertoire, housing competence, human validation, learner attainment, or exam readiness",
  );

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 274)).toMatchObject({
    chapter: 274,
    payoff: {
      lesson: "SA-C274-rental-final-retrieval",
      kind: "production",
      assesses: [rentalAtom],
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-07")?.probe).toEqual([
    rentalAtom,
  ]);
  expect(sanskritA1PointCoverage("SA-A1-M-07")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
