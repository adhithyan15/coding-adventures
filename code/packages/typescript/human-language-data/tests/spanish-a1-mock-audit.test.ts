import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../src/loader.js";
import {
  assertAnswerKeyParse,
  buildSpanishA1MockAudit,
  parseAnswerKeyRows,
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
    // 6 -> 48, AND THE RISE IS THE POINT. This is the one movement the note
    // below forbids, taken deliberately, because the instrument was wrong in
    // the direction that flatters us. HL-C421 found that a `requires` row lists
    // the words of the AUDIO PASSAGE and never the words of the question stem
    // or the answer options, so an item could pass while a candidate could not
    // read the sentence they have to choose between.
    //
    // FORTY-SEVEN lexemes were added, across FORTY-THREE of the hundred rows
    // the gate reads (fifty per mock).
    //
    // THE NUMBER DID NOT CONVERGE, AND THAT IS THE REAL FINDING.
    // 6 -> 15 -> 17 -> 32 -> 48, across three rounds of adversarial review,
    // each of which found MORE untaught words in items that were passing:
    //
    //     round 1   3 more    the count went 15 -> 17
    //     round 2  17 more    the count went 17 -> 32
    //     round 3  18 more    the count went 32 -> 48
    //
    // Three passes, thirty-eight corrections, and every single one in the same
    // direction: the narrowing had been too generous. A method that is wrong
    // three times running in one direction is not nearly right; it is BIASED,
    // and the bias is toward flattering the corpus. 48 is where the search
    // stopped, not where the truth is.
    //
    // The three false clears worth naming, because they show how the rules
    // failed rather than that they did:
    //
    //   `espacio`   cleared by a 3-CHARACTER PREFIX rule matching the taught
    //               `esperar`. Unrelated words. Mock 2 item 5, option (a).
    //   `ahorro`    cleared as a relative of a verb -- but `ahorrar` is not
    //               taught either. Mock 1 item 23, option (c).
    //   `mejor`     held back on the false claim that it "is a headword". The
    //               only headword containing it is `pasar a mejor vida`, which
    //               sits on SPINE-READ-CULTURAL-WEIGHT and derives to C2, so it
    //               is outside this gate's own taught set. It belongs with
    //               creer/explicar: taught, but above the ceiling.
    //
    // Round 3's additions are the ones that should worry a reader most,
    // because several sit in the KEYED option -- the correct answer turns on a
    // word the course never teaches: `sitio` (m1 12, m2 6), `prever` (m1
    // enunciado A), `sustituir` (m1 enunciado B), `obligatorio` (m2 9),
    // `inscribirse` (m2 12), `acudir` (m2 49), `comunicar`/`interrumpir`
    // (m2 50).
    //
    // `afirmar` was also held back as exam apparatus. It IS apparatus in the
    // two instruction lines, but mock 1 item 41 is a scored statement --
    // "Afirma que sin el curso no le daran el puesto" -- and that row already
    // lists curso, puesto and dar from the same stem, so the row does read the
    // stem and singling out one word was inconsistent.
    //
    // THREE EXCLUSIONS STAND, each because the book itself supplies the word:
    // `escolar` from the taught `la escuela`; `comedor`, which ES-C297-tenedor
    // does not merely make derivable but GLOSSES OUTRIGHT -- "A comedor is
    // where the eating is done" -- at chapter 297, pre-A1; and `coger`, which
    // appears in a message body rather than a stem or option and is therefore
    // outside what this pass measures.
    //
    // Every word added is load-bearing. Most sit in an option a candidate must
    // weigh; SEVEN sit in the STEM -- `alumno` (m1 22), `mejor` (m2 5),
    // `practicar` and `infancia` (m1 18), `trescientos` (m1 24), `quejarse`
    // (m2 17), `costumbre` (m2 19) -- so those questions cannot be read at all.
    //
    // The honest consequence: "the A2 vocabulary programme is complete" was
    // false. It was complete against an instrument that only read the passage.
    expect(audit.objectiveFailed).toBe(48);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 12, listening: 13, objectiveFailed: 25 },
      { reading: 10, listening: 17, objectiveFailed: 23 },
    ]);
    // THIS NUMBER MUST ONLY EVER FALL, with one exception already spent above:
    // a rise is allowed when it is the MEASUREMENT being corrected to be
    // harsher, never when it is the corpus losing ground. A rise means a mock
    // gained an item the corpus cannot support.
    //
    // 191 -> 161 -> 146 -> 126 -> 107 -> 85 -> 68 -> 54 -> 39 -> 35 -> 31 ->
    // 27 -> 23 -> 19 -> 15 -> 11 -> 8 -> 6 -> 4 are the eighteen vocabulary tranches: 431-436,
    // 437-439, 440-443, 444-447, 448-451, 452-455, 456-459, 460-464, 465, 466,
    // 467, 468, 469, 470, 471, 472, 473 and 474.
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
    //     tranche 4n (474)         4 words   0 items   <- LAST
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
    // AFTER 474 THE AUTHORABLE A2 VOCABULARY GAP IS EXHAUSTED. Every one of the
    // four lexemes still on this list is ALREADY TAUGHT and blocked only by how
    // this audit measures:
    //
    //     creer     ES-C41-creer       B1 via SPINE-GIVE-REASONS   HL-C418
    //     explicar  ES-C41-explicar    B1 via SPINE-GIVE-REASONS   HL-C418
    //     problema  ES-C268-problema   misfiled on a B1 travel node HL-C420
    //     responder ES-C40-contestar   taught but never a headword  HL-C422
    //
    // Writing a lesson for any of them is the duplication HL-C418 forbids, so
    // the programme STOPS here rather than moving this number dishonestly. The
    // four entries are decisions, not vocabulary work. 474's own HL-C421 check
    // found `importe` and `golpe`, neither in its row, in a MATCHING item where
    // the statement -- not an option set -- is the text to read.
    //
    // WHAT WAS LEFT AFTER 473: six lexemes, of which FOUR are already taught and
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
    //
    // 4 -> 51, and the note above predicted exactly this: "Expect this to
    // recur: the exactness asserted above was always a property of the ROWS,
    // not of the papers." The list is no longer four already-taught words
    // blocked on a spine decision; it is five such words plus 46 that are
    // not headwords at or below A2:
    //
    //     taught, but above this gate's ceiling (HL-C418, HL-C420, HL-C422)
    //       creer, explicar, problema, responder, mejor
    //     not a headword at or below A2, newly visible because the rows now
    //     read the question rather than only the passage
    //       aburrido, acudir, adelantado, afirmar, ahorro, alquiler, alumno,
    //       calzado, cancelar, comprender, comunicar, concreto, costumbre,
    //       descartar, disponible, equipaje, error, espacio, finalidad, ganar,
    //       infancia, iniciativa, inscribirse, interrumpir, jardinero, justo,
    //       material, mejora, multa, obligatorio, parecido, practicar, prever,
    //       profesional, prometer, quejarse, recuperar, resolver, rápido,
    //       sitio, sustituir, trescientos, título, utilizar, variedad, vigilar
    //
    // The second group is authorable vocabulary work, and its existence
    // retracts the claim that the A2 programme had run out of words. It had
    // run out of words THE PASSAGE NEEDED.
    //
    // "NOT A HEADWORD AT OR BELOW A2", precisely -- not "never taught
    // anywhere", and not "zero occurrences in the curriculum", both of which
    // earlier drafts of this comment claimed and both of which are false.
    // `alquiler` is glossed in ES-C441-contrato, `espacio` in
    // ES-C57-es-inicial, `mejora` in ES-C466-visible -- the very lesson that
    // supplies `visible` to the same row; `aburrido` appears in CHANGELOG.md
    // and roadmap.md prose, where it is named as untaught; `resolver` appears
    // in a grammar-cells.json overlay no lesson references. Counting all of
    // them missing is still right, because this gate is headword-only by
    // construction and `glossed-not-taught.ts` treats body presence as a
    // REVIEW QUEUE rather than a teaching claim; but the wording has to say
    // what the measurement measures. HL-C422 argues the rule itself.
    //
    // `mejora`, not `mejorar`: mock 1 item 23 reads "la mejora de las notas",
    // a deverbal NOUN. An earlier draft listed the infinitive, which appears
    // nowhere in the item -- lemmatising across a part-of-speech boundary,
    // which would have let a future `mejorar` headword flip the item to
    // passing while the option stayed unreadable.
    //
    // THIS LIST IS A FLOOR AND THE SEARCH FOR IT DID NOT CONVERGE. See the
    // note above: three rounds of review found 3, then 17, then 18 more, every
    // correction in the same direction. Do not read 51 as the answer; read it
    // as the largest number anyone has yet demonstrated.
    expect(audit.missingObjectiveLexemes).toHaveLength(51);
    expect(audit.missingObjectiveLexemes).toEqual([
      "aburrido",
      "acudir",
      "adelantado",
      "afirmar",
      "ahorro",
      "alquiler",
      "alumno",
      "calzado",
      "cancelar",
      "comprender",
      "comunicar",
      "concreto",
      "costumbre",
      "creer",
      "descartar",
      "disponible",
      "equipaje",
      "error",
      "espacio",
      "explicar",
      "finalidad",
      "ganar",
      "infancia",
      "iniciativa",
      "inscribirse",
      "interrumpir",
      "jardinero",
      "justo",
      "material",
      "mejor",
      "mejora",
      "multa",
      "obligatorio",
      "parecido",
      "practicar",
      "prever",
      "problema",
      "profesional",
      "prometer",
      "quejarse",
      "recuperar",
      "resolver",
      "responder",
      "rápido",
      "sitio",
      "sustituir",
      "trescientos",
      "título",
      "utilizar",
      "variedad",
      "vigilar",
    ]);
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

