import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("offers one bounded contemporary Sanskrit condolence twice", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C272-offer-condolence",
    "SA-C272-condolence-first-retrieval",
    "SA-C272-condolence-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    272, 272, 272,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    16060, 16070, 16080,
  ]);

  const condolenceAtom = "SA-PATTERN-C272-CONDOLENCE-01";
  const knownAtoms = [
    "SA-LEX-MAMA",
    "SA-LEX-C182-WEL182-02",
    "SA-LEX-C239-WEL239-04",
  ];
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }
  expect(chapterLessons[0]?.frontmatter["introduces.knowledge"]).toEqual([
    condolenceAtom,
  ]);
  expect(chapterLessons[1]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(chapterLessons[2]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    lessons.filter((lesson) =>
      lesson.frontmatter["introduces.knowledge"].includes(condolenceAtom),
    ),
  ).toHaveLength(1);
  for (const atom of knownAtoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const teachingLesson = chapterLessons[0];
  expect(teachingLesson?.body).toContain(
    "मम संवेदनाः शोकसन्तप्त-परिजनैः सह सन्ति।",
  );
  expect(teachingLesson?.body).toContain(
    "mama saṃvedanāḥ śokasantapta-parijanaiḥ saha santi",
  );
  expect(teachingLesson?.body).toContain(
    "https://sanskrit.hindusthansamachar.in/Encyc/2026/5/28/Chief-Minister-Expresses-Grief-Demise-Poet-Dr-Bash.php",
  );
  expect(teachingLesson?.body).toMatch(/quotes the same complete sentence/u);
  expect(teachingLesson?.body).toMatch(/normalizes that one character to the Devanagari visarga/u);
  expect(teachingLesson?.body).toMatch(/condolence to the bereaved family/u);
  expect(teachingLesson?.body).toMatch(/tribute to the person who died/u);
  expect(teachingLesson?.body).toMatch(/one whole social formula/u);
  expect(teachingLesson?.body).not.toMatch(/build any new condolence sentence/u);

  for (const lesson of chapterLessons.slice(1)) {
    expect(lesson?.frontmatter["requires.knowledge"]).toContain(condolenceAtom);
    expect(lesson?.frontmatter["practises.knowledge"]).toContain(condolenceAtom);
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
    expect(responseBlock?.markdown.indexOf("Say the Sanskrit")).toBeLessThan(
      responseBlock?.markdown.indexOf("[PAUSE 8s]") ?? -1,
    );
    const authoredPrefix =
      lesson?.blocks
        .slice(0, checkIndex)
        .map((block) => block.markdown)
        .join(" ") ?? "";
    expect(authoredPrefix).not.toMatch(
      /मम संवेदनाः शोकसन्तप्त-परिजनैः सह सन्ति|mama saṃvedanāḥ śokasantapta-parijanaiḥ saha santi/u,
    );
    expect(checkBlock?.markdown).toContain(
      "मम संवेदनाः शोकसन्तप्त-परिजनैः सह सन्ति।",
    );
    expect(checkBlock?.markdown).toMatch(
      /mama saṃvedanāḥ\s+śokasantapta-parijanaiḥ saha santi/u,
    );
  }

  const narrationPath = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/sanskrit/narration/ch272.json",
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
    expect(guided?.segments[1]?.text).toContain("Say the Sanskrit");
    expect(guided?.segments[2]).toMatchObject({ kind: "pause", seconds: 8 });
    const spokenPrefix =
      spoken?.blocks
        .slice(0, checkIndex)
        .flatMap((block) => block.segments)
        .filter((segment) => segment.kind === "speech")
        .map((segment) => segment.text ?? "")
        .join(" ") ?? "";
    expect(spokenPrefix).not.toMatch(
      /मम संवेदनाः शोकसन्तप्त-परिजनैः सह सन्ति|mama saṃvedanāḥ\s+śokasantapta-parijanaiḥ saha santi/u,
    );
    const spokenCheck =
      spoken?.blocks[checkIndex]?.segments
        .filter((segment) => segment.kind === "speech")
        .map((segment) => segment.text ?? "")
        .join(" ") ?? "";
    expect(spokenCheck).toContain("मम संवेदनाः शोकसन्तप्त-परिजनैः सह सन्ति।");
    expect(spokenCheck).toMatch(
      /mama saṃvedanāḥ\s+śokasantapta-parijanaiḥ saha santi/u,
    );
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);
  expect(learnerCopy).toContain(
    "not evidence of general social competence, human validation, learner attainment, or exam readiness",
  );

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 272)).toMatchObject({
    chapter: 272,
    payoff: {
      lesson: "SA-C272-condolence-final-retrieval",
      kind: "production",
      assesses: [condolenceAtom],
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-F-19")?.probe).toEqual([
    condolenceAtom,
  ]);
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    covered: 157,
    unmapped: 7,
    partial: 0,
  });
  expect(coverage.points.find((point) => point.id === "SA-A1-F-19")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
