// ---------------------------------------------------------------------------
// The gate that answers the owner's question (HL-C128).
//
//     "The goal is not whether something touches some level. The goal is can
//      someone pass that level of exam with just reading the book and slowly
//      following its gentle ramp."
//
// Every other measurement in this package walks our own lessons, so every one
// of them rises when a lesson is added — including a lesson on something the
// exam does not test. This one resolves the corpus against an external, finite
// list, so it can fall, and it can stay flat while the corpus grows.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../src/loader.js";
import {
  measureExamCoverage,
  formatExamCoverage,
  trackIntroducedAtoms,
  type ExamInventory,
} from "../src/exam-inventory.js";
import { parseLesson } from "../src/parse.js";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

function lesson(id: string, introduces: string[]) {
  return parseLesson(
    `---
schema_version: 2
id: ${id}
spine_node: HELLO
sequence: 10
chapter: 1
type: grammar
headword: prueba
gloss: a fixture
concept_tag: ES-TEST
prerequisites: []
duration:
  max_seconds: 60
requires:
  knowledge: []
introduces:
  knowledge: [${introduces.join(", ")}]
practises:
  knowledge: []
skills: [reading]
modes: [interpretive]
strands: [language-focus]
register: neutral
variety: general
---

# prueba

## Warm-up

[PAUSE 2s] Recall it.
`,
    "spanish",
  );
}

const FIXTURE: ExamInventory = {
  version: 1,
  language: "spanish",
  level: "A1",
  about: "fixture",
  source: "fixture",
  probeSemantics: "fixture",
  points: [
    { id: "P-1", category: "Uno", label: "both atoms present", probe: ["ES-A", "ES-B"] },
    { id: "P-2", category: "Uno", label: "one atom missing", probe: ["ES-A", "ES-MISSING"] },
    { id: "P-3", category: "Dos", label: "nothing corresponds", probe: null },
  ],
};

describe("probe semantics", () => {
  const corpus = [lesson("ES-1", ["ES-A"]), lesson("ES-2", ["ES-B"])];

  it("covers a point only when EVERY atom of its probe is introduced", () => {
    const coverage = measureExamCoverage(FIXTURE, corpus);
    const byId = new Map(coverage.points.map((point) => [point.id, point]));

    expect(byId.get("P-1")!.covered).toBe(true);
    // Half a paradigm is not half a mark in an exam, so it is not half a point
    // here. P-2 holds ES-A and is still uncovered.
    expect(byId.get("P-2")!.covered).toBe(false);
    expect(byId.get("P-2")!.missingAtoms).toEqual(["ES-MISSING"]);
  });

  it("counts an unmapped point as UNCOVERED rather than skipping it", () => {
    // The tempting reading of `probe: null` is "not yet classified, exclude from
    // the denominator". That would let the percentage be improved by deleting
    // the mapping — the one edit that changes nothing about what a reader knows.
    const coverage = measureExamCoverage(FIXTURE, corpus);
    expect(coverage.enumerated).toBe(3);
    expect(coverage.points.find((point) => point.id === "P-3")!.covered).toBe(false);
    expect(coverage.unmapped).toBe(1);
    expect(coverage.covered).toBe(1);
  });

  it("falls when a probed atom stops being introduced", () => {
    // The property an annotation cannot have. Retire the lesson that introduces
    // ES-B and P-1 must stop counting, with no edit to the inventory.
    const before = measureExamCoverage(FIXTURE, corpus);
    const after = measureExamCoverage(FIXTURE, [lesson("ES-1", ["ES-A"])]);
    expect(before.covered).toBe(1);
    expect(after.covered).toBe(0);
  });

  it("reads atoms through the shared helper, so the flat dotted key is honoured", () => {
    // `introduces.knowledge` is a FLAT frontmatter key. Reading it as a nested
    // object returns undefined for every lesson in the corpus and would report
    // 0% coverage — which looks like a devastating finding rather than a bug.
    expect(trackIntroducedAtoms(corpus, "spanish")).toEqual(new Set(["ES-A", "ES-B"]));
    expect(trackIntroducedAtoms(corpus, "french").size).toBe(0);
  });
});