describe("parseAnswerKeyRows", () => {
  // The rows this parser drops are the rows the gate never scores, and a row
  // the gate never scores is an item it reports as fine. Every case below is a
  // FAIL-OPEN shape: the audit comes back cleaner than the corpus is.
  const key = (body: string) => `## Prueba 1\n\n| # | Clave | Requiere |\n|---|---|---|\n${body}`;
  const clean = { unscored: [], malformed: [], declared: new Map() };

  it("reads a well-formed row", () => {
    expect(parseAnswerKeyRows(key("| 1 | b | casa, perro |"))).toEqual({
      rows: [{ paper: 1, item: 1, requires: ["casa", "perro"] }],
      ...clean,
    });
  });

  it.each([
    ["a lone CR", "\r"],
    ["U+2028", "\u2028"],
    ["U+2029", "\u2029"],
  ])("does not lose every row to %s endings", (_label, terminator) => {
    // `.` cannot match any of these, so under `/\r?\n/` the whole file
    // collapsed to ONE line, `$` was unreachable, and the parser returned
    // nothing -- from which the audit reports `objectiveFailed: 0`,
    // `reading: 0`, `listening: 0`. A clean bill of health for items it never
    // read, and `--write` would persist it. This is why the terminator is
    // shared with `mock-stem-coverage.ts` rather than spelled out twice.
    const text = ["## Prueba 1", "| 1 | b | casa |", "| 2 | a | perro |"].join(terminator);
    expect(parseAnswerKeyRows(text)).toEqual({
      rows: [
        { paper: 1, item: 1, requires: ["casa"] },
        { paper: 1, item: 2, requires: ["perro"] },
      ],
      ...clean,
    });
  });

  it("keeps CRLF as one terminator rather than two", () => {
    // The alternation puts `\r\n` first for this. Splitting it as two would
    // insert an empty line between every row -- harmless here, but the same
    // ordering bug in a parser that counts lines is not.
    expect(parseAnswerKeyRows("## Prueba 1\r\n| 1 | b | casa |").rows).toEqual([
      { paper: 1, item: 1, requires: ["casa"] },
    ]);
  });

  it("scores an empty requirement column as NEITHER a pass nor a failure", () => {
    // Both obvious answers hide something, which is why this row is reported
    // instead of scored:
    //   `[""]`  -- the item FAILS on a requirement nobody wrote (what `+` ->
    //             `*` produced before any filter);
    //   `[]`    -- `[].every(...)` is `true`, so the item PASSES
    //             unconditionally and counts toward `reading` (what filtering
    //             alone produced, and the FAIL-OPEN direction).
    expect(parseAnswerKeyRows(key("| 7 | b |  |"))).toEqual({
      rows: [],
      unscored: [{ paper: 1, item: 7 }],
      malformed: [],
      declared: new Map(),
    });
  });

  it("drops the empty entry a trailing comma leaves behind, and still scores the row", () => {
    expect(parseAnswerKeyRows(key("| 7 | b | casa, |"))).toEqual({
      rows: [{ paper: 1, item: 7, requires: ["casa"] }],
      ...clean,
    });
  });

  it("reports a numbered row the pattern rejected instead of losing it", () => {
    // ONE TRAILING SPACE after the closing pipe. That is the whole defect: the
    // row vanishes, and a vanished row is an item the gate never scores, which
    // downstream is indistinguishable from an item that passed. A guard that
    // only fires when EVERY row is lost never sees this.
    const text = ["## Prueba 1", "| 1 | b | casa |", "| 2 | a | perro | "].join("\n");
    expect(parseAnswerKeyRows(text)).toEqual({
      rows: [{ paper: 1, item: 1, requires: ["casa"] }],
      unscored: [],
      malformed: ["| 2 | a | perro | "],
      declared: new Map(),
    });
  });

  it("does not mistake a table header or separator for a malformed row", () => {
    // `malformed` only collects lines that open `| <digits> |`. A header or a
    // `|---|` separator must not trip the guard, or the gate refuses every
    // real key.
    expect(parseAnswerKeyRows(key("| 1 | b | casa |")).malformed).toEqual([]);
  });

  it("scores Prueba 1 and 2 and ignores rows under any other heading", () => {
    const text = [
      "## Prueba 1", "| 1 | b | casa |",
      "## Prueba 2", "| 2 | a | perro |",
      "## Prueba 3", "| 3 | c | gato |",
    ].join("\n");
    expect(parseAnswerKeyRows(text)).toEqual({
      rows: [
        { paper: 1, item: 1, requires: ["casa"] },
        { paper: 2, item: 2, requires: ["perro"] },
      ],
      ...clean,
    });
  });

  it("returns nothing for text with no rows, which the file reader turns into a throw", () => {
    // The pure parser is allowed to come back empty; "" is a legitimate input
    // to a parser. `parseAnswerKey` is the one that refuses, because "" is not
    // a legitimate answer key and a gate that read nothing must not report
    // success.
    expect(parseAnswerKeyRows("")).toEqual({ rows: [], ...clean });
    expect(parseAnswerKeyRows("## Prueba 1\n\nno table here")).toEqual({ rows: [], ...clean });
  });

  it("stays flat on a long line rather than backtracking", () => {
    // The old `\s*([^|]+)\s*\|$` measured CUBIC on this shape: 1.1s at
    // n=2000, 8.7s at n=4000, 29s at n=6000. It is reachable because
    // `buildSpanishA1MockAudit` is exported and takes a caller-supplied
    // `root`. A generous ceiling, so the test pins the complexity class rather
    // than a machine's speed.
    const started = Date.now();
    expect(parseAnswerKeyRows(`|1||${" ".repeat(8000)}`).rows).toEqual([]);
    expect(Date.now() - started).toBeLessThan(1000);
  });
});

