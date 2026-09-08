// ---------------------------------------------------------------------------
// The Marwadi A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Five inventories — Italian, Latin, Marwadi, Persian and Portuguese — were the
// five tracks with no inventory at all, and they were written from five
// branches. Each would have appended its describe block to the END of that file,
// and git conflicts on adjacent end-of-file additions however they are ordered.
// Bengali, Arabic and Urdu already live apart for exactly this reason.
//
// WHAT THIS SUITE HAS TO GUARD THAT THE OTHERS DO NOT
// This track's whole curriculum is a market transaction and for 257 lessons it
// taught no numeral. That is the kind of finding a coverage percentage buries —
// the point is one of 197 — so it got a test of its own, and so did the fact
// that a bargaining course has no word for "no".
//
// HL-C350 closed the numeral half (chapters 32–36) and the assertions below now
// pin the CLOSURE rather than the absence, in both directions: the probe must
// name real atoms, and the coverage count must move by exactly the two points
// that were closed. The polarity finding is untouched and still red.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../src/exam-inventory.js";

describe("the committed Marwadi A1 inventory", () => {
  const inventory = loadExamInventory("marwadi", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("gives every unmapped point a note, because an unmapped point IS the work queue", () => {
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim().length, `${point.id} is unmapped and says nothing`).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "marwadi");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total, and drops nothing", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
    // Nothing is dropped, and the file has to say why an empty list is a walk
    // that was DONE: Devanagari's lack of letter case, the danda, the matra
    // system and the postposition each answer a Spanish point that looks
    // Spanish-only. Restating around the target's machinery IS deriving.
    expect(dropped.size).toBe(0);
    expect(proxy.note).toMatch(/RESTATING A POINT AROUND THE TARGET LANGUAGE'S MACHINERY IS DERIVING IT/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; marwadiSpecific?: boolean };
      expect(cast.marwadiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "MW-A1-V-08", "MW-A1-F2-15",
      "MW-A1-REG-01", "MW-A1-REG-02", "MW-A1-REG-03", "MW-A1-REG-04",
      "MW-A1-SAU-01", "MW-A1-SAU-02", "MW-A1-SAU-03", "MW-A1-SAU-04",
    ]);
  });

  it("says NO external ladder exists, and that the internal envelope is a contract not evidence", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Marwadi may have no external syllabus at all/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: ONE EXISTS AND IT IS PROJECT-DEFINED/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("records the exam-levels caveat as STALE rather than deferring to it", () => {
    // The caveat says the mapping "does not claim that the current starter
    // chapter reaches it". The track has 312 lessons across 36 chapters, an
    // assessment spec, task shapes, and 25 four-skill scored practice atoms.
    // A caveat that has been overtaken is recorded as overtaken — the same
    // treatment a sibling track's OVERSTATED caveat got, in the other direction.
    const stale = inventory.points.find((point) => point.id === "MW-A1-REG-04")!;
    expect(stale.label).toMatch(/IS STALE, AND IN THE TRACK'S FAVOUR/);
    expect(stale.note).toMatch(/describes a track that no longer exists/);
    // And the register column was measured, not imported: the caveat is silent
    // on register, so the frontmatter was counted directly.
    expect(inventory.about).toMatch(/THE REGISTER COLUMN WAS MEASURED, NOT IMPORTED/);
    const register = inventory.points.filter((point) => point.category.startsWith("Aadar"));
    expect(register).toHaveLength(4);
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(new Set(anchors.map((anchor) => anchor.kind))).toEqual(
      new Set(["sourced-proxy", "external-framework", "project-owned", "editorial"]),
    );
    for (const anchor of anchors) expect(anchor.note.trim().length, anchor.id).toBeGreaterThan(0);
    const known = new Set(anchors.map((anchor) => anchor.id));
    for (const point of inventory.points) {
      const ids = (point as unknown as { anchorIds?: string[] }).anchorIds;
      expect(ids?.length, `${point.id} names no anchor`).toBeGreaterThan(0);
      for (const id of ids ?? []) expect(known.has(id), `${point.id} cites unknown anchor ${id}`).toBe(true);
    }
  });

  it("reports an EMPTY joining column, an empty adverb column, and a closed transaction column", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(197);
    expect(coverage.covered).toBe(76);
    expect(coverage.unmapped).toBe(121);
    expect(coverage.partial).toBe(0);
    // FLAT ZERO. `ar`, `ane`, `aur` (and), `pan`, `par` (but), `ke` (that),
    // `jad` (when) all return ZERO occurrences in 312 files. A learner with
    // twelve kinship words and seven foods cannot say "bread and tea".
    expect(coverage.byCategory["Yojak (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 0,
    });
    // Also flat: nothing is here, there, today, yesterday, badly, or not.
    expect(coverage.byCategory["Kriya-visheshan (adverbs)"]!).toEqual({ enumerated: 8, covered: 0 });
    // And the column this track is actually built around is closed outright.
    expect(coverage.byCategory["Saudo (the counter transaction this track is built around)"]!).toEqual({
      enumerated: 4,
      covered: 4,
    });
    expect(formatExamCoverage(coverage)).toContain(
      "marwadi A1 (partial inventory): 76/197 points covered (39%)",
    );
  }, 60_000);

  it("keeps the findings a percentage would bury: the numerals that closed, and no word for no", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "marwadi");
    // (1) THE MARKET COURSE THAT HAD NO NUMBERS. Chapters 26–31 teach "how much
    //     is this?", "how much altogether?", "this is very expensive", "make it
    //     a little cheaper", "what final price will you give?" and "take the
    //     money", and for 257 lessons not one numeral stood behind any of them:
    //     ek, do, teen, chaar and paanch returned zero (the raw matches for `do`
    //     were the imperative verb "give"). HL-C350's chapters 32–36 teach
    //     twelve cardinals, so this point is now CLOSED — and the assertion is
    //     the closure itself, atom by atom, rather than a boolean.
    const numerals = inventory.points.find((point) => point.id === "MW-A1-Q-01")!;
    expect(numerals.probe).not.toBeNull();
    expect(numerals.derivedFrom).toEqual(["A1-Q-01", "A1-NG2-01", "A1-NE16-01"]);
    for (const numeral of ["EK", "DO-TWO", "TEEN", "CHAAR", "PAANCH", "CHHA-SIX",
      "SAAT", "AATH", "NO-NINE", "DAS", "BEES", "SO"]) {
      // BOTH doors, because this track splits a word by skill and a probe that
      // named only the LEX atom would report a number the hand cannot write.
      expect(numerals.probe, numeral).toContain(`MW-LEX-${numeral}-01`);
      expect(numerals.probe, numeral).toContain(`MW-SCRIPT-${numeral}-01`);
      expect(taught.has(`MW-LEX-${numeral}-01`), numeral).toBe(true);
      expect(taught.has(`MW-SCRIPT-${numeral}-01`), numeral).toBe(true);
    }
    // The note must keep BOTH halves: what was missing, and what is still
    // missing. Eleven to nineteen wait on four independent vowels and one
    // consonant, which is a script debt and not a numeral one.
    expect(numerals.note).toMatch(/CLOSED BY HL-C350/);
    expect(numerals.note).toMatch(/NOT CLOSED, DELIBERATELY: the numbers between ten and twenty/);
    // …and the digits, the same hole seen from the script side, closed with it.
    const digits = inventory.points.find((point) => point.id === "MW-A1-LIP-13")!;
    expect(digits.probe).not.toBeNull();
    for (const digit of ["ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX",
      "SEVEN", "EIGHT", "NINE"]) {
      expect(digits.probe, digit).toContain(`MW-SCRIPT-DIGIT-${digit}-01`);
      expect(taught.has(`MW-SCRIPT-DIGIT-${digit}-01`), digit).toBe(true);
    }
    // (1b) ORDINALS did NOT close, and the note has to say which KIND of gap it
    //      is: pahlo/dujo/tijo/chautho need no untaught sign, so this is a
    //      sourcing gap, not a script one. A note that said only "absent" would
    //      send the next tranche looking for the wrong thing.
    const ordinals = inventory.points.find((point) => point.id === "MW-A1-Q-02")!;
    expect(ordinals.probe).toBeNull();
    expect(ordinals.note).toMatch(/no citable Marwari-specific source/);
    expect(ordinals.note).toMatch(/not a script debt/);
    // (2) POLARITY, untouched by HL-C350 and still red. `haan saa` gets a
    //     four-skill performance lesson in chapter 3 and there is NO negator at
    //     all, so a bargaining course still cannot decline, refuse or disagree —
    //     which now matters more, not less, since it can name a price.
    const polarity = inventory.points.find((point) => point.id === "MW-A1-ADV-05")!;
    expect(polarity.probe).toBeNull();
    expect(inventory.points.find((point) => point.id === "MW-A1-F4-09")!.probe).toBeNull();
    // (3) REPAIR, checked separately: nothing at all, at the one counter where
    //     mishearing the price is the whole risk.
    const repair = inventory.points.find((point) => point.id === "MW-A1-F2-15")!;
    expect(repair.probe).toBeNull();
    expect(repair.note).toMatch(/THE REPAIR COLUMN, CHECKED SEPARATELY, AND THERE IS NOTHING/);
  }, 60_000);

  it("records what this track does differently, since the numbers do not show it", () => {
    // Marwadi is one of only two tracks with zero reinforcement-window misses in
    // any window. The mechanism is visible in the curriculum shape and nowhere
    // in the coverage number, so it is enumerated as a point.
    const method = inventory.points.find((point) => point.id === "MW-A1-SAU-04")!;
    expect(method.probe).not.toBeNull();
    expect(method.note).toMatch(/expanding-interval retrieval schedule built into the curriculum shape/);
    expect(inventory.about).toMatch(/WHAT THIS TRACK DOES DIFFERENTLY/);
  });
});
