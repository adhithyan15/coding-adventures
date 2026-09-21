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
    expect(audit.objectiveFailed).toBe(31);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 20, listening: 14, objectiveFailed: 16 },
      { reading: 16, listening: 19, objectiveFailed: 15 },
    ]);
    // THIS NUMBER MUST ONLY EVER FALL. A rise means a mock gained an item the
    // corpus cannot support.
    //
    // 191 -> 161 -> 146 -> 126 -> 107 -> 85 are the five vocabulary tranches:
    // 431-436, 437-439, 440-443, 444-447 and 448-451. EVERY drop is exact -- 30
    // headwords removed 30 lexemes, then 15, then 20, then 19, then 22. That
    // arithmetic is the evidence a word was genuinely absent; one already taught
    // under another name would have made the drop smaller. Each was PREDICTED
    // from the audit before the chapters were wired and reproduced exactly by
    // the generator, so the selection rule is mechanical rather than a
    // judgement call.
    //
    // Tranche 4a's 22 lexemes came from 21 lessons, because ES-C448-estropear
    // carries the slash headword `estropear / estropeado`. The audit splits a
    // headword on `/ `, so a lesson that genuinely teaches a verb and its
    // participle-adjective together is credited with both -- which is the
    // honest reading, since the adjective is the form on the lift door.
    //
    // THE ITEM COUNT IS WHERE THE SELECTION RULE SHOWS, and it is the whole
    // point. objectiveFailed went 93 -> 88 -> 79 -> 59 -> 40 -> 31:
    //
    //     tranche 1  (431-436)  30 words   5 items
    //     tranche 2  (437-439)  15 words   9 items
    //     tranche 3a (440-443)  20 words  20 items
    //     tranche 3b (444-447)  19 words  19 items
    //     tranche 4a (448-451)  22 words   9 items
    //
    // An item passes only when EVERY lexeme in its `requires` row is taught, so
    // a word helps in proportion to how close its rows already are. Tranches
    // 1-2 ranked by how OFTEN a lexeme appeared, which stopped discriminating
    // once 145 of 146 remaining lexemes appeared in exactly one item. Tranche 3
    // ranked by how close each item was to being unblocked and taught only
    // words that were the sole survivor in their row: one word, one item, both
    // halves.
    //
    // TRANCHE 4a IS WHERE THE RANKING RUNS OUT, and the falling yield above is
    // the evidence rather than a regression. After tranche 3 only TWO solo
    // blockers were left, and only ONE lexeme (`explicar`) appeared in more
    // than one failing row; the other 106 appeared in exactly one. A greedy set
    // cover over the 40 remaining rows came out flat at roughly 2.7 words per
    // item from 5 words to 107, so no ordering front-loads value any more.
    // That is the ranking having finished its job, not a failure of it: the
    // cheap wins were all taken in tranches 1-3.
    //
    // So tranche 4a groups by SCENE instead -- a house move, a bike workshop,
    // the ground outside a sports centre, a service counter -- preferring
    // scenes whose words happen to finish whole rows. 22 words for 9 items is
    // 2.4 words per item, which is the flat rate the set cover predicted, and
    // predicting it in advance is what makes the number checkable.
    //
    // The two remaining solo blockers, `explicar` and `problema`, are NOT
    // authoring work: both are already headwords whose spine nodes put them at
    // B1, so a reader of the A2 book has not met them. See the HL-C418 shard in
    // BACKLOG.d, and note that its first version recommended re-mapping all
    // three candidates and was wrong -- `explicar` requires atoms from three B1
    // lessons and cannot move.
    //
    // The per-mock split moves unevenly because a tranche clears whole rows,
    // not a fixed share of each paper. A uniform movement would be the
    // surprising result. Tranche 4a is the clearest case: mock 2 lost 8 failing
    // items and mock 1 lost 1. Of the nine cleared rows, five were in mock 2's
    // listening paper and three in its reading paper, which is exactly what the
    // pass counts above record -- mock 2 went 13 -> 16 reading and 14 -> 19
    // listening, mock 1 went 19 -> 20 reading and did not move on listening.
    expect(audit.missingObjectiveLexemes).toHaveLength(85);
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