describe("answer-key headings", () => {
  it("resets the paper on a heading that is not `## Prueba <digit>`", () => {
    // LIVE IN THE CORPUS, not hypothetical: `a2/mock-{1,2}-answer-key.md` write
    // `## Pruebas 3 and 4` -- PLURAL, so `(\d)` cannot follow the `s` -- above a
    // section whose own text says it is "not read by the audit". The old
    // `if (heading) paper = ...` could only SET, never clear, so `paper` stayed
    // 2 and any numbered table there was scored as listening. The A1 keys were
    // safe only by luck: they spell `## Prueba 3` and `## Prueba 4`, which
    // match and reset.
    const text = [
      "## Prueba 2", "| 26 | a | perro |",
      "## Pruebas 3 and 4", "| 51 | b | ayuntamiento |",
    ].join("\n");
    expect(parseAnswerKeyRows(text).rows).toEqual([
      { paper: 2, item: 26, requires: ["perro"] },
    ]);
  });

  it("reads the item count the heading states about itself", () => {
    // The file says how big it is. That is the one invariant here that does not
    // depend on anticipating the shape of the damage.
    const text = "## Prueba 1 · Comprensión de lectura (25 items)\n| 1 | b | casa |";
    expect(parseAnswerKeyRows(text).declared).toEqual(new Map([[1, 25]]));
  });

  it("tolerates a heading that states no count", () => {
    expect(parseAnswerKeyRows("## Prueba 1\n| 1 | b | casa |").declared).toEqual(new Map());
  });
});

