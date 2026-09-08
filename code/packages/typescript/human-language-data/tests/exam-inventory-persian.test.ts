// ---------------------------------------------------------------------------
// The Persian A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Five inventories — Italian, Latin, Marwadi, Persian and Portuguese — were the
// five tracks with no inventory at all, and they were written from five
// branches. Each would have appended its describe block to the END of that file,
// and git conflicts on adjacent end-of-file additions however they are ordered.
// Bengali, Arabic and Urdu already live apart for exactly this reason.
//
// WHAT THIS SUITE HAS TO GUARD THAT THE OTHERS DO NOT
// exam-levels.json maps this track's rungs onto AMFA and its own caveat says the
// correspondence is this project's judgement rather than AMFA's. So the suite
// pins the negative claim: nothing in the file is derived from or attributed to
// AMFA, and the inventory does not depend on the mapping it qualifies.
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

describe("the committed Persian A1 inventory", () => {
  const inventory = loadExamInventory("persian", "A1");
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
    const taught = trackIntroducedAtoms(lessons, "persian");
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
    // Nothing drops, and the interesting transfers are the INVERSIONS: no
    // grammatical gender at all, no definite article, a mirrored closing
    // question mark where Spanish has an inverted opening one, a script with no
    // letter case. Restating around the target's machinery IS deriving.
    expect(dropped.size).toBe(0);
    expect(proxy.note).toMatch(/RESTATING A POINT AROUND THE TARGET\s+LANGUAGE'S MACHINERY IS DERIVING IT/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; persianSpecific?: boolean };
      expect(cast.persianSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "FA-A1-V-02", "FA-A1-V-03", "FA-A1-V-04", "FA-A1-F2-11",
      "FA-A1-REG-01", "FA-A1-REG-02", "FA-A1-REG-03", "FA-A1-REG-04",
    ]);
  });

  it("claims NOTHING from AMFA, and says the inventory does not rest on that mapping", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/NOTHING IN THIS FILE MAY BE\s+ATTRIBUTED TO AMFA/);
    expect(inventory.about).toMatch(/THIS INVENTORY DOES NOT DEPEND ON THAT MAPPING AND DOES NOT TEST IT/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: ONE EXISTS AND IT IS PROJECT-DEFINED/);
    // The caveat is neither stale nor overstated here — it is accurate. The
    // point that carries it says so, and says what this file can add: that it
    // does not depend on the correspondence the caveat qualifies.
    const caveat = inventory.points.find((point) => point.id === "FA-A1-REG-04")!;
    expect(caveat.note).toMatch(/neither stale nor overstated/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from the frontmatter it MEASURED, not from the caveat", () => {
    // The caveat is about the AMFA mapping and is silent on register, so there
    // is nothing to import. What the measurement found is that this is the only
    // track in its batch declaring two varieties and eight register values.
    expect(inventory.about).toMatch(/THE REGISTER COLUMN WAS MEASURED, NOT IMPORTED/);
    const register = inventory.points.filter((point) => point.category.startsWith("Sath-e sokhan"));
    expect(register).toHaveLength(4);
    expect(register.find((point) => point.id === "FA-A1-REG-03")!.note)
      .toMatch(/contemporary-iranian-persian/);
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

  it("reports an EMPTY joining column, no demonstrative at all, and a strong script column", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(180);
    expect(coverage.covered).toBe(74);
    expect(coverage.unmapped).toBe(106);
    expect(coverage.partial).toBe(0);
    // FLAT ZERO, the twelfth in this series. Every match for و in 71 files is
    // the LETTER vav being discussed — its shape in the chapter 15 writing
    // lesson, its silence inside خواندن and خواهر, its vowel duty inside
    // خورشید. Not one occurrence is the conjunction. اما, یا, چون and که all
    // return zero, and که takes four proxy points with it.
    expect(coverage.byCategory["Rabt (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 0,
    });
    // Nothing can be pointed at: neither این nor آن occurs anywhere.
    expect(coverage.byCategory["Eshare (pointing)"]!).toEqual({ enumerated: 1, covered: 0 });
    // The script column is the track's strength, and the ezafe column is where
    // it does something no Romance proxy point could have asked for.
    // 9 -> 10: HL-C350 closes FA-A1-KH-11, the Persian digits.
    expect(coverage.byCategory["Khatt (script and orthography)"]!).toEqual({ enumerated: 12, covered: 10 });
    expect(coverage.byCategory["Ezafe (the linker Spanish has no counterpart for)"]!)
      .toEqual({ enumerated: 3, covered: 2 });
    expect(formatExamCoverage(coverage)).toContain(
      "persian A1 (partial inventory): 74/180 points covered (41%)",
    );
  }, 60_000);

  it("keeps the numeral column CLOSED, and says what each closure cost", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "persian");
    // THE COLUMN THAT WAS FLAT ZERO. yek, do, se and chahar returned no
    // occurrences in 71 files, so a learner could give no age, no price, no
    // telephone number and no time. HL-C350's chapters 16-20 close it, and the
    // assertions below pin HOW rather than merely THAT.
    const cardinals = inventory.points.find((point) => point.id === "FA-A1-Q-01")!;
    expect(cardinals.probe).not.toBeNull();
    for (const c of ["YEK", "DO", "SE", "CHAHAR", "PANJ", "SHESH", "HAFT", "HASHT", "NOH", "DAH"]) {
      expect(cardinals.probe, c).toContain(`FA-LEX-${c}`);
      expect(taught.has(`FA-LEX-${c}`), c).toBe(true);
    }
    // The teens are probed as a RULE and three worked examples, not as nine
    // lexical atoms, because that is what the track teaches: Persian puts the
    // unit in front of dah. A probe naming nine words would claim six lessons
    // that do not exist.
    expect(cardinals.probe).toContain("FA-GRAMMAR-TEENS-DAH");
    expect(taught.has("FA-GRAMMAR-TEENS-DAH")).toBe(true);
    expect(cardinals.probe).not.toContain("FA-LEX-CHAHARDAH");
    // The digits close from the script side, and the DIRECTION rule is probed
    // with them: a reader who knows the ten shapes and reads them right-to-left
    // reads every two-digit price backwards.
    const digits = inventory.points.find((point) => point.id === "FA-A1-KH-11")!;
    expect(digits.probe).toContain("FA-SCRIPT-DIGITS-LTR");
    for (const d of ["ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE"]) {
      expect(digits.probe, d).toContain(`FA-SCRIPT-DIGIT-${d}`);
      expect(taught.has(`FA-SCRIPT-DIGIT-${d}`), d).toBe(true);
    }
    // ORDINALS, the weakest column in the whole corpus -- twenty tracks
    // enumerate one and eighteen leave it uncovered. Persian's cost ONE ending,
    // so the probe names the suffix rather than a list of forms.
    const ordinals = inventory.points.find((point) => point.id === "FA-A1-Q-02")!;
    expect(ordinals.probe).toContain("FA-MORPH-ORDINAL-OM");
    expect(taught.has("FA-MORPH-ORDINAL-OM")).toBe(true);
    expect(ordinals.note).toMatch(/it cost one ending/);
  }, 60_000);

  it("keeps the finding that governs every other number in the file", () => {
    // FOURTEEN INFINITIVES, ELEVEN PRESENT STEMS, AND NOT ONE CONJUGATED VERB.
    // The learner holds the two hardest parts of the Persian verb and can use
    // neither, so a covered verb point here means vocabulary, not grammar.
    const present = inventory.points.find((point) => point.id === "FA-A1-V-05")!;
    expect(present.probe).toBeNull();
    expect(present.note).toMatch(/THE GAP THAT MAKES EVERY VERB IN THIS TRACK INERT/);
    // The stem point IS covered, and it is persianSpecific because Spanish stem
    // changes are exceptions to a rule while Persian's are the rule.
    const stem = inventory.points.find((point) => point.id === "FA-A1-V-02")!;
    expect(stem.probe).not.toBeNull();
    // POLARITY AND NEGATION COME APART, as in Latin: بله and نه are both taught
    // in chapter 1, and the na- prefix that negates a verb is not.
    expect(inventory.points.find((point) => point.id === "FA-A1-ADV-05")!.probe).not.toBeNull();
    expect(inventory.points.find((point) => point.id === "FA-A1-V-11")!.probe).toBeNull();
    // REPAIR, checked separately: every morpheme of نمی‌فهمم is separately
    // taught and no lesson assembles them.
    const repair = inventory.points.find((point) => point.id === "FA-A1-F2-11")!;
    expect(repair.probe).toBeNull();
    expect(repair.note).toMatch(/THE PIECES ARE ALL PRESENT AND UNJOINED/);
  });
});
