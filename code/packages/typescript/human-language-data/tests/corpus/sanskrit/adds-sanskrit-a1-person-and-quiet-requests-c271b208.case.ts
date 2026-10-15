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

it("asks for a person and requests quiet twice in Sanskrit A1", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C271-ask-for-a-person",
    "SA-C271-request-quiet",
    "SA-C271-spoken-requests-first-retrieval",
    "SA-C271-spoken-requests-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    271, 271, 271, 271,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    16020, 16030, 16040, 16050,
  ]);

  const personAtom = "SA-PATTERN-C271-ASK-FOR-PERSON-01";
  const quietAtom = "SA-PATTERN-C271-REQUEST-QUIET-02";
  const pointAtoms = [personAtom, quietAtom];
  const componentAtoms = [
    "SA-LEX-C14-DEIXIS-03",
    "SA-LEX-ASTI",
    "SA-LEX-C83-SCHOOL-03",
    "SA-LEX-AM-YES",
    "SA-LEX-C29-COURTESY-01",
  ];

  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }
  expect(chapterLessons[0]?.frontmatter["introduces.knowledge"]).toEqual([personAtom]);
  expect(chapterLessons[1]?.frontmatter["introduces.knowledge"]).toEqual([quietAtom]);
  expect(chapterLessons[2]?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(chapterLessons[3]?.frontmatter["introduces.knowledge"]).toEqual([]);
  for (const atom of pointAtoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }
  for (const atom of componentAtoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const personLesson = chapterLessons[0];
  expect(personLesson?.body).toContain("At a doorway or on a call");
  expect(personLesson?.body).toContain("अध्यापकः अत्र अस्ति?");
  expect(personLesson?.body).toContain("आम्, अध्यापकः अत्र अस्ति।");
  expect(personLesson?.body).toContain("Is the teacher here?");
  expect(personLesson?.body).toContain("Yes, the teacher is here.");
  expect(personLesson?.body).toMatch(/does not claim a general polar-question system/u);

  const quietLesson = chapterLessons[1];
  expect(quietLesson?.body).toContain(
    "https://bseh.org.in/uploads/ebooks/20_Class_10th_Shemushi_Sansktit_120pp.pdf",
  );
  expect(quietLesson?.body).toContain(
    "https://sanskritdocuments.org/learning_tools/ashtadhyayi/vyakhya/3/3.4.63.htm",
  );
  expect(quietLesson?.body).toMatch(/does not teach the\s+imperative paradigm/u);
  expect(quietLesson?.body).toMatch(
    /The sources attest the command \*\*तूष्णीं भव\*\*\s+and its construction/u,
  );
  expect(quietLesson?.body).toMatch(
    /request transparently combines that\s+command with already-owned \*\*कृपया\*\*/u,
  );
  expect(quietLesson?.body).not.toMatch(/complete sourced request/u);

  const retrievalLessons = chapterLessons.slice(2);
  for (const lesson of retrievalLessons) {
    expect(lesson?.frontmatter["requires.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
    expect(lesson?.frontmatter["practises.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
    const responseIndex = lesson?.blocks.findIndex((block) =>
      block.markdown.includes("No Sanskrit model is shown before you answer"),
    ) ?? -1;
    const checkIndex = lesson?.blocks.findIndex((block) =>
      block.markdown.includes("Now check:"),
    ) ?? -1;
    expect(responseIndex).toBeGreaterThanOrEqual(0);
    expect(checkIndex).toBeGreaterThan(responseIndex);
    const responseBlock = lesson?.blocks[responseIndex];
    const checkBlock = lesson?.blocks[checkIndex];
    expect(responseBlock?.markdown).toContain("[PAUSE 8s]");
    expect(responseBlock?.markdown.indexOf("Say the Sanskrit")).toBeLessThan(
      responseBlock?.markdown.indexOf("[PAUSE 8s]") ?? -1,
    );
    const authoredPrefix = lesson?.blocks
      .slice(0, checkIndex)
      .map((block) => block.markdown)
      .join(" ") ?? "";
    expect(authoredPrefix).not.toMatch(
      /अध्यापकः अत्र अस्ति|आम्, अध्यापकः अत्र अस्ति|कृपया तूष्णीं भव|adhyāpakaḥ atra asti|ām, adhyāpakaḥ atra asti|kṛpayā tūṣṇīṃ bhava/u,
    );
    expect(checkBlock?.markdown).toContain("अध्यापकः अत्र अस्ति?");
    expect(checkBlock?.markdown).toContain("आम्, अध्यापकः अत्र अस्ति।");
    expect(checkBlock?.markdown).toContain("कृपया तूष्णीं भव");
    expect(checkBlock?.markdown).toMatch(/adhyāpakaḥ\s+atra asti\?/u);
    expect(checkBlock?.markdown).toMatch(/ām, adhyāpakaḥ\s+atra asti/u);
    expect(checkBlock?.markdown).toContain("kṛpayā tūṣṇīṃ bhava");
  }

  const narrationPath = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/sanskrit/narration/ch271.json",
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
  for (const id of ids.slice(2)) {
    const spoken = narration.lessons.find((lesson) => lesson.id === id);
    const guidedIndex = spoken?.blocks.findIndex(
      (block) => block.type === "guided-production",
    ) ?? -1;
    const checkIndex = spoken?.blocks.findIndex((block) => block.type === "notice") ?? -1;
    expect(guidedIndex).toBeGreaterThanOrEqual(0);
    expect(checkIndex).toBeGreaterThan(guidedIndex);
    const guided = spoken?.blocks[guidedIndex];
    const check = spoken?.blocks[checkIndex];
    expect(guided?.segments.map((segment) => segment.kind)).toEqual([
      "speech",
      "speech",
      "pause",
    ]);
    expect(guided?.segments[1]?.text).toContain("Say the Sanskrit");
    expect(guided?.segments[2]).toMatchObject({ kind: "pause", seconds: 8 });
    const spokenPrefix = spoken?.blocks
      .slice(0, checkIndex)
      .flatMap((block) => block.segments)
      .map((segment) => segment.text ?? "")
      .join(" ") ?? "";
    expect(spokenPrefix).not.toMatch(
      /अध्यापकः अत्र अस्ति|आम्, अध्यापकः अत्र अस्ति|कृपया तूष्णीं भव|adhyāpakaḥ atra asti|ām, adhyāpakaḥ atra asti|kṛpayā tūṣṇīṃ bhava/u,
    );
    expect(check?.segments[0]?.text).toMatch(
      /अध्यापकः अत्र अस्ति.*आम्, अध्यापकः अत्र अस्ति.*कृपया तूष्णीं भव|कृपया तूष्णीं भव.*अध्यापकः अत्र अस्ति.*आम्, अध्यापकः अत्र अस्ति/u,
    );
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);
  expect(learnerCopy).toContain(
    "not evidence of general imperative mastery, conversational competence, human validation, learner attainment, or exam readiness",
  );

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 271)).toMatchObject({
    chapter: 271,
    payoff: {
      lesson: "SA-C271-spoken-requests-final-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-F-22")?.probe).toEqual(
    pointAtoms,
  );
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    covered: 155,
    unmapped: 9,
    partial: 0,
  });
  expect(coverage.byCategory["Structuring the exchange"]).toEqual({
    enumerated: 2,
    covered: 2,
  });
  expect(coverage.points.find((point) => point.id === "SA-A1-F-22")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