describe("the committed A1 inventory", () => {
  const inventory = loadExamInventory("spanish", "A1");

  it("refuses an empty probe, because an empty probe scores as covered", () => {
    // `probe: []` asks for zero atoms, every one of which is trivially present,
    // so the point would be reported covered while demonstrating nothing. It is
    // the one malformed shape that moves the number in the flattering direction,
    // which is why the loader rejects it rather than tolerating it.
    for (const point of inventory.points) {
      expect(Array.isArray(point.probe) ? point.probe.length : 1).toBeGreaterThan(0);
    }
  });

  it("names every probe atom in the corpus convention, so a typo is visible", () => {
    // A misspelt atom resolves to "not introduced", which is fail-safe but
    // silent. Shape-checking the name catches the common half of that.
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) {
        expect(atom, `${point.id} probes '${atom}'`).toMatch(/^ES-[A-Z0-9-]+$/);
      }
    }
  });

  it("classifies every unmapped point, so a new null cannot slip in unnoticed", () => {
    // This test used to hold a hand-written list of nulls that were PARTLY true
    // and therefore needed a note saying which half existed. Chapters 257-261
    // closed the last of those, and emptying the list left `for (const id of
    // [])` behind — a loop that never runs, in a test with no other assertion,
    // passing unconditionally. The invariant it existed for was enforced by
    // nothing, and a future null probe with no note would have sailed through.
    //
    // So it is derived now instead of listed. Pinning the exact set of nulls
    // means any NEW one fails here and has to be classified deliberately —
    // which is the whole job this test was meant to do.
    const unmapped = inventory.points.filter((point) => point.probe === null).map((point) => point.id);
    // A1 is complete, so this is now the empty set -- and it stays an assertion
    // rather than a vacuous loop: adding ANY new null probe fails here and has
    // to be justified, which is exactly what this test is for.
    expect(unmapped.sort()).toEqual([]);

    // None of the four is partly true: each is absent outright, so none needs a
    // note explaining which half exists. If a partly-true null is ever added,
    // the assertion above fails first and forces that decision into the open.
    for (const id of unmapped) {
      const point = inventory.points.find((candidate) => candidate.id === id)!;
      expect(point.probe, `${id} must stay null or gain a probe deliberately`).toBeNull();
    }
  });
});

describe("the loader refuses what would move the number the wrong way", () => {
  it("refuses a language or level that could escape the curriculum root", () => {
    // `join` NORMALISES `..` inside an interpolated filename rather than
    // rejecting it, so before the guard `level = "../../../../etc/shadow"`
    // resolved to `/etc/shadow.json`. The trailing `.json` was no protection:
    // `.docker/config.json` holds registry credentials. Found by a security
    // review of this very PR, while the only callers still passed literals.
    expect(() => loadExamInventory("../../../../../../etc/passwd", "a1")).toThrow(/unsafe/);
    expect(() => loadExamInventory("spanish", "../../../../../../root/.docker/config")).toThrow(/unsafe/);
    expect(() => loadExamInventory("spanish", "A1/../A1")).toThrow(/unsafe/);
    // The control: the legitimate call still works, so the guard is not simply
    // refusing everything.
    expect(loadExamInventory("spanish", "A1").points.length).toBeGreaterThan(0);
  });
});