describe("looksLikeDataRow, via the malformed bucket", () => {
  const under = (row: string) => parseAnswerKeyRows(`## Prueba 1\n${row}`).malformed;

  it.each([
    ["one trailing space", "| 2 | a | perro | "],
    ["one leading space", " | 2 | a | perro |"],
    ["a bolded item number", "| **2** | a | perro |"],
    ["an item label with a suffix", "| 2a | a | perro |"],
  ])("flags a row broken by %s", (_label, row) => {
    // The first detector was a PREFIX test, `/^\|\s*\d+\s*\|/`, which needed the
    // pipe at index 0 and a bare ASCII digit right after it -- blind to every
    // edit but the trailing space it was written for. Bold inside these tables
    // is already house style: `pre-a1/mock-1-answer-key.md` writes
    // `| 21 | **gracias** | *gracias* |`.
    expect(under(row)).toEqual([row]);
  });

  it.each([
    ["a table header", "| # | Clave | Requiere |"],
    ["a separator", "|---|---|---|"],
    ["prose", "Not objectively keyed."],
    ["a two-column numbered table", "| 1 | ***onru*** |"],
  ])("does not flag %s", (_label, line) => {
    // The digit in the first cell is what separates a data row from the
    // furniture; the pipe ARITY is what separates this table from another one.
    // Flagging the header or the separator would make the gate refuse every
    // real key. Measured rather than assumed: a `cells.length >= 4` draft
    // flagged a line in 177 of the 8670 markdown files under
    // `human-languages/`, and requiring this table's three columns brings that
    // to 100 -- none of them among the six keys `parseAnswerKey` opens, all of
    // which flag zero lines.
    expect(under(line)).toEqual([]);
  });
});

