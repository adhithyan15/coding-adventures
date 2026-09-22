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
    expect(audit.objectiveFailed).toBe(6);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 24, listening: 23, objectiveFailed: 3 },
      { reading: 22, listening: 25, objectiveFailed: 3 },
    ]);
    // THIS NUMBER MUST ONLY EVER FALL. A rise means a mock gained an item the
    // corpus cannot support.
    //
    // 191 -> 161 -> 146 -> 126 -> 107 -> 85 -> 68 -> 54 -> 39 -> 35 -> 31 ->
    // 27 -> 23 -> 19 -> 15 -> 11 -> 8 -> 6 are the seventeen vocabulary tranches: 431-436,
    // 437-439, 440-443, 444-447, 448-451, 452-455, 456-459, 460-464, 465, 466,
    // 467, 468, 469, 470, 471, 472 and 473.
    // The drop was exact for the first fourteen -- 30 headwords removed 30
    // lexemes, then 15, 20, 19, 22, 17, 14, 15, 4, 4, 4, 4, 4, 4. THE
    // FIFTEENTH IS THE FIRST THAT IS NOT: chapter 471 teaches SIX headwords
    // and this list falls by FOUR, because two of the six were never on it.
    // The SIXTEENTH breaks it the other way: chapter 472 teaches FIVE and
    // this list falls by THREE, and objectiveFailed does not move AT ALL.
    // Both are deliberate and are explained below. That
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
    // point. objectiveFailed went 93 -> 88 -> 79 -> 59 -> 40 -> 31 -> 23 -> 18 -> 13 -> 12 -> 11 -> 10 -> 9 -> 8 -> 7 -> 6:
    //
    //     tranche 1  (431-436)  30 words   5 items
    //     tranche 2  (437-439)  15 words   9 items
    //     tranche 3a (440-443)  20 words  20 items
    //     tranche 3b (444-447)  19 words  19 items
    //     tranche 4a (448-451)  22 words   9 items
    //     tranche 4b (452-455)  17 words   8 items
    //     tranche 4c (456-459)  14 words   5 items
    //     tranche 4d (460-464)  15 words   5 items
    //     tranche 4e (465)         4 words   1 item
    //     tranche 4f (466)         4 words   1 item
    //     tranche 4g (467)         4 words   1 item
    //     tranche 4h (468)         4 words   1 item
    //     tranche 4i (469)         4 words   1 item
    //     tranche 4j (470)         4 words   1 item
    //     tranche 4k (471)         6 words   1 item
    //     tranche 4l (472)         5 words   0 items
    //     tranche 4m (473)         3 words   0 items
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
    // TRANCHE 4b IS THE SAME RULE APPLIED HARDER, and it beats 4a: 17 words for
    // 8 items, 2.12 per item. The improvement is not a better ranking -- no
    // ranking exists any more -- it is the tie-break used deliberately. 4b goes
    // after the CHEAPEST remaining rows (seven two-word rows and one
    // three-word row, which is why chapter 455 carries five headwords rather
    // than four) grouped into four scenes, so EVERY chapter finishes exactly
    // two rows on its own. When every word costs the same, the only lever left
    // is which rows a scene happens to complete, and choosing scenes around the
    // cheapest rows is that lever.
    //
    // TRANCHE 4c IS WHERE THAT LEVER RUNS OUT TOO, and the rate says so: 2.80,
    // up from 4b's 2.12. Before authoring it, every coherent scene left was
    // measured, and all but one came out at EXACTLY 3.00 words per item. The
    // exception was `La cuenta` at 2.50, holding the last unblocked two-word
    // row. So 4c took that scene first and then three 3.00 scenes, and no
    // grouping available could have done better.
    //
    // From here the floor is 3.00 until the B1-mapped words move. FIVE of the
    // remaining rows are waiting on `explicar`, `creer` or `problema` -- all
    // three ALREADY TAUGHT, all three above the A2 book's ceiling. Crediting
    // them costs ZERO new lessons and is now worth more than a whole tranche
    // of authoring: see the HL-C418 shard in BACKLOG.d.
    //
    // TRANCHE 4d CONFIRMS THE FLOOR RATHER THAN BEATING IT: 15 words, 5 items,
    // exactly 3.00. Every scene still available was measured before authoring
    // and all five chosen came out at 3.00 -- there was no cheaper grouping to
    // find, and saying so is the point of measuring first. The B1-mapped words
    // are now FIVE of the thirteen remaining rows, so the mapping question is
    // worth more than the next tranche of authoring by a widening margin.
    //
    // The two remaining solo blockers, `explicar` and `problema`, are NOT
    // authoring work: both are already headwords whose spine nodes put them at
    // B1, so a reader of the A2 book has not met them. See the HL-C418 shard in
    // BACKLOG.d, and note that its first version recommended re-mapping all
    // three candidates and was wrong -- `explicar` cannot be MAPPED on its own,
    // because it requires atoms from three B1 lessons and introduces a grammar
    // atom of its own.
    //
    // Read that carefully, because an earlier changelog entry got it backwards.
    // `explicar` cannot be re-mapped alone. What it CAN do, once mapped, is
    // clear a row alone: it is the sole blocker on mock 2 / paper 1 / item 3.
    // The lexeme that clears nothing by itself is `creer`, whose only row also
    // wants `descontar`. Crediting `problema` alone takes 12 -> 11, `explicar`
    // alone 12 -> 11, and all three 12 -> 10.
    //
    // TRANCHE 4e (465) IS THE FLOOR ARRIVING, exactly where 4d predicted it.
    // 4 words for 1 item is 4.00, against 2.44 / 2.12 / 2.80 / 3.00 for
    // 4a-4d. Nothing went wrong: the eight remaining authorable rows each need
    // four words and share none of them, so 4.00 is the rate for all of them
    // and there is no scene grouping or tie-break left that beats it. 4e is one
    // chapter rather than four deliberately -- 4d ran 25 lessons and its review
    // found twenty authoring errors, so the unit of work is now sized to what a
    // review pass can actually check.
    //
    // The per-mock split moves unevenly because a tranche clears whole rows,
    // not a fixed share of each paper. A uniform movement would be the
    // surprising result. Tranche 4a is the clearest case: mock 2 lost 8 failing
    // items and mock 1 lost 1. Of the nine cleared rows, five were in mock 2's
    // listening paper and three in its reading paper, which is exactly what the
    // pass counts above record -- mock 2 went 13 -> 16 reading and 14 -> 19
    // listening, mock 1 went 19 -> 20 reading and did not move on listening.
    //
    // Tranche 4b fell evenly by comparison: mock 1 lost 2 (one reading, one
    // listening) and mock 2 lost 6 (three and three). Every one of the eight
    // deltas above is accounted for by a named row, which is the check worth
    // running -- a pass count that rose without a cleared row to explain it
    // would mean the audit had changed rather than the corpus.
    //
    // 4c fell 3 and 2: mock 1 lost three listening rows, mock 2 lost two
    // listening rows, and neither reading paper moved. All five cleared rows
    // are paper-2 rows, which is why both reading counts are unchanged.
    //
    // 4d fell 1 and 4, and MOCK 2 IS NOW DOWN TO THREE FAILING ITEMS. Of its
    // four cleared rows three are paper-1 (reading 19 -> 22) and one paper-2
    // (listening 24 -> 25); mock 1 cleared one paper-2 row. The two papers are
    // now diverging sharply, which is itself information: what remains is
    // concentrated in mock 1.
    //
    // 4e fell 1 and 0: its single row is mock 1 / paper 1 / item 22 (reading
    // 21 -> 22), so mock 1 is down to nine and mock 2 has not moved. SEVEN
    // authorable rows are left, not eight -- 465 took one -- and six of the
    // seven are mock 1's, which is the divergence above having run to its
    // conclusion. 7 rows x 4 words = 28, plus the 7 distinct words behind the
    // five B1-mapped rows, is the 35 asserted here.
    //
    // 469 is the fourth consecutive tranche to fall by exactly one item and
    // four lexemes, and it took mock 1 / paper 2 / item 28 (listening 20 ->
    // 21). That is the predicted floor rather than a stall: once no two
    // remaining rows share a word, a four-word chapter can clear at most one
    // row, so 4.00 is what the ranking now yields per chapter until the
    // B1-mapped rows are reconsidered.
    //
    // 473 IS THE SECOND SUCH TRANCHE, blocked the same way: its row (mock 2 /
    // paper 1 / item 25) is `recomendar, empezar, estado, explicar, norma`, and
    // after it the item is blocked by `explicar` alone -- taught at
    // ES-C41-explicar but deriving to B1 through SPINE-GIVE-REASONS, per
    // HL-C418. Its HL-C421 check found `aconsejar`, the QUESTION STEM'S OWN
    // VERB, absent corpus-wide -- the second consecutive item whose stem verb
    // the book does not teach, after `surgir` in 472. That check is now the
    // most productive step in the pre-check.
    //
    // WHAT IS LEFT AFTER 473: six lexemes, of which FOUR are already taught and
    // blocked only by how this audit measures -- creer, explicar and problema
    // (HL-C418, HL-C420) and responder (HL-C422). Only `descontar` and
    // `invitar` are genuinely untaught, so one more vocabulary chapter exhausts
    // the authorable A2 gap entirely.
    //
    // 472 IS THE FIRST TRANCHE TO CLEAR NO ITEM AT ALL, ON PURPOSE. Its row
    // (mock 2 / paper 1 / item 21) is `encuesta, preguntar, usuario,
    // responder, lectura`, and after this chapter the item is blocked by
    // `responder` ALONE -- a word ES-C40-contestar already teaches, as
    // ES-LEX-RESPONDER-06, with a Grammar Lens, the re- plus spondere
    // derivation and la respuesta. The audit cannot see it because the taught
    // set is built from lesson.realization.headword only and never reads
    // introduces.knowledge. Writing a second `responder` lesson would clear
    // the row and would be the duplication HL-C418 forbids, so it was not
    // written. See BACKLOG.d HL-C422: the repair is a citationFormCredits
    // entry or teaching the audit to read introduces, both of which move this
    // number and want their own branch. A tranche that closes real gaps and
    // moves no item is the honest reading of that situation.
    //
    // 471 IS THE FIRST TRANCHE TO TEACH MORE WORDS THAN ITS ROW NAMES, AND
    // THE ARITHMETIC ABOVE BREAKS HERE ON PURPOSE. Its row (mock 1 / paper 2 /
    // item 31) is `curso, grupo, avanzado, perderse, principiante, sencillo`,
    // every one of them a word of the AUDIO PASSAGE. The question's correct
    // option reads `el nivel era demasiado alto para ella`, and BOTH `nivel`
    // and `demasiado` have zero substring hits anywhere in the corpus. So the
    // four ranked words clear the row while leaving the item unanswerable: a
    // candidate who understood every word of the dialogue still cannot read
    // option (b). The chapter teaches six, which makes the item genuinely
    // answerable and moves this count by the same four either way -- the two
    // extra words were never on the list, because nothing puts option text on
    // it. See BACKLOG.d HL-C421. Expect this to recur: the exactness asserted
    // above was always a property of the ROWS, not of the papers.
    //
    // 470 is the fifth such tranche and took mock 1 / paper 2 / item 47
    // (listening 21 -> 22). TWO clean four-word rows remain -- mock 1 / p2 /
    // item 31 and mock 2 / p1 / item 21 -- and the other five all turn on
    // `explicar`, `creer` or `problema`, which are ALREADY TAUGHT and excluded
    // only because their spine node derives above A2. See BACKLOG.d HL-C418
    // and HL-C420; do not teach them a second time.
    expect(audit.missingObjectiveLexemes).toHaveLength(6);
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