describe("a hostile inventory cannot corrupt the process", () => {
  function hostile(category: string): ExamInventory {
    return {
      version: 1,
      language: "spanish",
      level: "A1",
      about: "fixture",
      source: "fixture",
      probeSemantics: "fixture",
      points: [{ id: "X", category, label: "attack", probe: null }],
    };
  }

  it("does not write onto Object.prototype through a category name", () => {
    // With a plain `{}` accumulator, `byCategory["__proto__"] ??= {...}` finds
    // Object.prototype (truthy), skips the assignment, and `+= 1` lands on the
    // prototype itself — after which EVERY object in the process inherits
    // `enumerated: NaN`. The accumulator is `Object.create(null)` for this
    // reason, and this test is what stops that turning back into `{}`.
    const coverage = measureExamCoverage(hostile("__proto__"), []);
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, "enumerated")).toBe(false);
    expect(({} as Record<string, unknown>).enumerated).toBeUndefined();

    // And the report stays self-consistent: the category is a real own key, so
    // the per-category lines still sum to the total. Under the old accumulator
    // the point vanished from `byCategory` while still counting in `enumerated`.
    expect(Object.keys(coverage.byCategory)).toEqual(["__proto__"]);
    const summed = Object.values(coverage.byCategory).reduce((total, entry) => total + entry.enumerated, 0);
    expect(summed).toBe(coverage.enumerated);
  });

  it("hands back a NORMAL object, so a consumer can still call hasOwnProperty", () => {
    // The first fix here was `Object.create(null)`, which closes the sink but
    // leaks into a public return type: `byCategory.hasOwnProperty(x)` and
    // `String(byCategory)` both throw on a null-prototype object. The Map plus
    // `Object.fromEntries` is safe for the same reason and normal to hold.
    const coverage = measureExamCoverage(hostile("__proto__"), []);
    expect(Object.getPrototypeOf(coverage.byCategory)).toBe(Object.prototype);
    expect(Object.prototype.hasOwnProperty.call(coverage.byCategory, "__proto__")).toBe(true);
    expect(() => JSON.stringify(coverage.byCategory)).not.toThrow();
    // And the safety survives the change: still an own key, still no pollution.
    expect(({} as Record<string, unknown>).enumerated).toBeUndefined();
  });

  it("reports 0% rather than NaN% for an inventory with no points", () => {
    const coverage = measureExamCoverage({ ...hostile("Uno"), points: [] }, []);
    expect(coverage.percent).toBe(0);
    expect(Number.isNaN(coverage.percent)).toBe(false);
  });
});

describe("the loader refuses a malformed file rather than crashing downstream", () => {
  // These exercise the SECONDARY defenses, which the primary tests above do not
  // reach: a caller cannot hand `loadExamInventory` a bad object, only a bad
  // file, so testing them needs a file.
  function withTempInventory<T>(body: (root: string) => T): T {
    const root = mkdtempSync(join(tmpdir(), "exam-inventory-"));
    mkdirSync(join(root, "core"));
    try {
      return body(root);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }
  const write = (root: string, value: unknown) =>
    writeFileSync(join(root, "core", "exam-inventory-es-a1.json"), JSON.stringify(value));

  it("names the file when `points` is missing or empty", () => {
    withTempInventory((root) => {
      write(root, { version: 1, language: "spanish", level: "A1" });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/no non-empty 'points' array/);
      write(root, { version: 1, points: [] });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/no non-empty 'points' array/);
    });
  });

  it("refuses a reserved category name before it reaches the accumulator", () => {
    withTempInventory((root) => {
      write(root, { version: 1, points: [{ id: "X", category: "__proto__", label: "l", probe: null }] });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/reserved category name/);
    });
  });

  it("refuses a duplicate point id, which would double-count a category", () => {
    withTempInventory((root) => {
      write(root, {
        version: 1,
        points: [
          { id: "X", category: "Uno", label: "l", probe: null },
          { id: "X", category: "Uno", label: "l", probe: null },
        ],
      });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/duplicate point id/);
    });
  });
});

