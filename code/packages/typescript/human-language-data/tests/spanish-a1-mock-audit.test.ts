import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../src/loader.js";
import {
  buildSpanishA1MockAudit,
  runSpanishA1MockAudit,
} from "../src/spanish-a1-mock-audit-cli.js";

describe("Spanish A1 book-bounded mock audit", () => {
  it("pins the current whole-item residual and its reproducible credit policy", () => {
    const audit = buildSpanishA1MockAudit();
    expect(audit.objectiveFailed).toBe(0);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 25, listening: 25, objectiveFailed: 0 },
      { reading: 25, listening: 25, objectiveFailed: 0 },
    ]);
    expect(audit.missingObjectiveLexemes).toHaveLength(0);
    expect(audit.policy.citationFormCredits).toContain("llamarse");
  });

  it("keeps the committed report canonical and current", () => {
    expect(runSpanishA1MockAudit(["--check"], defaultCurriculumRoot())).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// The same audit, one rung lower. It is the same measurement with a different
// cut-off -- `lessonsUpToLevel` already took the level, so only the three places
// that spelled `a1` out loud had to change.
//
// The claim these assertions defend is the one a mock paper makes implicitly and
// almost never proves: EVERY OBJECTIVE ITEM IS ANSWERABLE FROM WHAT THE BOOK HAS
// TAUGHT BY THIS RUNG. A hand-written "every word here is taught" note in a
// paper's preamble is an author's recollection; this is a parse of the answer
// keys against the headword set.
//
// It caught nine real failures on the first run, and every one of them was a
// word my own prose search had cleared -- because that search read lesson
// BODIES and the corpus's definition of taught is the HEADWORD set. `buenos
// días`, `la casa`, `el día`, `estoy` and `¿cómo te llamas?` all appear in
// Spanish pre-A1 lessons and none of them is taught there. Five items had to be
// rewritten rather than re-annotated.
// ---------------------------------------------------------------------------
describe("Spanish pre-A1 book-bounded mock audit", () => {
  it("proves every objective item on both forms is answerable from pre-A1 headwords", () => {
    const audit = buildSpanishA1MockAudit(defaultCurriculumRoot(), "pre-A1");
    expect(audit.level).toBe("pre-A1");
    expect(audit.objectiveFailed).toBe(0);
    expect(audit.missingObjectiveLexemes).toHaveLength(0);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 10, listening: 10, objectiveFailed: 0 },
      { reading: 10, listening: 10, objectiveFailed: 0 },
    ]);
  });

  it("measures a SMALLER taught set than A1, which is what makes it a different gate", () => {
    const root = defaultCurriculumRoot();
    const preA1 = buildSpanishA1MockAudit(root, "pre-A1");
    const a1 = buildSpanishA1MockAudit(root, "A1");

    // If the level argument were ignored, both would measure the same corpus and
    // the pre-A1 audit would be a second copy of the A1 one wearing a new name.
    // This is the assertion that the cut-off is real.
    expect(preA1.lessonCount).toBeLessThan(a1.lessonCount);
    expect(preA1.taughtForms).toBeLessThan(a1.taughtForms);
  });

  it("keeps the committed report canonical and current", () => {
    expect(runSpanishA1MockAudit(["--check", "--level", "pre-A1"], defaultCurriculumRoot())).toBe(0);
  });
});
