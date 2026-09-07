// ---------------------------------------------------------------------------
// The Portuguese A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Five inventories — Italian, Latin, Marwadi, Persian and Portuguese — were the
// five tracks with no inventory at all, and they were written from five
// branches. Each would have appended its describe block to the END of that file,
// and git conflicts on adjacent end-of-file additions however they are ordered.
// Bengali, Arabic and Urdu already live apart for exactly this reason.
//
// WHAT THIS SUITE HAS TO GUARD THAT THE OTHERS DO NOT
// This is the best-covered track in the batch and the joining column that has
// been near-empty in every previous measurement is 4 of 12 here. A number that
// breaks a run of eleven has to be pinned, or the next reader will assume it is
// a mistake — and the register column has to be pinned as MEASURED, because
// both frontmatter fields are artefacts and exam-levels.json carries no caveat
// to fall back on.
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

describe("the committed Portuguese A1 inventory", () => {
  const inventory = loadExamInventory("portuguese", "A1");
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
    const taught = trackIntroducedAtoms(lessons, "portuguese");
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
    expect(dropped.size).toBe(0);
    expect(proxy.note).toMatch(/EMPTY ON PURPOSE/);
    expect(proxy.note).toMatch(/RESTATING A POINT AROUND THE TARGET LANGUAGE'S MACHINERY IS DERIVING IT/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; portugueseSpecific?: boolean };
      expect(cast.portugueseSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "PT-A1-F2-16", "PT-A1-REG-01", "PT-A1-REG-02", "PT-A1-REG-03", "PT-A1-REG-04",
    ]);
  });

  it("says NO exam envelope exists here, while an external exam DOES exist", () => {
    // Like italian/, portuguese/ ships no assessment.json, no task-shapes/ and
    // no mocks — `npm run plan` still lists writing one as outstanding work —
    // while exam-levels.json records a real published exam with no caveat. The
    // file may claim neither an internal contract nor the external syllabus.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/CAPLE/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: NONE EXISTS IN THIS REPOSITORY/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from lesson BODIES, because BOTH frontmatter fields are artefacts", () => {
    // All 102 lessons declare `register: neutral` and all 102 declare
    // `variety: standard-contemporary`, so neither field carries information —
    // the same shape Bengali's has. exam-levels.json carries NO caveat for
    // portuguese either, so there was nothing to import and nothing to defer to.
    expect(inventory.about).toMatch(
      /THE REGISTER COLUMN IS DERIVED FROM LESSON BODIES, NOT FROM FRONTMATTER, AND BOTH FRONTMATTER FIELDS ARE\s+ARTEFACTS/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Registo"));
    expect(register).toHaveLength(4);
    expect(register.find((point) => point.id === "PT-A1-REG-04")!.note)
      .toMatch(/both fields are artefacts and carry no information at all/);
    // …and the finding the measurement produced that no field could have: the
    // corpus teaches TWO major varieties at once and never says so.
    const variety = inventory.points.find((point) => point.id === "PT-A1-REG-03")!;
    expect(variety.probe).not.toBeNull();
    expect(variety.note).toMatch(/being taught two dialects at once and has not been told/);
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

  it("reports the FIRST non-empty joining column in this series, and a closed sentence column", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(208);
    expect(coverage.covered).toBe(120);
    expect(coverage.unmapped).toBe(88);
    expect(coverage.partial).toBe(0);
    // 4 of 12, where eleven tracks before this one reported 0 or 1. `e` is
    // GLOSSED in chapter 2 — "the glue is e você?, and e continues Latin et" —
    // `que` is taught three separate times (ter que as "the link", Penso que
    // sim, Espero que sim), and `ou` is on the page inside mais ou menos. The
    // hole is `mas`, which returns zero occurrences in 102 files: a learner can
    // add, offer an alternative, report an opinion and state an obligation, and
    // cannot set one thing against another.
    expect(coverage.byCategory["Coordenacao e subordinacao (joining and subordination)"]!).toEqual({
      enumerated: 12,
      covered: 4,
    });
    // Nothing can be pointed at and nothing can be owned — the two columns that
    // are empty in almost every track measured.
    expect(coverage.byCategory["Os demonstrativos (pointing)"]!).toEqual({ enumerated: 3, covered: 0 });
    expect(coverage.byCategory["Os possessivos (possession)"]!).toEqual({ enumerated: 3, covered: 0 });
    // And the columns this track closes outright.
    expect(coverage.byCategory["A frase simples (the simple sentence)"]!).toEqual({ enumerated: 7, covered: 7 });
    expect(coverage.byCategory["O substantivo (the noun)"]!).toEqual({ enumerated: 7, covered: 7 });
    expect(formatExamCoverage(coverage)).toContain(
      "portuguese A1 (partial inventory): 120/208 points covered (58%)",
    );
  }, 60_000);

  it("keeps the findings a percentage would bury", () => {
    // (1) NEGATION IS ONE SENTENCE. `não sei` is introduced in chapter 18 with
    //     the note that it is the sentence a learner will need first — and it
    //     is the whole of negation in 102 files. The operation is on the page,
    //     attached to a verb, which is more than the Italian track can say.
    const negation = inventory.points.find((point) => point.id === "PT-A1-OS-02")!;
    expect(negation.probe).toEqual(["PT-LEX-SABER-CONHECER-02"]);
    expect(negation.note).toMatch(/COVERED BY ONE SENTENCE AND IT IS THE RIGHT ONE/);
    // (2) REPAIR is EMPTY here, and Italian's — the weaker track — is not.
    //     `como` is taught in chapter 2 as the question word, so a rising
    //     `Como?` is one line away from an existing lesson.
    const repair = inventory.points.find((point) => point.id === "PT-A1-F6-05")!;
    expect(repair.probe).toBeNull();
    expect(repair.note).toMatch(/NOTHING/);
    // …and the understanding half is empty too, although entender AND
    // compreender are both taught and `não` is taught attached to a verb.
    expect(inventory.points.find((point) => point.id === "PT-A1-F2-16")!.probe).toBeNull();
    // (3) TWO OF THE THREE CONJUGATIONS ARE NEVER SET OUT. Nine -er and -ir
    //     verbs are taught as words and no lesson gives their endings.
    const conj = inventory.points.find((point) => point.id === "PT-A1-V-02")!;
    expect(conj.probe).toBeNull();
    expect(conj.note).toMatch(/NEVER SET OUT/);
    // (4) NO WORD FOR PLEASE, in a track that teaches four thanking words —
    //     and chapter 20 explicitly declines to give the verb for asking for a
    //     coffee.
    const please = inventory.points.find((point) => point.id === "PT-A1-F4-02")!;
    expect(please.probe).toBeNull();
    expect(please.note).toMatch(/por favor returns ZERO occurrences/);
  });
});
