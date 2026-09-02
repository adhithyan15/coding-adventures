import { describe, expect, it, vi, afterEach } from "vitest";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { runCompletionPlan } from "../src/plan-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

// `plan-cli` had no test file at all, so every one of the security review's dirty
// controls left the suite green. These pin the coupling the review found: the
// presence list, the coverage measurement, and what happens when they disagree.

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
  vi.restoreAllMocks();
});

function corpus(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-plan-"));
  roots.push(root);
  cpSync(defaultCurriculumRoot(), root, { recursive: true });
  return root;
}

function run(root: string, headSize = 25): { code: number; out: string; err: string } {
  let out = "";
  let err = "";
  vi.spyOn(process.stdout, "write").mockImplementation((chunk) => ((out += chunk), true));
  vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((err += chunk), true));
  const code = runCompletionPlan(["--root", root, "--head", String(headSize)]);
  return { code, out, err };
}

function editInventory(root: string, name: string, edit: (doc: Record<string, unknown>) => void): void {
  const path = join(root, "core", name);
  const doc = JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;
  edit(doc);
  writeFileSync(path, JSON.stringify(doc));
}

describe("the plan CLI", () => {
  it("leads French and German with their exam gap on a clean corpus", () => {
    // HL16 adds one assessment-contract item per track ahead of content proxy
    // work. Ask for the complete enumerable queue so this test continues to
    // assert that the measured French/German exam gaps survive behind that gate.
    const { code, out } = run(corpus(), 200);
    expect(code).toBe(0);
    expect(out).toMatch(/exam-point — french/);
    expect(out).toMatch(/exam-point — german/);
    // Spanish USED to teach 100% of the points enumerated, so it got no
    // exam-point item at all. Enumerating the functional, notional and
    // orthographic dimensions took it to 223/273, so it now has 50 uncovered
    // points and joins the queue. That is the planner behaving correctly: the
    // work did not appear, it was always there and was invisible while the
    // target list covered one of four content dimensions.
    expect(out).toMatch(/exam-point — spanish/);
    // Hindi is the first Indic track with a target list, so it is the first one
    // whose exam gap is a MEASUREMENT rather than a proxy. It joins the queue
    // for the same reason Spanish did: the work was always there and was
    // invisible while no inventory named it.
    expect(out).toMatch(/exam-point — hindi/);
    expect(out).toMatch(/human-validation — marwadi/);
    expect(out).toMatch(/human-validate 2 of 2 pre-A1 full mock\(s\) for marwadi/);
    // Marathi joins with an A1 inventory of its own, so this is the first
    // exam-point item on a track with no external syllabus behind it.
    expect(out).toMatch(/exam-point — marathi/);
    // Telugu is the third track with no external syllabus behind it, and the
    // first whose inventory is bounded by the Spanish proxy ALONE: unlike Hindi
    // it has no timed mocks to mine, and unlike Marathi it has no task shapes
    // either. Its exam gap is still a measurement rather than a proxy, which is
    // the property this assertion pins.
    expect(out).toMatch(/exam-point — telugu/);
    // Still PARTIAL, and therefore still "0 complete": the pronunciation half
    // of phonology-orthography has no A1-only boundary in the source, so that
    // single dimension keeps the whole inventory partial. This is the HL20 rule
    // doing its job — three complete dimensions do not add up to a complete
    // file, and the inventory item is not suppressed.
    //
    // 4 -> 5 partial: Hindi A1 lands, and lands partial in all FOUR dimensions.
    // No Hindi awarding body publishes a content syllabus — DBHPS names its
    // examinations and prescribes readers, and stops — so the file is bounded by
    // the CEFR A1 descriptors and this project's own checked-in A1 mocks. A
    // wholly editorial inventory is exactly what `partial` is for.
    // 4 -> 5 partial: `exam-inventory-marathi-a1.json` is a PROJECT-DEFINED
    // equivalent, since `exam-levels.json` records Marathi as `basis: editorial`
    // with no widely-sat ladder. Every one of its four dimensions is partial and
    // says why, so it lands on exactly the same side of this gate as the three
    // externally-sourced files. An editorial basis buys no discount here, which
    // is the property that makes the number worth pinning.
    // 6 -> 7 partial: `exam-inventory-telugu-a1.json` lands, and lands partial in
    // all FOUR dimensions like the two Indic files before it. Its phonology
    // dimension is partial for a reason worth keeping visible — the track's sound
    // material lives in per-lesson `sounds:` front matter and in a reference page,
    // neither of which declares an atom, so no probe can reach it.
    // 7 -> 8 partial: `exam-inventory-tamil-a1.json` lands, partial in all four
    // for the same reason every proxy-derived file is: a proxy lends a level and
    // cannot close a dimension.
    expect(out).toMatch(/0 complete and 8 partial of 138/);
  }, 120_000);

  it("does not let an unreadable inventory look like an absent one", () => {
    // The HIGH finding. `listExamInventories` lists any file that parses;
    // `loadExamInventory` is stricter. A file in the gap was listed as PRESENT
    // (so no `exam-inventory` item) and threw on load (so no `exam-point` item) —
    // the track vanished from both families while the report asserted its
    // inventory existed, silently, at exit 0.
    const root = corpus();
    editInventory(root, "exam-inventory-french-a1.json", (doc) => {
      doc.points = [];
    });
    const { code, out, err } = run(root);
    expect(code).toBe(1);
    expect(err).toMatch(/french A1 inventory exists but could not be read/);
    // It comes BACK as an inventory to write, which is what the old comment
    // falsely claimed already happened.
    expect(out).toMatch(/138\s+exam-inventory/);
    // 3 -> 4: French is the one made unreadable here, and Hindi A1 now sits
    // alongside Spanish and the two German files in the readable remainder.
    // 3 -> 4: Spanish A1, German A1, German A2 and Marathi A1 still parse; only
    // the French file was corrupted by this test.
    // 5 -> 6: Telugu A1 joins the readable remainder. The French file is still
    // the only one this test corrupts, so this number tracks the written total
    // minus exactly one.
    // 6 -> 7: Tamil A1 joins it too, on the same rule.
    expect(out).toMatch(/0 complete and 7 partial of 138/);
    expect(out).toMatch(/1 exist but could not be READ/);
  }, 120_000);

  it("refuses an inventory whose points lost their probe key", () => {
    // `covered` is `probe !== null && …`, and a missing key is `undefined`, which
    // is not `null` — so an inventory with every probe deleted reported 100% and
    // suppressed its own work item. It never reached the try/catch.
    const root = corpus();
    editInventory(root, "exam-inventory-french-a1.json", (doc) => {
      for (const point of doc.points as Record<string, unknown>[]) delete point.probe;
    });
    const { code, err } = run(root);
    expect(code).toBe(1);
    expect(err).toMatch(/has no usable probe/);
  }, 120_000);

  it("counts a duplicated inventory once", () => {
    // `listExamInventories` dedupes nothing, so two files declaring the same
    // (language, level) produced two items with the IDENTICAL id, a doubled
    // projection, and a shrunken inventory backlog.
    const root = corpus();
    cpSync(
      join(root, "core", "exam-inventory-french-a1.json"),
      join(root, "core", "exam-inventory-fr-a1.json"),
    );
    const { out } = run(root);
    // 103 -> 98: the French questions chapter covered five A1 points (HL-C229).
    // German A2 adds 51 source-bounded points; three already have exact atoms.
    // 146 -> 196: Spanish A1 contributed 0 uncovered points while it enumerated
    // grammar only. Enumerating its functional, notional and orthographic
    // dimensions added 50 points with no corresponding atom, and this total is
    // the sum across all four written inventories.
    // 196 -> 194: HL23 §10 authors `saber` (chapter 389) and maps `A1-F2-16`
    // and `A1-F2-17`, the two PCIC ability points, onto it. The backlog falls
    // because two points were CLOSED, not because an inventory shrank — the
    // Spanish denominator is still 273.
    // 194 -> 190. HL23 §13 maps four `Nociones evaluativas` points: A1-NG6-03, -08
    // and -10 are closed by the qualities rung's own lessons, and -09 needed no
    // authoring at all — its note claimed the corpus never introduces `saber`, which
    // stopped being true when chapter 389 authored it two slices earlier.
    // 190 -> 317, and 4 -> 5 written. `core/exam-inventory-hindi-a1.json`
    // enumerates 282 A1 points and the track covers 155 of them, so Hindi
    // contributes its 127 unmapped points to the total. The unmeasured remainder
    // falls 20 -> 19 for the same reason: Hindi stopped being a proxy.
    //
    // The Hindi denominator is 282 and not 172 because its point set is derived
    // structurally from the DELE-sourced Spanish inventory used as a proxy for
    // LEVEL. A first draft built from CEFR descriptors alone reached 172 and
    // measured 67%; the proxy added 110 demands the descriptors never
    // enumerated -- `apna`, object-marking `ko`, transport, payment, `aaj` --
    // and the honest figure fell to 55%. A ruler drawn too short flatters the
    // corpus, so the bigger denominator is the point, not a side effect.
    // 530 -> 529: retiring hand-written French chapter 6 closed A1-PRON-03,
    // obligatory liaison, which the generated chapter teaches as a named rule
    // with its own atom instead of two passing mentions inside `sounds` blocks.
    // 686 -> 793, and 7 -> 8 written. Two inventories landed independently
    // and met in an earlier merge, so that figure was RE-MEASURED rather than
    // added up by hand:
    //   +157  Telugu A1: 326 points enumerated, 169 covered.
    //   +107  Tamil A1:  262 points enumerated, 155 covered.
    //
    // 793 -> 792: French chapter 8 closed A1-LEX-07, telling the time -- the
    // generated chapter teaches et quart, et demie and moins le quart, which the
    // hand-written one named in a sentence and then deferred. RE-MEASURED for
    // the same reason as the line above: this branch was written when the total
    // was 529 across 6 inventories, and Telugu and Tamil landed in between.
    //
    // 748 -> 725: the Marathi joining tranche (chapters 30-36) closed
    // twenty-three of Marathi's own 213 uncovered points. Two exam-driven
    // tranches met in this merge -- Telugu's 44 points landed on main while this
    // was open -- so the figure below was RE-MEASURED with `npm run plan` on the
    // merged tree. This branch was cut when the total was 686 across 7
    // inventories; subtracting 23 from any of the numbers in between would have
    // been wrong every time.
    expect(out).toMatch(/725 uncovered point\(s\) across 8 written/);
    // 190 -> 403, and 4 -> 5 written. Marathi's own A1 inventory enumerates 301
    // points and the corpus covers 88, so it contributes 213. Nothing regressed:
    // a twentieth track stopped being unmeasurable, and the backlog grew by
    // exactly the debt that was previously invisible. The size of the jump is
    // itself the finding — Marathi's target list is DERIVED from the Spanish
    // one, which is the only DELE-sourced set here, so its denominator is what an
    // attributable A1 inventory actually asks for rather than what a
    // descriptor-led guess remembered to include.
    expect(out).toMatch(/725 uncovered point\(s\) across 8 written/);
    // 792 -> 748. The Telugu chapter 74-80 vocabulary tranche was authored
    // against `exam-inventory-telugu-a1.json`'s OWN uncovered list rather than by
    // topic, so 35 headwords closed 44 points and the corpus-wide backlog fell by
    // exactly that many. Telugu went 169/326 (52%) to 213/326 (65%) against an
    // unchanged denominator: no point was added, removed or reworded to move it.
    // The base moved twice while this branch was open -- Tamil's inventory took
    // it to 793 and French chapter 8 to 792 -- which is why it is not the 686
    // this branch was cut against. Re-measured on the merged tree, never
    // subtracted.
    //

    // 529 -> 686, and 6 -> 7 written. `core/exam-inventory-telugu-a1.json`
    // enumerates 326 A1 points and the corpus covers 169, so Telugu contributes
    // its 157 unmapped points. The unmeasured remainder falls 18 -> 17 for the
    // same reason: a twenty-first track stopped being unmeasurable, and the
    // backlog grew by exactly the debt that was previously invisible.
    //
    // 326 is the largest denominator here, and deliberately so. Telugu's point
    // set is derived from the same DELE-sourced Spanish proxy as Hindi's and
    // Marathi's, and it splits several of the proxy's points in two wherever the
    // corpus covers one half and not the other -- the parts of a dwelling but not
    // the word for a house, the age question but not the age answer. A merged
    // point would have scored those as covered, which is the flattering
    // direction; splitting them is what makes the 52% honest.
    //
    // Tamil's 262 is the smallest Indic denominator and that is a property of
    // the LANGUAGE: eight Spanish past-tense points collapse into one Tamil gap
    // because Tamil's past is one slot in one machine, four article points
    // collapse into two because Tamil has no article, and nine punctuation
    // points collapse into one because modern Tamil uses the same marks as
    // English. Every collapse lists all of its source points in `derivedFrom`,
    // which is what the totality test above checks.
    expect(out).toMatch(/0 complete and 8 partial of 138/);
    expect(out).toMatch(/the other 16 track\(s\)/);
  }, 120_000);

  it("rejects a flag used as another flag's value", () => {
    expect(run(corpus()).code).toBe(0);
    let err = "";
    vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((err += chunk), true));
    expect(runCompletionPlan(["--root", "--format"])).toBe(2);
    expect(err).toMatch(/--root requires a value/);
  }, 120_000);
});