describe("assertAnswerKeyParse", () => {
  // Exported and taking a parse rather than a path because every hole in these
  // guards, across FIVE rounds of review, was found by reading -- the only way
  // in was a run over the real corpus.
  //
  // The checks used to be a pile of partial ones (zero rows, then adjacency,
  // then span, then a conditional count) and each round found a hole at the
  // join between two of them. They are now ONE set equality against what the
  // headings declare, so every test below is the same failure seen from a
  // different side.
  const parse = (over: Partial<Parameters<typeof assertAnswerKeyParse>[0]> = {}) => ({
    rows: [
      { paper: 1, item: 1, requires: ["casa"] },
      { paper: 2, item: 2, requires: ["perro"] },
    ],
    unscored: [],
    malformed: [],
    declared: new Map<number, number>([[1, 1], [2, 1]]),
    ...over,
  });
  // Prueba 1 is 1..2 and Prueba 2 is 3..4, so a test can move one item without
  // also tripping the count.
  const two = (rows: { paper: number; item: number; requires: string[] }[]) =>
    parse({ rows, declared: new Map([[1, 2], [2, 2]]) });
  const full = [
    { paper: 1, item: 1, requires: ["casa"] },
    { paper: 1, item: 2, requires: ["gato"] },
    { paper: 2, item: 3, requires: ["perro"] },
    { paper: 2, item: 4, requires: ["sol"] },
  ];

  it("accepts a complete parse", () => {
    expect(() => assertAnswerKeyParse(parse(), "key.md")).not.toThrow();
    expect(() => assertAnswerKeyParse(two(full), "key.md")).not.toThrow();
  });

  it("names the rejected line rather than only counting it", () => {
    // "1 table row rejected" tells a maintainer that something is wrong and
    // nothing about where, in a file of 160 lines.
    expect(() => assertAnswerKeyParse(parse({ malformed: ["| 2 | a | perro | "] }), "key.md"))
      .toThrow(/first "\| 2 \| a \| perro \| "/);
  });

  it("refuses an item that states no requirements", () => {
    expect(() => assertAnswerKeyParse(parse({ unscored: [{ paper: 1, item: 7 }] }), "key.md"))
      .toThrow(/item\(s\) 1\.7 state no requirements/);
  });

  it("refuses an empty parse", () => {
    expect(() => assertAnswerKeyParse(parse({ rows: [] }), "key.md"))
      .toThrow(/parsed no answer-key rows/);
  });

  it("REQUIRES a declared count rather than skipping the check without one", () => {
    // This read `count !== undefined && ...`, so the one guard that does not
    // depend on anticipating the damage switched itself off whenever a heading
    // stopped saying `(25 items)` -- silently. Both edits that do that are
    // ordinary: `## Prueba 3 · Expresión e interacción escritas` in the same
    // file already carries no count, and `ítems` is the correct Spanish
    // spelling. Either one, plus a lost row, gave a clean bill of health.
    expect(() => assertAnswerKeyParse(parse({ declared: new Map() }), "key.md"))
      .toThrow(/Prueba 1 heading declares no item count/);
  });

  // Each of these pins a hole that a PARTIAL check let through, and the
  // declared counts are set so that a count comparison alone cannot fire --
  // the point is that the set equality catches them, not the arithmetic.
  it("catches a lost FIRST row", () => {
    // `findIndex((item, index) => index > 0 && ...)` never examines index 0, so
    // `[2, 3, ..., 25]` read as perfectly contiguous. The span check that
    // replaced it is blind here too: `[2,3]` has span 2 and length 2.
    expect(() => assertAnswerKeyParse(two(full.filter((row) => row.item !== 1)), "key.md"))
      .toThrow(/Prueba 1 declares 2 items 1-2, but parsed is missing 1/);
  });

  it("catches a lost LAST row", () => {
    expect(() => assertAnswerKeyParse(two(full.filter((row) => row.item !== 4)), "key.md"))
      .toThrow(/Prueba 2 declares 2 items 3-4, but parsed is missing 4/);
  });

  it("catches a duplicate that fills the gap a drop left", () => {
    // `[1, 1, 3]` -- span 3, length 3. The SPAN check passed this, and the
    // adjacency check it replaced had caught it, so that round was a strict
    // regression. An ordinary copy-paste over the next row does exactly this.
    const rows = [
      { paper: 1, item: 1, requires: ["casa"] },
      { paper: 1, item: 1, requires: ["gato"] },
      ...full.filter((row) => row.paper === 2),
    ];
    expect(() => assertAnswerKeyParse(two(rows), "key.md"))
      .toThrow(/Prueba 1 declares 2 items 1-2, but parsed is missing 2 duplicates 1/);
  });

  it("catches a renumbered paper whose count is still right", () => {
    // Every count is correct and the numbers are contiguous; they are simply
    // the wrong numbers. No aggregate check can see this.
    const rows = full.map((row) => (row.paper === 2 ? { ...row, item: row.item + 1 } : row));
    expect(() => assertAnswerKeyParse(two(rows), "key.md"))
      .toThrow(/Prueba 2 declares 2 items 3-4, but parsed is missing 3 has unexpected 5/);
  });

  it("anchors Prueba 2 to the end of Prueba 1 rather than to 1", () => {
    // The papers number straight through -- 1..25 then 26..50 -- so the second
    // paper's expected run depends on the first's declared size. A key whose
    // Prueba 2 restarted at 1 would otherwise look fine.
    const rows = full.map((row) => (row.paper === 2 ? { ...row, item: row.item - 2 } : row));
    expect(() => assertAnswerKeyParse(two(rows), "key.md"))
      .toThrow(/Prueba 2 declares 2 items 3-4/);
  });

  it("refuses a scored paper whose heading declares ZERO items", () => {
    // `(0 items)` makes the expected set EMPTY, and an empty expectation
    // matches an empty parse -- so the paper is skipped entirely and its items
    // are never scored. The per-paper `parsed no Prueba N rows` check that the
    // set equality replaced caught this unconditionally; deleting it alongside
    // the others reopened the hole.
    //
    // It was asymmetric, too: `(0 items)` on Prueba 1 is caught incidentally,
    // because the anchor never advances and Prueba 2's run then starts at the
    // wrong place. Only the LAST paper failed open -- and stubbing out a
    // not-yet-authored paper as `(0 items)` is ordinary editorial work.
    const rows = [{ paper: 1, item: 1, requires: ["casa"] }];
    expect(() => assertAnswerKeyParse(parse({ rows, declared: new Map([[1, 1], [2, 0]]) }), "key.md"))
      .toThrow(/Prueba 2 heading declares 0 items, outside 1-1000/);
  });

  it("refuses a declared count large enough to abort the process", () => {
    // `Array.from({ length: count })` does the work BEFORE anything caps the
    // message. `(999999999 items)` aborted outright -- FATAL ERROR, exit 134,
    // uncatchable, no gate message at all -- while `(99999999999 items)` was
    // SAFE, because `ArrayCreate` rejects a length at or above 2^32 with a
    // plain RangeError. The merely enormous number was the dangerous one.
    expect(() => assertAnswerKeyParse(parse({ declared: new Map([[1, 999999999], [2, 1]]) }), "key.md"))
      .toThrow(/Prueba 1 heading declares 999999999 items, outside 1-1000/);
  });

  it("sanitises the name it was handed rather than trusting the caller", () => {
    // This function is EXPORTED and `package.json` declares no `exports` map --
    // the same deep-import argument `spanishMockDir` hardens against. A `\r`
    // in the name would otherwise forge a second log line. The in-repo caller
    // passes the RAW path, because `reportableFilename` quotes and so is not
    // idempotent.
    expect(() => assertAnswerKeyParse(parse({ rows: [] }), "a\r\nFORGED"))
      .toThrow(/^"a\\nFORGED": parsed no answer-key rows$/);
  });

  it("truncates a long list of missing items rather than printing all of them", () => {
    const rows = [{ paper: 1, item: 1, requires: ["casa"] }, { paper: 2, item: 21, requires: ["sol"] }];
    expect(() => assertAnswerKeyParse(parse({ rows, declared: new Map([[1, 20], [2, 1]]) }), "key.md"))
      .toThrow(/is missing 2, 3, 4, 5, 6, \.\.\./);
  });
});

