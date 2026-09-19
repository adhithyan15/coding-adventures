import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed German A2 source tranche", () => {
  it("stays explicitly partial while turning official source evidence into named gaps", () => {
    const inventory = loadExamInventory("german", "A2");
    expect(isExamInventoryComplete(inventory)).toBe(false);
    expect(Object.values(inventory.scope).every((entry) => entry.status === "partial")).toBe(true);

    const lessons = loadTrackLessons("german");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage).toMatchObject({
      language: "german",
      level: "A2",
      inventoryComplete: false,
      enumerated: 51,
      covered: 3,
      unmapped: 48,
    });
    expect(formatExamCoverage(coverage)).toContain("german A2 (partial inventory): 3/51 points covered (6%)");
  });
});

describe("the committed German A1 inventory", () => {
  const inventory = loadExamInventory("german", "A1");

  it("keeps every point's probe key", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
    }
  });

  it("refuses an empty probe, which would score as covered", () => {
    // Symmetry with the French block. `loadExamInventory` throws on `probe: []`
    // regardless, but the assertion belongs beside every inventory so a future
    // one cannot be added without it.
    for (const point of inventory.points) {
      expect(Array.isArray(point.probe) ? point.probe.length : 1).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that exist in the corpus", () => {
    const lessons = loadTrackLessons("german");
    const taught = trackIntroducedAtoms(lessons, "german");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("never lets an unmapped point read as 'nobody has looked yet'", () => {
    // The rule Marathi has had since HL-C290, applied here for the reason it was
    // written: every one of this inventory's 49 unmapped points carried NO note,
    // so "the corpus does not teach it" and "nobody has checked" were the same
    // JSON. Sixteen of the 49 turned out to be fully taught and merely unprobed,
    // and the only way to tell the two apart was to read 297 lessons. A note is
    // what stops that reading being redone.
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim(), `${point.id} is unmapped and must say why`).toBeTruthy();
    }
  });

  it("reports the same grammar-shaped gap French does", () => {
    // German holds 468 atoms across 297 lessons. The categories that stay empty
    // are the ones a candidate is examined on: questions and prepositions.
    // Vocabulary is again the strongest column. Two independent tracks, one
    // shape — see HL-C226.
    const lessons = loadTrackLessons("german");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(70);
    // 21 -> 37, and NOT ONE LESSON CHANGED. Sixteen points were taught in full
    // and had `probe: null`, which `exam-inventory.ts` documents as "no atom in
    // the corpus corresponds to this point" — a finding, scored as uncovered.
    // Here it was not a finding; it was 49 points nobody had written a probe for,
    // 33 of them genuinely open and 16 of them closed since the chapter that
    // taught them was generated. Each of the sixteen was confirmed by reading the
    // lesson, not by the atom's name looking right:
    //
    //   A1-N-01  A1-N-02  A1-ART-01  A1-ART-02   the capitalisation lesson states
    //     the rule outright; GE-C01-der-die-das states three genders AND that they
    //     are unpredictable; GE-C06-ein-eine gives ein/eine off the der/die/das split.
    //   A1-PRO-01  A1-PRO-03   all eight nominative pronouns have their own lessons,
    //     and GE-C05-ihr prints the du/Sie/ihr register grid.
    //   A1-V-01  A1-V-02  A1-V-03   chapter 26 gives sein one lesson PER PERSON,
    //     chapter 22 does the same for haben, and the weak endings are five atoms
    //     across chapters 2 and 5. The paradigms exist as cells, which is why no
    //     single atom looked like the point.
    //   A1-V-08  A1-V-09   the haben-perfect and the sein-perfect, chapters 24 and 29.
    //   A1-SATZ-01  A1-AUS-01   verb-second is named in chapter 2; the three umlauts
    //     are three sound atoms plus the fronting rule in the writing segment.
    //   A1-LEX-01  A1-LEX-08  A1-LEX-12   seven greetings, all seven weekdays, all
    //     twelve months, all four seasons, fourteen everyday verbs.
    //
    // A tranche aimed at any of those sixteen would have written a second lesson
    // for material already in the book. That is the failure an inventory exists to
    // prevent, and it had been running in the flattering direction for months.
    //
    // 37 -> 56. Seven chapters, thirty-five items, forty-four atoms, forty-two
    // lessons. Nineteen points close, and FIVE COLUMNS go to full:
    //
    //   Das Verb        7/12 -> 12/12
    //   Das Pronomen     3/5  ->  5/5
    //   Der Satz         2/5  ->  5/5
    //   Die Frage        0/4  ->  4/4
    //   Die Negation     1/3  ->  3/3
    //
    // The ratio comes from spending rather than minting. kein is `ein` with two
    // letters on the front, so it inherits an ending set the reader bought in the
    // ein/eine lesson; `den` then names the row those endings move on, which is
    // the chapter GE-C14-einen promised in plain text and never delivered; and
    // the four possessives inherit the same set a third time. Six words end up
    // sharing one paradigm and only two of the six are new.
    expect(coverage.covered).toBe(56);
    for (const full of ["Das Verb", "Das Pronomen", "Der Satz", "Die Frage", "Die Negation"]) {
      const column = coverage.byCategory[full];
      expect(column?.covered, full).toBe(column?.enumerated);
    }
    // Die Praeposition is the one column the tranche did not touch, and it is
    // the next one: A1-ART-05 (dative article forms) sits under A1-PRAEP-02, so
    // one lesson set unblocks two points at once.
    expect(coverage.byCategory["Die Praeposition"]).toEqual({ enumerated: 4, covered: 0 });
    expect(coverage.byCategory["Der Artikel"]).toEqual({ enumerated: 5, covered: 4 });
    expect(coverage.byCategory["Grundwortschatz"]!.covered).toBeGreaterThan(0);
  }, 60_000);

  it("closes A1-ART-04 on the atom the corpus DEFERRED, not on a new paradigm", () => {
    // Named rather than left to the aggregate. The 56/70 above would move if this
    // probe were nulled, but it would move for eighteen other reasons too; this
    // pins WHICH four atoms close the point, and that two of the four are words
    // the reader has been saying since the haben and negation chapters.
    //
    // Both halves were falsified before this was kept: adding a fabricated id
    // fails the "probes only atoms that exist" test above, and nulling the probe
    // drops the total to 55.
    const point = inventory.points.find((p) => p.id === "A1-ART-04");
    expect(point?.probe).toEqual([
      // The one new article.
      "GE-LEX-DEN-01",
      // The rule that says only the der row moves — which is why the German case
      // system costs one row and not a nine-cell grid at this level.
      "GE-GRAMMAR-AKKUSATIV-NUR-MASKULIN-01",
      // SPENT, not minted. GE-C14-einen taught `einen` as a bare word and said on
      // the page: "the system behind it ... gets a chapter of its own later".
      "GE-LEX-EINEN-01",
      // Minted by this tranche's own first chapter, three chapters earlier, and
      // spent here.
      "GE-LEX-KEINEN-01",
    ]);
  });
});

// ---------------------------------------------------------------------------
// The first PROXY-DERIVED inventory (HL-C290).
//
// Spanish, French and German each restate an awarding body. Marathi has none:
// `core/exam-levels.json` records it as `exam: "no widely-sat ladder"`,
// `basis: "editorial"`, and the parallel Hindi effort settled the search
// negatively against the best-placed South Asian candidate — DBHPS publishes
// examination names and prescribed readers and no syllabus, and the Council of
// Europe has issued no Reference Level Description for Hindi. There is no South
// Asian equivalent of the Plan Curricular.
//
// So this file BORROWS A LEVEL RATHER THAN A LANGUAGE. Spanish's 273 points are
// DELE/PCIC-sourced and therefore an attributable statement of what an A1
// learner must handle; each is walked for what it DEMANDS, and the Marathi point
// that carries the same load is written down with the derivation recorded. That
// is legitimate, and it is exactly the kind of claim that decays into a fake
// standard if nobody guards the difference.
//
// These tests guard the difference. They do not check that the inventory is
// RIGHT; nothing automatable can. They check that the derivation stays total and
// auditable, that the file never stops saying what kind of claim it is, and that
// its probes stay executable.
// ---------------------------------------------------------------------------