describe("what the corpus actually covers", () => {
  it("pins A1 coverage, which is the number this project is judged on", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(loadExamInventory("spanish", "A1"), lessons);

    // First measured at 53/85 over 220 chapters — a curriculum that had climbed
    // to a B2 node while missing 62% of... no: while holding only 62% of the A1
    // grammar an examiner may ask for. Chapters 221-225 then taught the
    // demonstratives, which had been absent ENTIRELY (este/ese/aquel and the
    // neuters, 3 of 3 points), taking it to 56/85.
    //
    // Chapters 226-229 then taught the degree words -- `muy`, `bastante`, `mal`
    // -- closing four more points across three categories, and taking
    // `El sintagma adjetival` off the floor at last.
    //
    // Chapters 230-235 then closed the contractions, `quien`, and both missing
    // coordinators -- which finished `Coordinacion` outright.
    //
    // Chapters 236-240 then taught the gerund and the personal `a`. A third
    // point, A1-V-03, was DELIBERATELY left open: chapter 238 teaches the
    // progressive and contrasts it with the plain present, which is adjacent to
    // that point but is not it, and closing it with progressive atoms would be
    // exactly the gaming this gate exists to catch.
    //
    // Chapters 241-245 then paid HL-C127's debt: the vosotros preterite and the
    // imperfect plural, both of which chapter 204 promised the reader in print.
    // Both past tenses are now complete paradigms.
    //
    // Chapters 246-250 then closed the stressed pronouns, the exclamative `que`
    // and the vocative.
    //
    // Chapters 251-255 then finished every set the book had taught only half of:
    // ahi/alli beside aqui, ahora/hoy beside manana, unos/unas beside un/una,
    // vuestro beside nuestro, and the ver/dar preterite. A1-Q-04 closed with
    // no new content at all -- `bastante` was taught at ch227 and its probe had
    // simply never been wired, which is its own kind of measurement error.
    //
    // The eight that remain are a DIFFERENT problem. Four of them -- A1-SN-03,
    // A1-Q-03, A1-N-01, A1-V-03 -- are things the book demonstrates on nearly
    // every page and never states, so they need lessons that make explicit what
    // the reader already does by reflex. The rest are unbuilt structures.
    //
    // This number is allowed to move only two ways. Up, when a lesson teaches
    // something the inventory lists. Down, when one is retired. It must NOT
    // move because a probe was loosened or a point deleted — if this assertion
    // fails alongside an edit to exam-inventory-es-a1.json, read that edit
    // before re-pinning.
    //
    // Chapters 257-261 then closed the last four of the "demonstrated but never
    // stated" points, which is why `partiallyTrue` is now empty: no null probe
    // remains that is PARTLY true. The four that are still null are absent
    // outright -- the ordinals, `uno...otro`, word-order flexibility, and the
    // infinitive as subject -- and need no note to say which half exists.
    expect(coverage.enumerated).toBe(85);
    expect(coverage.covered).toBe(85); // ...and 262-266 close the last four. A1 is COMPLETE: every point the inventory enumerates is taught. // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250 // +6 ch251-256 // +4 ch257-261: the four rules the book had always demonstrated and never stated // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250 // +6 ch251-255: the half-taught sets finished, plus bastante which was already taught and merely unwired // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250: the stressed pronouns, the exclamative and the vocative // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245: the vosotros preterite and the imperfect plural, both promised in chapter 204 // +3: ch221-225 demonstratives // +4: ch226-229 degree words // +4: ch230-235 joining words // +2: ch236-240 the gerund and the personal a // +3: chapters 221-225 teach the demonstratives // +4: chapters 226-229 teach muy, bastante and mal // +4: chapters 230-235 teach al/del, quien, o and ni
    expect(coverage.percent).toBe(100); // 53 -> 56 -> 60 -> 64 -> 66 -> 68 -> 71 -> 77 -> 81 -> 85/85 // 53/85 -> 56 -> 60 -> 64/85

    // Whole categories missing is a different failure from thin coverage, and
    // the report has to keep them distinguishable.
    expect(coverage.byCategory["Los demostrativos"]).toEqual({ enumerated: 3, covered: 3 }); // closed by chapters 221-225
    expect(coverage.byCategory["El sintagma adjetival"]).toEqual({ enumerated: 1, covered: 1 }); // closed by ch226-229: muy, poco and bastante are all taught now
    expect(coverage.byCategory["La oracion simple"]).toEqual({ enumerated: 6, covered: 6 });
  });

  it("reports the shortfall in a form somebody can act on", () => {
    const { lessons } = loadEverything();
    const report = formatExamCoverage(
      measureExamCoverage(loadExamInventory("spanish", "A1"), lessons),
    );
    expect(report).toContain("spanish A1: 85/85 points covered (100%)");
    // Worst category first, not alphabetical. This USED to be checkable against
    // the real corpus, whose emptiest category kept changing as the campaign
    // closed points — `El sintagma adjetival` at 0/1, then `Los cuantificadores`
    // at 1/4, then 2/4, then 3/4. A1 is now COMPLETE: every category is at
    // 100%, so the ordering falls back to the alphabetical tie-break, and
    // asserting the real report's first line would pin that tie-break while
    // claiming to pin the sort.
    //
    // The property therefore moves to data that can still falsify it. This is
    // not a weakening — the real report simply stopped being a test case for
    // ordering the moment there was nothing left to order.
    const uneven = measureExamCoverage(
      {
        ...FIXTURE,
        points: [
          // The names matter. "Poor"/"Rich" would order the same way
          // alphabetically as by shortfall, so the assertion could not tell the
          // sort from the tie-break — a security review proved that by deleting
          // the shortfall comparator and watching this test still pass. These
          // names make the two orderings CONTRADICT: alphabetically Alpha comes
          // first, by shortfall Zeta does.
          { id: "F-1", category: "Alpha", label: "covered", probe: ["ES-A"] },
          { id: "F-2", category: "Alpha", label: "covered", probe: ["ES-B"] },
          { id: "F-3", category: "Zeta", label: "uncovered", probe: null },
        ],
      },
      [lesson("ES-1", ["ES-A"]), lesson("ES-2", ["ES-B"])],
    );
    const unevenReport = formatExamCoverage(uneven).split("\n");
    expect(unevenReport[2]).toContain("0/1  Zeta");
    expect(unevenReport[3]).toContain("2/2  Alpha");
  });
});