describe("a pipe inside the requirement cell", () => {
  // `([^|]*)` takes the LAST pipe-delimited run, so a stray pipe silently
  // TRUNCATES the requirement list -- and a shorter list is the fail-open
  // direction, because there is less for `taught` to miss.
  //
  // Two checks, because neither sees the other's case. A first version used
  // one (comparing the regex capture to the last split piece) and was
  // worthless: both take the last run, so they agree by construction. My own
  // attack matrix caught that, not review.
  const rows = (row: string) =>
    parseAnswerKeyRows(`## Prueba 1 (2 items)\n| 1 | a | casa | perro |\n${row}`);

  it("rejects a row split by an inline-code pipe, on cell count", () => {
    const row = "| 2 | b | casa | `a|b`, gato |";
    expect(rows(row).malformed).toEqual([row]);
  });

  it("rejects a row hiding a pipe behind a backslash escape", () => {
    // Here the split and the regex disagree in OPPOSITE directions, so the
    // cell counts come out equal and the arity check sees nothing -- while
    // `([^|]*)` still stops at the escaped pipe and drops `casa`.
    const row = "| 2 | b | casa | casa \\| gato |";
    expect(rows(row).malformed).toEqual([row]);
  });

  it("keeps a row whose cell count matches its table", () => {
    expect(rows("| 2 | b | casa | gato |").rows).toHaveLength(2);
  });
});
