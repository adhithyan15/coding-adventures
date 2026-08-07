// HL08's gentle-ramp budgets, measured. See src/ramp.ts for why this did not exist.

import { describe, expect, it } from "vitest";
import { loadChapterPolicy, loadEverything } from "../src/loader.js";
import { measureRamp } from "../src/ramp.js";
import { parseLesson } from "../src/parse.js";
import type { ChapterPolicy } from "../src/types.js";

const POLICY = {
  version: 1,
  payoffRepresentativeness: 0.5,
  maxNewAtomsPerLesson: 3,
  maxNewAtomsPerChapter: 12,
  maxLinearisableTableColumns: 3,
} as ChapterPolicy;

function lesson(id: string, chapter: number, atoms: string[]) {
  const directive = `<!-- hl-knowledge: introduces=[${atoms.join(", ")}]; assesses=[] -->\n\n`;
  return parseLesson(
    `---\nschema_version: 2\nid: ${id}\nchapter: ${chapter}\ntype: word\n` +
      `headword: x\ngloss: x\nconcept_tag: GREETING-HELLO\n---\n\n# ${id}\n\n` +
      `## Warm-up\n\n${directive}Say it.\n`,
    "spanish",
  );
}

describe("the lesson budget", () => {
  it("flags a lesson above the budget and leaves one at it alone", () => {
    const report = measureRamp(
      [lesson("ES-1", 1, ["A", "B", "C", "D"]), lesson("ES-2", 1, ["E", "F", "G"])],
      POLICY,
    );
    expect(report.lessons.map((v) => v.lessonId)).toEqual(["ES-1"]);
    // Exactly at the budget is compliant — it is a maximum, not a target to stay under.
    expect(report.summary.lessonViolations).toBe(1);
  });

  it("orders violations steepest first, so the list is a work queue", () => {
    const report = measureRamp(
      [lesson("ES-1", 1, ["A", "B", "C", "D"]), lesson("ES-2", 2, ["E", "F", "G", "H", "I"])],
      POLICY,
    );
    expect(report.lessons.map((v) => v.atoms)).toEqual([5, 4]);
    expect(report.summary.steepestLesson?.lessonId).toBe("ES-2");
  });
});

describe("the chapter budget", () => {
  it("catches a chapter that splitting alone would have hidden", () => {
    // Four compliant 3-atom lessons still put 12 new atoms in one chapter; a fifth
    // breaks the chapter budget even though no single lesson does. That is the whole
    // point of having both numbers — splitting must not be able to game the rule.
    const atoms = (n: number) => Array.from({ length: 3 }, (_, i) => `A${n}${i}`);
    const five = [1, 2, 3, 4, 5].map((n) => lesson(`ES-${n}`, 1, atoms(n)));
    const report = measureRamp(five, POLICY);
    expect(report.summary.lessonViolations).toBe(0);
    expect(report.summary.chapterViolations).toBe(1);
    expect(report.chapters[0]).toMatchObject({ chapter: 1, atoms: 15, lessonCount: 5 });
  });
});

describe("what the measurement cannot see", () => {
  it("reports atom-less lessons as unmeasurable, never as compliant", () => {
    // A schema-v1 lesson declares no atoms. Counting it as 0-and-therefore-fine would
    // let an unmigrated track look perfectly gentle.
    const report = measureRamp([lesson("ES-1", 1, []), lesson("ES-2", 1, ["A", "B", "C", "D"])], POLICY);
    expect(report.summary.unmeasurableLessons).toBe(1);
    expect(report.summary.measurablePercent).toBe(50);
    expect(report.summary.lessonViolations).toBe(1);
  });
});

describe("corpus snapshot", () => {
  // The first reproducible measurement of the gentle ramp. The quoted "52 over-budget
  // lessons" was an ad-hoc count no test reproduced, and it could not be reproduced
  // because the answer depends on how much of the corpus is schema-v2 that day.
  it("pins the ramp, and the size of its blind spot", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy());

    expect(report.policy).toEqual({ maxNewAtomsPerLesson: 3, maxNewAtomsPerChapter: 12 });
    expect(report.summary.lessonViolations).toBe(40);
    expect(report.summary.chapterViolations).toBe(25);

    // HALF THE CORPUS IS INVISIBLE HERE. 572 lessons declare no atoms, so they are
    // neither compliant nor violating — they are unmigrated. A track with few violations
    // and many unmeasurable lessons has not proved it is gentle. Ratchet this DOWN as
    // schema-v2 migration lands; the violation count will rise as it does, and that is
    // the measurement improving rather than the corpus worsening.
    expect(report.summary.unmeasurableLessons).toBe(572);
    expect(report.summary.measurablePercent).toBe(53);
  });

  it("names the steepest lesson, which is where a burn-down starts", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy());
    expect(report.summary.steepestLesson).toMatchObject({ atoms: 6, budget: 3 });
  });
});
