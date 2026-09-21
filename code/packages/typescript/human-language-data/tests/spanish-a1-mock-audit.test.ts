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

// ---------------------------------------------------------------------------
// The same audit, one rung HIGHER, and the first one that does not pass.
//
// A1 and pre-A1 both report zero: their mocks were written after the vocabulary
// existed, so the audit could only ever confirm what was already true. A2 is the
// other way round. The mocks were written FIRST, against the real DELE A2 shape,
// and the audit is what names the vocabulary still to be taught.
//
// So the number pinned below is a DEBT, not an achievement, and the gate is
// deliberately built to tolerate it: `--check` asserts the committed report is
// not stale, never that it is clean. Until the words land, this file is the
// repo's honest, machine-checked statement of how far Spanish A2 is from
// passable. Every vocabulary tranche should move these numbers DOWN, and the
// day they reach zero this block should read like the A1 one above.
//
// Why the mocks came first: choosing A2 words by theme was tried and failed --
// 27 of 35 candidates were already taught, because at ~817 headwords the obvious
// concrete domains are saturated. Deriving the list from the exam has no such
// waste, and it prioritises by what the paper actually demands.
// ---------------------------------------------------------------------------

describe("Spanish A2 book-bounded mock audit", () => {
  it("pins the CURRENT DEBT: the exam names the vocabulary that is still missing", () => {
    const audit = buildSpanishA1MockAudit(defaultCurriculumRoot(), "A2");
    expect(audit.level).toBe("A2");
    expect(audit.objectiveFailed).toBe(79);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 10, listening: 2, objectiveFailed: 38 },
      { reading: 1, listening: 8, objectiveFailed: 41 },
    ]);
    // THIS NUMBER MUST ONLY EVER FALL. A rise means a mock gained an item the
    // corpus cannot support.
    //
    // 191 -> 161 is the first vocabulary tranche (chapters 431-436) and
    // 161 -> 146 is the second (437-439). BOTH drops are exact: 30 headwords
    // removed 30 lexemes, then 15 removed 15. That arithmetic is the evidence a
    // word was genuinely absent -- one already taught under another name would
    // have made the drop smaller. The second drop was PREDICTED from the audit
    // before the chapters were wired, and the generator reproduced 79 and 146
    // exactly, so the selection rule is mechanical rather than a judgement call.
    //
    // objectiveFailed fell 93 -> 88 -> 79, more slowly than the lexeme count, and
    // that is expected rather than disappointing: an item passes only when EVERY
    // lexeme in its `requires` row is taught, so the last missing word in a row
    // holds the whole item red. The lexeme count is the leading indicator; the
    // item count moves in steps as rows complete.
    //
    // The per-mock split moved unevenly (mock 1's reading 6 -> 10, mock 2's
    // listening 4 -> 8) because a tranche clears whole rows, not a fixed share
    // of each paper. A uniform movement would be the surprising result.
    expect(audit.missingObjectiveLexemes).toHaveLength(146);
  });

  it("measures a LARGER taught set than A1, which is what makes it a different gate", () => {
    const root = defaultCurriculumRoot();
    const a1 = buildSpanishA1MockAudit(root, "A1");
    const a2 = buildSpanishA1MockAudit(root, "A2");

    // The mirror of the pre-A1 assertion above: if the level argument were
    // ignored, A2 would measure the same corpus as A1 and the new gate would be
    // a copy of the old one wearing a new name.
    expect(a2.lessonCount).toBeGreaterThan(a1.lessonCount);
    expect(a2.taughtForms).toBeGreaterThan(a1.taughtForms);
  });

  it("keeps the committed report canonical and current", () => {
    expect(runSpanishA1MockAudit(["--check", "--level", "A2"], defaultCurriculumRoot())).toBe(0);
  });
});
