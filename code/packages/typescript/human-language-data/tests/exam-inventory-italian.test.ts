// ---------------------------------------------------------------------------
// The Italian A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Five inventories — Italian, Latin, Marwadi, Persian and Portuguese — were the
// five tracks with no inventory at all, and they were written from five
// branches. Each would have appended its describe block to the END of that
// file, and git conflicts on adjacent end-of-file additions however they are
// ordered, so they would have collided pairwise for no reason other than where
// they were parked. Bengali, Arabic and Urdu already live apart for exactly
// this reason; this follows them.
//
// Nothing here is weaker for living apart: it loads the committed file through
// the same strict `loadExamInventory` door and measures it against the same
// corpus as every block in the shared file.
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

describe("the committed Italian A1 inventory", () => {
  const inventory = loadExamInventory("italian", "A1");
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
    const taught = trackIntroducedAtoms(lessons, "italian");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("drops NOTHING, because the proxy and the target are sister languages", () => {
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
    // The claim this file is willing to make that no sibling inventory can:
    // Italian and Spanish are close enough that every one of the 273 proxy
    // points names a real Italian demand. Restating a point around the target's
    // machinery — a final-stress accent where Spanish has an exception-marking
    // one, the ABSENCE of the inverted question mark — is deriving it, which is
    // the lesson Kannada's first draft had to be corrected for, carried to its
    // end. An empty list has to say so out loud or it reads as a walk not done.
    expect(dropped.size).toBe(0);
    expect(proxy.note).toMatch(/EMPTY ON PURPOSE/);
    expect(proxy.note).toMatch(/RESTATING A POINT AROUND\s+THE TARGET LANGUAGE'S MACHINERY IS DERIVING IT/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; italianSpecific?: boolean };
      expect(cast.italianSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "IT-A1-N-07", "IT-A1-N-08", "IT-A1-PRON-03", "IT-A1-F2-17",
      "IT-A1-REG-01", "IT-A1-REG-02", "IT-A1-REG-03", "IT-A1-REG-04",
    ]);
  });

  it("says NO exam envelope exists here, while an external exam DOES exist", () => {
    // Italian is not Tamil. exam-levels.json records a real published exam
    // (CILS / CELI) with no caveat at all, so this file may not borrow Tamil's
    // "no ladder exists" sentence — and it may not claim the exam either,
    // because nobody here has read a CILS content syllabus. The envelope claim
    // is about THIS REPOSITORY: italian/ ships no assessment.json, no
    // task-shapes/ and no mocks/, unlike latin/ and persian/.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.about).toMatch(/CILS/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: NONE EXISTS IN THIS REPOSITORY/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from lesson BODIES, because exam-levels.json carries NO caveat", () => {
    // Both tracks in this batch with a published external exam — italian and
    // portuguese — carry no caveat, so there is nothing to import and nothing
    // to defer to. Unlike Bengali's, italian's `register` frontmatter is not a
    // pure artefact: six of 93 lessons declare something other than `neutral`.
    // It is still not an index, because it misses come-stai, come-ti-chiami and
    // buongiorno, all of which teach the tu/Lei contrast in their bodies.
    expect(inventory.about).toMatch(
      /THE REGISTER COLUMN IS DERIVED FROM LESSON BODIES, NOT FROM FRONTMATTER, AND THE FRONTMATTER FIELD IS\s+HALF-APPLIED RATHER THAN AN ARTEFACT/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Registro"));
    expect(register).toHaveLength(4);
    expect(register.some((point) => point.id === "IT-A1-REG-04")).toBe(true);
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

  it("reports the tranche that closed four columns outright, and what it did not touch", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(292);
    expect(coverage.covered).toBe(164);
    expect(coverage.unmapped).toBe(128);
    expect(coverage.partial).toBe(0);

    // FOUR COLUMNS CLOSED OUTRIGHT by the chapter 29-35 tranche. Two of them
    // were the flat zeros this file reported when it was written -- nothing in
    // 93 lessons could point at anything, and nothing could be owned -- and one
    // of them, the adverb column, was 2 of 8 and is the reason the negation and
    // joining chapters were ordered the way they were: polarity, place deixis,
    // time deixis, quantity and the connective adverbs all sit in it.
    expect(coverage.byCategory["I dimostrativi (pointing)"]!).toEqual({ enumerated: 3, covered: 3 });
    expect(coverage.byCategory["I possessivi (possession)"]!).toEqual({ enumerated: 3, covered: 3 });
    expect(coverage.byCategory["L'avverbio (adverbs)"]!).toEqual({ enumerated: 8, covered: 8 });
    expect(coverage.byCategory["La frase semplice (the simple sentence)"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    expect(coverage.byCategory["Funzioni: dare e chiedere informazioni"]!).toEqual({
      enumerated: 5,
      covered: 5,
    });

    // THE JOINING COLUMN, 1 of 13 to 10 of 13. Its one previously covered point
    // was covered sideways -- the written-accent lesson had to gloss plain `e`
    // as "and" in order to contrast it with `è`. `ma`, `o`, `perché`, `quando`,
    // `né` and `che` were all zero-occurrence words in this track and now have
    // lessons. What stays open is the distributive uno … l'altro (IT-A1-CJ-05)
    // and the two infinitive-clause points, which need volere, potere and
    // dovere -- none of which this tranche taught.
    expect(coverage.byCategory["Congiunzione e subordinazione (joining and subordination)"]!).toEqual({
      enumerated: 13,
      covered: 10,
    });
    expect(coverage.byCategory["Il pronome (pronouns)"]!).toEqual({ enumerated: 10, covered: 9 });
    expect(coverage.byCategory["Funzioni: opinioni, atteggiamenti e conoscenza"]!).toEqual({
      enumerated: 17,
      covered: 14,
    });

    // UNTOUCHED, AND DELIBERATELY. The specific-notion column is 66 uncovered
    // points of pure vocabulary -- clothing, transport, the meals of the day,
    // the workplace -- and no amount of grammar closes any of it. It is where
    // the next tranche has to go, and the ratio says why this one did not:
    // these 66 points want roughly 250 words, where the 50 points below cost 35.
    expect(coverage.byCategory["Nozioni specifiche (specific notions)"]!).toEqual({
      enumerated: 78,
      covered: 12,
    });
    // Punctuation is still 0 of 5 inside this column: not one mark is taught in
    // 142 lessons, and every mark on every page is used correctly.
    expect(coverage.byCategory["Ortografia e punteggiatura (spelling and punctuation)"]!).toEqual({
      enumerated: 16,
      covered: 8,
    });
    expect(coverage.byCategory["Il verbo (the verb)"]!).toEqual({ enumerated: 19, covered: 15 });
    expect(coverage.byCategory["Il sostantivo (the noun)"]!).toEqual({ enumerated: 8, covered: 8 });
    expect(formatExamCoverage(coverage)).toContain(
      "italian A1 (partial inventory): 164/292 points covered (56%)",
    );
  }, 60_000);

  it("keeps the findings that no coverage percentage would surface", () => {
    // (1) NEGATION, WHICH WAS ONE WORD. `non` occurred in 2 of 93 files, both
    //     inside the frozen `non c'è male`, whose own lesson deferred negation
    //     and the `c'è` construction BY NAME. Both halves of that deferral are
    //     now paid, four lessons apart, and the single `non` lesson is what
    //     unlocked the negative declarative, disagreement, `né … né`, `neanche`,
    //     `non capisco` and `non so` between them.
    const negation = inventory.points.find((point) => point.id === "IT-A1-OS-02")!;
    expect(negation.probe).toEqual(["IT-GRAMMAR-NON-02", "IT-ETYMON-NON-03"]);
    expect(negation.note).toMatch(/CLOSED, AND IT WAS ONE WORD/);
    // (2) THE REPAIR COLUMN, both halves. Rising `Prego?` had a lesson of its
    //     own from the start; `non capisco` needed a negator the track did not
    //     have, and the two are now taught in sequence and in that order.
    const repair = inventory.points.find((point) => point.id === "IT-A1-F6-05")!;
    expect(repair.probe).toEqual(["IT-PRAGMATIC-PREGO-COME-AGAIN-06"]);
    const understanding = inventory.points.find((point) => point.id === "IT-A1-F2-17")!;
    expect(understanding.probe).toEqual(["IT-PRAGMATIC-NON-CAPISCO-02", "IT-LEX-CAPIRE-02"]);
  });

  it("records the two points that were RE-PROBED rather than authored against", () => {
    // A note in an inventory decays: it is a claim about the corpus on the day
    // it was written, and the corpus moves. Two of this file's notes were wrong
    // when the tranche went looking, and both were wrong in the same direction
    // -- the thing was already taught, in another guise, and the note had not
    // looked at the lesson that taught it. Neither point cost a new lesson.
    //
    // Pinned here, and not merely fixed, because the failure mode is silent: a
    // point that is covered and reads `null` is indistinguishable from real debt
    // and gets a lesson written for it.
    const pronouns = inventory.points.find((point) => point.id === "IT-A1-PRON-02")!;
    expect(pronouns.probe).toEqual(["IT-GRAMMAR-ESSERE-03"]);
    expect(pronouns.note).toMatch(/COVERED ALL ALONG, AND THE OLD NOTE WAS WRONG/);
    const knowing = inventory.points.find((point) => point.id === "IT-A1-F2-13")!;
    expect(knowing.probe).toContain("IT-NOTICE-INCONTRARE-04");
    expect(knowing.note).toMatch(/THE OLD NOTE WAS WRONG ABOUT THE OTHER/);
  });

  it("closes five points with NO NEW WORD, by spending what the track already owned", () => {
    // The cheapest points in the tranche. Each one is a pattern the track had
    // both halves of and had never joined, or a word it had used and never
    // named. They are pinned as a set because "one new item per lesson" makes
    // it easy to forget that the best lesson sometimes introduces none.
    const spent: Record<string, string> = {
      // `che` four lessons old + `buono`, which had lived inside buongiorno,
      // buonasera and buonanotte since the first chapter and was never once
      // predicated of anything.
      "IT-A1-PRON-09": "IT-GRAMMAR-CHE-BUONO-02",
      // essere + adjective: both halves taught a hundred lessons apart.
      "IT-A1-ADJ-03": "IT-GRAMMAR-BUONO-PREDICATO-03",
      // `per` arrived inside `perché` one lesson earlier; the infinitives were
      // all taught between chapters 5 and 20.
      "IT-A1-CJ-12": "IT-GRAMMAR-PER-02",
      // The demonstrative evicts the article; the possessive keeps it.
      "IT-A1-DEM-03": "IT-GRAMMAR-QUESTO-CAFFE-02",
      // Addressing a named person: every practice exchange in the track opened
      // with a bare greeting and never with a name.
      "IT-A1-SN-04": "IT-GRAMMAR-SIGNORE-SIGNORA-03",
    };
    for (const [pointId, atom] of Object.entries(spent)) {
      const point = inventory.points.find((candidate) => candidate.id === pointId)!;
      expect(point.probe, pointId).toContain(atom);
    }
  });
});
