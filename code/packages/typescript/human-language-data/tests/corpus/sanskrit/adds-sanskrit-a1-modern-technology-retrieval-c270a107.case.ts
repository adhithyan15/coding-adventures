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

it("retrieves the two existing Sanskrit A1 modern-technology nouns twice", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C270-modern-technology-first-retrieval",
    "SA-C270-modern-technology-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    270, 270,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    16000, 16010,
  ]);

  const pointAtoms = ["SA-LEX-C105-THINGS105-02", "SA-LEX-C269-MEDIA-02"];
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
    expect(lesson?.frontmatter["introduces.knowledge"]).toEqual([]);
    expect(lesson?.frontmatter["requires.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
    expect(lesson?.frontmatter["practises.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
  }
  for (const atom of pointAtoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  for (const lesson of chapterLessons) {
    const responseBlock = lesson?.blocks.find((block) =>
      block.markdown.includes("No Sanskrit model is shown before you answer"),
    );
    const checkBlock = lesson?.blocks.find((block) =>
      block.markdown.includes("Now check:"),
    );
    expect(responseBlock).toBeDefined();
    expect(checkBlock).toBeDefined();
    expect(responseBlock?.markdown).toContain("[PAUSE 8s]");
    expect(responseBlock?.markdown.indexOf("Say the Sanskrit")).toBeLessThan(
      responseBlock?.markdown.indexOf("[PAUSE 8s]") ?? -1,
    );
    expect(responseBlock?.markdown).not.toMatch(
      /संगणकम्|अन्तर्जालम्|saṅgaṇakam|antarjālam/u,
    );
    expect(checkBlock?.markdown).toContain("संगणकम्");
    expect(checkBlock?.markdown).toContain("अन्तर्जालम्");
    expect(checkBlock?.markdown).toContain("saṅgaṇakam");
    expect(checkBlock?.markdown).toContain("antarjālam");
  }

  const narrationPath = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/sanskrit/narration/ch270.json",
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
  for (const id of ids) {
    const spoken = narration.lessons.find((lesson) => lesson.id === id);
    const guided = spoken?.blocks.find((block) => block.type === "guided-production");
    const check = spoken?.blocks.find((block) => block.type === "notice");
    expect(guided?.segments.map((segment) => segment.kind)).toEqual([
      "speech",
      "speech",
      "pause",
    ]);
    expect(guided?.segments[1]?.text).toContain("Say the Sanskrit");
    expect(guided?.segments[2]).toMatchObject({ kind: "pause", seconds: 8 });
    expect(guided?.segments.map((segment) => segment.text ?? "").join(" ")).not.toMatch(
      /संगणकम्|अन्तर्जालम्|saṅgaṇakam|antarjālam/u,
    );
    expect(check?.segments[0]?.text).toMatch(/संगणकम्.*अन्तर्जालम्|अन्तर्जालम्.*संगणकम्/u);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);
  expect(learnerCopy.match(/संगणकम्/gu)).toHaveLength(2);
  expect(learnerCopy.match(/अन्तर्जालम्/gu)).toHaveLength(2);
  expect(learnerCopy).toContain(
    "not evidence of device use, internet navigation, digital literacy, technological competence, human validation, learner attainment, or exam readiness",
  );

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 270)).toMatchObject({
    chapter: 270,
    payoff: {
      lesson: "SA-C270-modern-technology-final-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-13")?.probe).toEqual(
    pointAtoms,
  );
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    covered: 153,
    unmapped: 11,
    partial: 0,
  });
  expect(coverage.byCategory[
    "Modern life — fields a spoken revival needs and a classical corpus lacks"
  ]).toEqual({ enumerated: 15, covered: 11 });
  expect(coverage.points.find((point) => point.id === "SA-A1-M-13")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
