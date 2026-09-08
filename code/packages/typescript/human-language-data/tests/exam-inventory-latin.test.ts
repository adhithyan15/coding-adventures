// ---------------------------------------------------------------------------
// The Latin A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Five inventories — Italian, Latin, Marwadi, Persian and Portuguese — were the
// five tracks with no inventory at all, and they were written from five
// branches. Each would have appended its describe block to the END of that
// file, and git conflicts on adjacent end-of-file additions however they are
// ordered. Bengali, Arabic and Urdu already live apart for exactly this reason.
//
// WHAT THIS SUITE HAS TO GUARD THAT THE OTHERS DO NOT
// exam-levels.json's caveat for latin says the CEFR construct is the wrong one
// for a language learned almost entirely for reading. That caveat is carried in
// the file as a deliberately uncovered point rather than as a footnote, and a
// test pins it there, because a footnote can be deleted without a suite noticing.
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

describe("the committed Latin A1 inventory", () => {
  const inventory = loadExamInventory("latin", "A1");
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
    const taught = trackIntroducedAtoms(lessons, "latin");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total, and drops SIX points for a chronological reason", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    for (const id of dropped) expect(sourceIds.has(id), `notTransferred names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
    // Five of the six are ANACHRONISMS, which no other inventory in this series
    // has dropped a point for: all 112 lessons declare `variety: classical`, and
    // a Classical Latin A1 inventory cannot demand the telephone. Everything
    // that is merely a grammatical mismatch is RESTATED and therefore derived —
    // the articles Latin has not got, the punctuation Roman texts did not use,
    // the possessive that is a dative case rather than a word.
    expect([...dropped].sort()).toEqual([
      "A1-NE09-03", "A1-NE09-05", "A1-NE09-06", "A1-NE16-02", "A1-NE18-05", "A1-O1-06",
    ]);
    expect(proxy.note).toMatch(/they are ANACHRONISMS rather than grammatical mismatches/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; latinSpecific?: boolean };
      expect(cast.latinSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "LA-A1-N-07", "LA-A1-PRON-04", "LA-A1-V-09", "LA-A1-V-13", "LA-A1-V-14", "LA-A1-F2-17",
      "LA-A1-LEC-01", "LA-A1-LEC-02", "LA-A1-LEC-03", "LA-A1-LEC-04",
      "LA-A1-TES-01", "LA-A1-TES-02", "LA-A1-TES-03",
    ]);
  });

  it("CARRIES THE READING CAVEAT AS AN UNCOVERED POINT, not as a footnote", () => {
    // core/exam-levels.json, tracks.latin: "CEFR is built around communicative
    // can-do statements, and Latin is learned almost entirely for reading."
    // The 54 functional points in this file therefore measure something Latin is
    // not learned for. That has to be IN the file, where a reader of the
    // coverage number will meet it, and it has to be uncovered, so that the
    // number it qualifies cannot quietly absorb it. Sanskrit's inventory set the
    // precedent with its point about the parīkṣā it cannot measure.
    const construct = inventory.points.find((point) => point.id === "LA-A1-LEC-01")!;
    expect(construct.probe).toBeNull();
    expect(construct.note).toMatch(/THIS INVENTORY MEASURES THE WRONG CONSTRUCT|no living-speaker construct/);
    expect(construct.label).toMatch(/MEASURES THE WRONG CONSTRUCT/);
    expect(inventory.about).toMatch(/THE CAVEAT IN exam-levels\.json IS LOAD-BEARING/);
    // …and the reading construct it names gets a column of its own, one of whose
    // four points the corpus actually meets: every noun is cited with its
    // genitive and every verb with its infinitive, which is dictionary skill.
    const reading = inventory.points.filter((point) => point.category.startsWith("Lectio"));
    expect(reading).toHaveLength(4);
    expect(reading.find((point) => point.id === "LA-A1-LEC-04")!.probe).not.toBeNull();
  });

  it("keeps the attestation column, which only a DEAD language needs", () => {
    const attested = inventory.points.filter((point) => point.category.startsWith("Testimonium"));
    expect(attested).toHaveLength(3);
    // The track labels its own inventions: bonam noctem, bonum māne and bonum
    // vesperum are marked modern pedagogical formulas, nihil est a modern
    // convention, crās tē vidēbō built on an attested pattern but not itself
    // attested — and propediem tē vidēbō as Cicero's own, twice, with the
    // letters named. No proxy point exists for any of that.
    expect(attested.find((point) => point.id === "LA-A1-TES-01")!.probe).toEqual([
      "LA-GRAMMAR-BONAM-NOCTEM-02", "LA-PRAGMATICS-BONUM-MANE-01",
      "LA-PRAGMATICS-NIHIL-EST-03", "LA-PRAGMATICS-CRAS-TE-VIDEBO-03",
    ]);
  });

  it("says its exam envelope is project-defined, and that no modern exam exists", () => {
    // Unlike italian/ and portuguese/, latin/ DOES ship assessment-spec.md,
    // assessment.json and task-shapes/a1.json, so there is an envelope and it is
    // internal. The spec says in its own words that having the A1 task inventory
    // "alone is not readiness evidence"; this file must not claim more.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/no modern proficiency exam/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: ONE EXISTS AND IT IS PROJECT-DEFINED/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
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

  it("reports the joining column REBUILT, and the ordinal column CLOSED", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(270);
    expect(coverage.covered).toBe(135);
    expect(coverage.unmapped).toBe(135);
    expect(coverage.partial).toBe(0);
    // WAS 0 OF 12, AND IT WAS THE ELEVENTH FLAT ZERO IN THIS SERIES. `et`
    // occurred twice in 112 files and both were inside quoted classical
    // sentences; `sed` only inside the Cicero letter; `aut`, `vel`, `neque`,
    // `quod`, `quia`, `cum`, `ubi` and `quī` all returned zero.
    //
    // The tranche chose against those points rather than by topic: `et` with
    // the enclitic `-que`, `sed`, `aut` with `vel` beside it, `quī` with its
    // relative clause, `quia`, `quandō`, and `volō` for the complementary
    // infinitive. Seven of twelve, from one chapter-and-a-bit of vocabulary.
    expect(coverage.byCategory["Coniunctio et subiunctio (joining and subordination)"]!).toEqual({
      enumerated: 12,
      covered: 7,
    });
    // Was 0 of 3 and is now closed outright: hic, iste and ille, one lesson
    // each, with the three-way system Spanish still keeps.
    expect(coverage.byCategory["Demonstrativa (pointing)"]!).toEqual({ enumerated: 3, covered: 3 });
    // Unmoved, and named so the next tranche knows where to go: the case system
    // is what actually lets somebody read Latin and a vocabulary tranche cannot
    // close it — LA-A1-CAS-02, -03, -05, -06 and -07 all need a grammar lesson.
    expect(coverage.byCategory["Casus (the case system)"]!).toEqual({ enumerated: 7, covered: 2 });
    // THE ORDINAL TRANCHE, measured here rather than asserted in prose. HL-C350
    // found ordinals the weakest column in the whole corpus — twenty tracks
    // enumerate an ordinal point and eighteen left it uncovered — and Latin was
    // one of the cheap ones, because the cardinals to ten were already taught in
    // chapter 2 and Quīntīlis and Sextīlis were already glossed with their
    // ordinal sense in chapter 11 without the words behind them ever being
    // given. Numeri goes 3 of 7 to 4 of 7 on LA-A1-Q-05 alone; the two that
    // moved in Notiones generales are LA-A1-NG-15 and LA-A1-NG-19, both closed
    // by `ante` and `post`. LA-A1-Q-03 (cardinals above twenty) is untouched and
    // stays named, because vīgintī is still the ceiling.
    expect(coverage.byCategory["Numeri (quantity and number)"]!).toEqual({ enumerated: 7, covered: 4 });
    expect(coverage.byCategory["Notiones generales (general notions)"]!).toEqual({
      enumerated: 36,
      covered: 19,
    });
    expect(formatExamCoverage(coverage)).toContain(
      "latin A1 (partial inventory): 135/270 points covered (50%)",
    );
  }, 60_000);

  it("records what the tranche closed, and what it deliberately did not", () => {
    // (1) NEGATION. `nōn` was taught as a WORD on the first day and never once
    //     attached to a verb, so `nōn intellegō` was unsayable although both
    //     halves were separately taught. The nesciō lesson states the rule in
    //     one line and works it on four verbs, and it is the single cheapest
    //     structural repair the inventory named.
    const negation = inventory.points.find((point) => point.id === "LA-A1-OS-02")!;
    expect(negation.probe).toEqual(["LA-GRAMMAR-C48-NON-VERB-01"]);
    expect(negation.note).toMatch(/CLOSED BY THE TRANCHE/);
    // …and polarity was already covered before the tranche, in chapter 1. The
    // two coming apart is the finding; both halves are now present.
    expect(inventory.points.find((point) => point.id === "LA-A1-ADV-05")!.probe).not.toBeNull();

    // (2) REPAIR. There was NONE — dīcō and quaesō were both taught and no
    //     lesson joined them. `iterum, quaesō` closes it.
    const repair = inventory.points.find((point) => point.id === "LA-A1-F6-05")!;
    expect(repair.probe).toEqual(["LA-LEX-C53-SCHOOL-01", "LA-ETYMON-QUAESO-01"]);
    expect(repair.note).toMatch(/IT WAS THE LARGEST HOLE IN THE COURSE/);
    // The other half of the repair column — saying you have not understood —
    // needed no new word at all, only the negation rule.
    expect(inventory.points.find((point) => point.id === "LA-A1-F2-17")!.probe)
      .toEqual(["LA-LEX-INTELLEGO-01", "LA-GRAMMAR-C48-NON-VERB-01"]);

    // (3) WHAT THE TRANCHE COULD NOT CLOSE, kept uncovered rather than fudged.
    //     Asking about ability needs a polar question particle, which is
    //     grammar and not vocabulary; the reading construct the caveat names is
    //     still the wrong measure; and the case system is untouched.
    expect(inventory.points.find((point) => point.id === "LA-A1-F2-15")!.probe).toBeNull();
    expect(inventory.points.find((point) => point.id === "LA-A1-LEC-01")!.probe).toBeNull();
    expect(inventory.points.find((point) => point.id === "LA-A1-CAS-02")!.probe).toBeNull();
  });

  it("closes every point it closes with an atom the tranche actually introduces", () => {
    // The tranche was chosen AGAINST this file, so the check that matters is
    // that the probes now point at real new atoms rather than at hopeful ids.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "latin");
    const tranche = [...taught].filter((atom) => /^LA-(LEX|GRAMMAR)-C(4[89]|5[0-4])-/.test(atom));
    expect(tranche).toHaveLength(36);
    const probed = new Set(inventory.points.flatMap((point) => point.probe ?? []));
    const unused = tranche.filter((atom) => !probed.has(atom));
    // EVERY one of the 36 earns its place. That is the property "ranked by
    // points-per-item" is supposed to produce, and an inequality would let a
    // future word in that closes nothing — so this is an equality against the
    // empty list, named rather than counted.
    expect(unused, "tranche atoms that no point probes").toEqual([]);
  }, 60_000);
});
