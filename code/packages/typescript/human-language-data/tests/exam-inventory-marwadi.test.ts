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
// This track began as a market transaction and for 257 lessons taught no
// numeral. It now has 901 direct lesson owners through chapter 151. That is the
// kind of moving corpus an inventory can silently fall behind: chapters 44–149
// added hundreds of exact atoms while this file still reported many as absent.
//
// HL-C350 closed the numeral half (chapters 32–36) and the assertions below now
// pin the CLOSURE rather than the absence, in both directions: the probe must
// name real atoms, and the coverage count must move by exactly the two points
// that were closed. The polarity finding is untouched and still red.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { loadExamInventory, loadLessons, loadTrackChapters } from "../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../src/exam-inventory.js";

// One read-only lesson snapshot serves every assertion in this suite. Loading
// the entire corpus per case obscures the inventory claim with Windows I/O.
const lessons = loadLessons();

// These are direct lesson owners, not words found incidentally in prose. Keeping
// the table beside the count makes a right total from the wrong evidence much
// harder to publish.
const reconciledLaterAtoms = {
  "MW-A1-N-03": [
    "MW-LEX-C78-TIME78-01", "MW-LEX-C78-TIME78-02", "MW-LEX-C78-TIME78-03",
    "MW-LEX-C90-TIME90-01", "MW-LEX-C90-TIME90-02",
  ],
  "MW-A1-PRON-05": ["MW-LEX-C102-DEF102-03"],
  "MW-A1-ADV-01": ["MW-LEX-C82-PLACE82-01", "MW-LEX-C82-PLACE82-02"],
  "MW-A1-ADV-02": ["MW-LEX-C79-TIME79-03", "MW-LEX-C79-TIME79-04"],
  "MW-A1-ADV-07": ["MW-LEX-KAISO-01", "MW-LEX-C96-LOC96-01"],
  "MW-A1-ADV-08": ["MW-LEX-C117-WHY117-01", "MW-LEX-C117-WHY117-05"],
  "MW-A1-V-07": ["MW-LEX-C50-FAMILY50-05", "MW-LEX-C57-VERBS57-03", "MW-LEX-C57-VERBS57-04"],
  "MW-A1-NG-04": [
    "MW-LEX-C81-PLACE81-01", "MW-LEX-C81-PLACE81-02", "MW-LEX-C96-LOC96-03",
    "MW-LEX-C96-LOC96-04", "MW-LEX-C97-LOC97-05", "MW-LEX-C98-LOC98-01",
  ],
  "MW-A1-NG-09": [
    "MW-LEX-C79-TIME79-03", "MW-LEX-C91-TIME91-02", "MW-LEX-C92-TIME92-03",
    "MW-LEX-C94-TIME94-03", "MW-LEX-C94-TIME94-04",
  ],
  "MW-A1-NG-12": [
    "MW-LEX-C88-WORLD88-01", "MW-LEX-C77-ADJ77-01", "MW-LEX-C77-ADJ77-02",
    "MW-LEX-C77-ADJ77-03",
  ],
  "MW-A1-NG-17": ["MW-LEX-C73-ADJ73-03", "MW-LEX-C73-ADJ73-04"],
  "MW-A1-NG-18": ["MW-LEX-C57-VERBS57-03", "MW-LEX-C57-VERBS57-04"],
  "MW-A1-NE-01": [
    "MW-LEX-C44-BODY44-01", "MW-LEX-C44-BODY44-02", "MW-LEX-C44-BODY44-03",
    "MW-LEX-C44-BODY44-04", "MW-LEX-C44-BODY44-05",
  ],
  "MW-A1-NE-02": [
    "MW-LEX-C144-QUAL144-03", "MW-LEX-C144-QUAL144-04", "MW-LEX-C144-QUAL144-05",
    "MW-LEX-C146-QUAL146-05", "MW-LEX-C147-QUAL147-02", "MW-LEX-C147-QUAL147-03",
  ],
  "MW-A1-NE-03": [
    "MW-LEX-C52-VERBS52-01", "MW-LEX-C52-VERBS52-02", "MW-LEX-C52-VERBS52-03",
    "MW-LEX-C52-VERBS52-04", "MW-LEX-C52-VERBS52-05",
  ],
  "MW-A1-NE-07": ["MW-LEX-C50-FAMILY50-02", "MW-LEX-C50-FAMILY50-03", "MW-LEX-C50-FAMILY50-04"],
  "MW-A1-NE-30": ["MW-LEX-C69-NATURE69-01", "MW-LEX-C69-NATURE69-02", "MW-LEX-C69-NATURE69-03"],
  "MW-A1-NE-31": ["MW-LEX-C84-TOWN84-01", "MW-LEX-C84-TOWN84-02", "MW-LEX-C89-WORLD89-03"],
  "MW-A1-NE-35": ["MW-LEX-C67-SCHOOL67-03", "MW-LEX-C67-SCHOOL67-04"],
} as const;

