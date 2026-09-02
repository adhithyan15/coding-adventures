import { describe, expect, it, vi, afterEach } from "vitest";
import { cpSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
    // 8 -> 9 partial: `exam-inventory-sanskrit-a1.json` lands, and lands partial
    // in all four for that reason plus one of its own. Sanskrit's
    // `exam-levels.json` caveat says the traditional ladder is ordered by
    // grammar and text rather than by function, so the proxy cannot reach it at
    // all; the file carries a Register category that says so and a deliberately
    // uncovered point, SA-A1-RG-02, rather than pretending a functional
    // inventory measures a pariksha.
    // 9 -> 10 partial: `exam-inventory-french-a2.json` lands. It is the SECOND
    // A2 file and only the second file at any level above A1, and it is partial
    // in all four dimensions for a reason the German A2 file does not share:
    // Goethe publishes a finite ~1,300-item A2 word list to close a lexicon
    // against, and there is no published French equivalent. So its fifteen
    // lexical points are enumerated at the level of the DOMAIN, and every
    // covered one carries a note naming the exact set the corpus holds.
    expect(out).toMatch(/0 complete and 11 partial of 138/);
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
    // 7 -> 8: Sanskrit A1 joins it as well. French is still the only file this
    // test corrupts, so this number stays the written total minus exactly one.
    // 8 -> 9: Kannada A1 joins it, on the same rule.
    // 9 -> 10: French A2 joins the readable remainder. Only the French A1 file
    // is corrupted by this test, so this stays the written total minus one.
    expect(out).toMatch(/0 complete and 10 partial of 138/);
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
    //
    // This assertion used to pin the absolute corpus total, e.g.
    // `851 uncovered point(s) across 10 written`. That number is NOT what this
    // test is about -- it was only standing in for "the duplicate did not double
    // the count" -- and pinning it made this the most contended line in the
    // repo. In one day it read 529 -> 686 -> 793 -> 792 -> 775 -> 774 -> 786 ->
    // 839 -> 851, because it rises when an inventory lands and falls when a
    // tranche covers points. Every parallel author had to edit it, four PRs sat
    // DIRTY on it at once, and -- worse than the churn -- two branches that both
    // lower it merge QUIETLY because they agree, leaving an agreed wrong value.
    // Composing it by arithmetic was wrong every time it was tried, including
    // once when the arithmetic happened to agree.
    //
    // A ratchet is not available either: the number legitimately RISES when an
    // inventory lands, because an inventory enumerates points that were
    // previously unmeasurable. A ceiling would fail on exactly the work we most
    // want. So assert the invariant this test actually owns -- duplication
    // changes nothing -- by comparing the two runs. HL-C310.
    const clean = run(corpus());
    const root = corpus();
    cpSync(
      join(root, "core", "exam-inventory-french-a1.json"),
      join(root, "core", "exam-inventory-fr-a1.json"),
    );
    const { out } = run(root);

    const gap = /(\d+) uncovered point\(s\) across (\d+) written/;
    const before = clean.out.match(gap);
    const after = out.match(gap);
    expect(before, "clean corpus must report an exam-point gap").not.toBeNull();
    expect(after, "duplicated corpus must report an exam-point gap").not.toBeNull();

    // The duplicate must change NEITHER figure. Comparing the runs keeps the
    // real claim while letting both numbers move freely as authors land work.
    expect(after![1]).toBe(before![1]);
    expect(after![2]).toBe(before![2]);

    // And the written count must equal the distinct (language, level) pairs on
    // disk -- derived from the directory listing, which is not produced by the
    // plan engine, so this is a genuine cross-check rather than f(x) == x.
    const distinct = new Set(
      readdirSync(join(defaultCurriculumRoot(), "core"))
        .filter((f) => /^exam-inventory-.*\.json$/.test(f))
        .map((f) => {
          const doc = JSON.parse(
            readFileSync(join(defaultCurriculumRoot(), "core", f), "utf8"),
          ) as { language?: string; level?: string };
          return `${doc.language}/${doc.level}`;
        }),
    );
    expect(Number(before![2])).toBe(distinct.size);
    expect(distinct.size).toBeGreaterThan(0);
  });

  it("rejects a flag used as another flag's value", () => {
    expect(run(corpus()).code).toBe(0);
    let err = "";
    vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((err += chunk), true));
    expect(runCompletionPlan(["--root", "--format"])).toBe(2);
    expect(err).toMatch(/--root requires a value/);
  }, 120_000);
});