describe("the committed French A1 inventory", () => {
  const inventory = loadExamInventory("french", "A1");

  it("keeps every point's probe key, because a MISSING probe scores as covered", () => {
    // `covered` is `point.probe !== null && …`. A point whose `probe` key is
    // absent reads as `undefined`, which is not `null`, so it scores COVERED
    // while demonstrating nothing. Authoring this file with a helper that
    // omitted null-valued keys reported 65/74 (88%) for a track with nine
    // grammar atoms, and only the implausibility of the number caught it.
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
    }
  });

  it("refuses an empty probe, which would score as covered", () => {
    for (const point of inventory.points) {
      expect(Array.isArray(point.probe) ? point.probe.length : 1).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The other direction of the same failure: `FR-LEX-CAFE-01` exists but
    // `FR-LEX-VERT-01` does not, because the suffix varies per lesson. A guessed
    // id resolves to "not introduced", which is fail-safe, silent, and wrong —
    // it reports taught material as a content gap.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "french");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("reports the gap as GRAMMAR-shaped, which is the finding", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(74);
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    expect(coverage.covered).toBe(20);
    // The shape, not the score: vocabulary is the best-covered column and the
    // sentence-level categories are empty. No quantity of headwords moves these.
    for (const empty of ["L'interrogation", "La phrase", "Le nom", "Les prepositions"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    expect(coverage.byCategory["Lexique de base"]!.covered).toBeGreaterThan(0);
  }, 60_000);
});