describe("the committed Marwadi A1 inventory", () => {
  const inventory = loadExamInventory("marwadi", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("pins the direct-owner snapshot behind the reconciliation", () => {
    const marwadiLessons = lessons.filter((lesson) => lesson.language === "marwadi");
    const marwadiChapters = loadTrackChapters().find((track) => track.language === "marwadi");
    expect(marwadiLessons).toHaveLength(901);
    expect(marwadiChapters?.chapters).toHaveLength(151);
    expect(trackIntroducedAtoms(lessons, "marwadi").size).toBe(838);
  });

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
    expect(inventory.about).toMatch(/NO READINESS, ATTAINMENT, MOCK-EXAM OR HUMAN-VALIDATION CLAIM/);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("records the exam-levels caveat as STALE rather than deferring to it", () => {
    // The caveat says the mapping "does not claim that the current starter
    // chapter reaches it". The track has 901 lessons across 151 chapters, an
    // assessment spec, task shapes, and 31 scored performance atoms.
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

  it("reports the first joining tranche, the reconciled adverb column, and a closed transaction column", () => {
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(197);
    expect(coverage.covered).toBe(103);
    expect(coverage.unmapped).toBe(94);
    expect(coverage.partial).toBe(0);
    // The first tranche closes copulative, alternative, adversative and causal
    // joining. Negative/distributive coordination and complement, relative,
    // infinitive, purpose and simultaneity clauses remain explicit gaps.
    expect(coverage.byCategory["Yojak (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 4,
    });
    // Five are now exact: place, time, polarity, how/where, and why/because.
    // Quantity, manner and connective adverbs remain genuinely partial.
    expect(coverage.byCategory["Kriya-visheshan (adverbs)"]!).toEqual({ enumerated: 8, covered: 5 });
    // And the column this track is actually built around is closed outright.
    expect(coverage.byCategory["Saudo (the counter transaction this track is built around)"]!).toEqual({
      enumerated: 4,
      covered: 4,
    });
    expect(formatExamCoverage(coverage)).toContain(
      "marwadi A1 (partial inventory): 103/197 points covered (52%)",
    );
  }, 60_000);

  it("reconciles later direct owners without converting partial vocabulary into coverage", () => {
    const taught = trackIntroducedAtoms(lessons, "marwadi");
    for (const [id, atoms] of Object.entries(reconciledLaterAtoms)) {
      const point = inventory.points.find((candidate) => candidate.id === id)!;
      expect(point.probe, id).toEqual(atoms);
      for (const atom of atoms) expect(taught.has(atom), `${id}:${atom}`).toBe(true);
    }

    const stillPartial = [
      "MW-A1-Q-02", "MW-A1-F2-02", "MW-A1-F2-13", "MW-A1-F2-14", "MW-A1-F2-15",
      "MW-A1-F3-01", "MW-A1-F3-04", "MW-A1-NG-03", "MW-A1-NG-08", "MW-A1-NG-13",
      "MW-A1-NG-14", "MW-A1-NG-16", "MW-A1-NE-04", "MW-A1-NE-05", "MW-A1-NE-08",
      "MW-A1-NE-11", "MW-A1-NE-13", "MW-A1-NE-14", "MW-A1-NE-15", "MW-A1-NE-16",
      "MW-A1-NE-18", "MW-A1-NE-22", "MW-A1-NE-23", "MW-A1-NE-25", "MW-A1-NE-28",
      "MW-A1-LIP-05",
    ];
    for (const id of stillPartial) {
      const point = inventory.points.find((candidate) => candidate.id === id)!;
      expect(point.probe, `${id} must stay partial`).toBeNull();
      expect(point.note, `${id} needs a reconciled partial note`).toMatch(/NOW|now|still|partial|ONLY|one|five/);
    }

    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  }, 60_000);

  it("keeps the findings a percentage would bury: the numerals that closed, and no word for no", () => {
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
    // THE TEENS HAVE NOW LANDED, and the probe has to name them or the file
    // keeps reporting a count that stops at ten while the corpus reaches
    // twenty. Eleven to nineteen were a SCRIPT gap, priced at four signs, and
    // the four were taught: the standing vowels i, a and u and the retroflex
    // consonant lla. This point was already covered before they landed, so the
    // teens move no coverage number at all -- which is exactly why the probe,
    // and not the percentage, is where they have to be recorded.
    for (const teen of ["IGYAARA", "BAARA", "TERA", "CHAUDA", "PANDARA",
      "SOLA", "SATARA", "ATHARA", "UGHANIS"]) {
      expect(numerals.probe, teen).toContain(`MW-LEX-${teen}-01`);
      expect(numerals.probe, teen).toContain(`MW-SCRIPT-${teen}-01`);
      expect(taught.has(`MW-LEX-${teen}-01`), teen).toBe(true);
      expect(taught.has(`MW-SCRIPT-${teen}-01`), teen).toBe(true);
    }
    for (const sign of ["I-INDEPENDENT", "A-INDEPENDENT", "U-INDEPENDENT", "LLA"]) {
      expect(taught.has(`MW-SCRIPT-${sign}-01`), sign).toBe(true);
    }
    expect(numerals.note).toMatch(/CLOSED BY HL-C350/);
    // The ORDER is the finding, and the note has to carry it: nineteen is
    // ughanis, one short of twenty, so twenty had to be taught first.
    expect(numerals.note).toMatch(/TWENTY WAS TAUGHT BEFORE THE TEENS/);
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
    // RE-CHECKED by the HL-C354 ordinal sweep, which searched again and found
    // nothing citable, so no series was invented. The note records the
    // re-check, because a reader who saw the teens land in the same tranche
    // would otherwise assume the ordinals were simply forgotten.
    expect(ordinals.note).toMatch(/RE-CHECKED by HL-C354's ordinal sweep and STILL UNSOURCED/);
    // (2) POLARITY, RED SINCE CHAPTER 3 AND NOW CLOSED. `haan saa` had a
    //     four-skill performance lesson and no opposite anywhere in 312 files,
    //     so a bargaining course could name a price and not decline one.
    //     Chapter 39 teaches `koni`, sourced to a Swadesh word list collected
    //     for Marwari, and it cost the hand NOTHING: ko + ni is four signs the
    //     track has had since chapter 5. The gap was never a writing problem,
    //     which is why it is asserted here atom by atom rather than as a
    //     boolean.
    const polarity = inventory.points.find((point) => point.id === "MW-A1-ADV-05")!;
    expect(polarity.probe).toContain("MW-LEX-KONI-01");
    expect(polarity.probe).toContain("MW-SCRIPT-KONI-01");
    expect(taught.has("MW-LEX-KONI-01")).toBe(true);
    expect(taught.has("MW-SCRIPT-KONI-01")).toBe(true);
    // …and the move this file called the single most consequential missing one.
    // The walk-away reuses `paachhe milsoo`, taught in chapter 7 as a farewell.
    const decline = inventory.points.find((point) => point.id === "MW-A1-F4-09")!;
    expect(decline.probe).toContain("MW-PERFORMANCE-REFUSAL-FOUR-SKILL-01");
    expect(decline.probe).toContain("MW-SCRIPT-PACHHE-MILSOO-01");
    // WHAT IS STILL NOT CLAIMED, and the note must say so: `koni` is taught as
    // a one-word answer, not inside a sentence, so the negative DECLARATIVE is
    // still open and is now blocked on a frame rather than on a word.
    const negativeDeclarative = inventory.points.find((point) => point.id === "MW-A1-OS-02")!;
    expect(negativeDeclarative.probe).toBeNull();
    expect(negativeDeclarative.note).toMatch(/negative DECLARATIVE/);
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
