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
import { listExamInventories, loadEverything, loadExamInventory } from "../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
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

const COMPLETE_SCOPE: ExamInventory["scope"] = {
  "communicative-functions": { status: "complete", source: "fixture", note: "fixture" },
  grammar: { status: "complete", source: "fixture", note: "fixture" },
  "phonology-orthography": { status: "complete", source: "fixture", note: "fixture" },
  lexicon: { status: "complete", source: "fixture", note: "fixture" },
};

const FIXTURE: ExamInventory = {
  version: 1,
  language: "spanish",
  level: "A1",
  about: "fixture",
  source: "fixture",
  scope: COMPLETE_SCOPE,
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

describe("inventory completeness", () => {
  it("requires every content dimension and keeps partial point coverage measurable", () => {
    expect(Object.keys(COMPLETE_SCOPE).sort()).toEqual([...EXAM_CONTENT_DIMENSIONS].sort());
    expect(isExamInventoryComplete(FIXTURE)).toBe(true);
    const partial = {
      ...FIXTURE,
      scope: {
        ...COMPLETE_SCOPE,
        lexicon: { ...COMPLETE_SCOPE.lexicon, status: "partial" as const },
      },
    };
    expect(isExamInventoryComplete(partial)).toBe(false);
    const coverage = measureExamCoverage(partial, [lesson("ES-1", ["ES-A", "ES-B"])]);
    expect(coverage).toMatchObject({ enumerated: 3, covered: 1, inventoryComplete: false });
    expect(formatExamCoverage(coverage)).toContain("(partial inventory)");
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
    // This list was EMPTY while the file enumerated grammar and nothing else.
    // Enumerating the PCIC functional inventory, the general and specific
    // notions, and the orthography inventory added 188 points, and 50 of them
    // have no corresponding atom anywhere in the corpus. That is the finding,
    // not a defect: an unmapped point is uncovered and reported by name, never
    // skipped. Every entry below carries a `note` naming the source exponent
    // that is missing, and the loop underneath proves the note is there.
    //
    // 50 -> 48. `A1-F2-16` and `A1-F2-17` — ask about and express ability —
    // leave this list because chapter 389 teaches `saber`, which is the exponent
    // the PCIC actually asks for. They were NOT closed by pointing them at
    // `poder`: both notes said in as many words that the source asks for *saber*
    // plus an infinitive and that substituting `poder` would be a different
    // structure, so closing them honestly meant authoring the verb the syllabus
    // names. HL23 §10 minted `SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO` for these two
    // points, which is why that rung is justified by this inventory rather than
    // invented to give two verbs somewhere to live.
    //
    // Read as a map of the real gaps: the F-* entries are speech acts the book
    // never performs (the affirmative imperative, toasting, congratulating);
    // the NE-* entries are whole A1 domains with no lesson
    // at all (clothing, cinema and music, the internet and dictating an
    // address, police and fire); and the O-* entries are nearly the entire
    // orthography inventory -- the alphabet, capitalisation, and every
    // punctuation mark except the question and exclamation pair.
    // 48 -> 44. THE WHOLE `Nociones evaluativas` GAP CLOSES AT ONCE, and three of
    // the four are the reason `SPINE-DESCRIBE-QUALITIES` exists at all.
    //
    // `A1-NG6-03` (attractiveness), `A1-NG6-08` (interest) and `A1-NG6-10` (ease and
    // difficulty) leave this list because chapters 397-399 author the exponents the
    // PCIC actually names — guapo, feo and bonito; interesante; fácil and difícil.
    // HL23 §12.2 justified the qualities rung BY these three points, and a
    // justification that leaves the points unmapped is a justification used as an
    // excuse. They close the same way `A1-F2-16`/`A1-F2-17` did: by authoring the
    // source's own exponent, never by pointing the point at a word already present.
    //
    // `A1-NG6-09` (capacity and competence with saber) is DIFFERENT, and it is a bug
    // this slice found rather than work it did. Its note read "the corpus never
    // introduces saber as a verb, only the fixed phrase no se" — which stopped being
    // true when chapter 389 authored `saber` for the two points named above. The atom
    // it needs, `ES-LEX-SABER`, has existed since #13154 and nothing pointed at it.
    // A null whose stated reason has expired is worse than a bare null, because the
    // note is precisely what the loop below trusts to prove the null was considered.
    expect(unmapped.sort()).toEqual([
      "A1-F2-10", "A1-F3-03", "A1-F4-01", "A1-F5-09",
      "A1-F5-10", "A1-F6-06", "A1-NE02-01", "A1-NE06-01", "A1-NE06-05",
      "A1-NE07-04", "A1-NE07-06", "A1-NE08-02", "A1-NE09-06", "A1-NE11-04",
      "A1-NE12-02", "A1-NE13-03", "A1-NE15-02", "A1-NE15-03", "A1-NE15-04",
      "A1-NE16-02", "A1-NE17-02", "A1-NE18-01", "A1-NE18-02", "A1-NE18-05",
      "A1-NE18-06", "A1-NE20-05",
      "A1-O1-01", "A1-O1-02", "A1-O1-03", "A1-O1-04", "A1-O1-05",
      "A1-O1-06", "A1-O1-07", "A1-O3-01", "A1-O3-02", "A1-O3-03", "A1-O3-05",
      "A1-O3-06", "A1-O3-07", "A1-O3-08", "A1-O3-09", "A1-O4-01", "A1-O4-02",
      "A1-O4-03",
    ]);

    // Every null must SAY why it is null. The note is what stops a null from
    // reading as "nobody has looked yet": it names the exponent the source asks
    // for and states that the corpus does not introduce it. Without this
    // assertion the list above could grow by a bare null with no reasoning.
    for (const id of unmapped) {
      const point = inventory.points.find((candidate) => candidate.id === id)!;
      expect(point.probe, `${id} must stay null or gain a probe deliberately`).toBeNull();
      expect(point.note?.trim(), `${id} is unmapped and must say why`).toBeTruthy();
    }
  });
});

describe("the committed German A2 source tranche", () => {
  it("stays explicitly partial while turning official source evidence into named gaps", () => {
    const inventory = loadExamInventory("german", "A2");
    expect(isExamInventoryComplete(inventory)).toBe(false);
    expect(Object.values(inventory.scope).every((entry) => entry.status === "partial")).toBe(true);

    const { lessons } = loadEverything();
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
      scope: COMPLETE_SCOPE,
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
      write(root, { ...FIXTURE, points: [{ id: "X", category: "__proto__", label: "l", probe: null }] });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/reserved category name/);
    });
  });

  it("refuses a duplicate point id, which would double-count a category", () => {
    withTempInventory((root) => {
      write(root, {
        ...FIXTURE,
        points: [
          { id: "X", category: "Uno", label: "l", probe: null },
          { id: "X", category: "Uno", label: "l", probe: null },
        ],
      });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/duplicate point id/);
    });
  });

  it("requires an exact, sourced boundary for every content dimension", () => {
    withTempInventory((root) => {
      const { lexicon: _lexicon, ...missingLexicon } = COMPLETE_SCOPE;
      write(root, { ...FIXTURE, scope: missingLexicon });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(
        /scope must contain exactly.*missing \[lexicon\]/,
      );

      write(root, {
        ...FIXTURE,
        scope: {
          ...COMPLETE_SCOPE,
          grammar: { ...COMPLETE_SCOPE.grammar, source: "" },
        },
      });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(
        /scope\.grammar\.source must name its provenance/,
      );
    });
  });

  it("loads and lists a partial inventory without claiming it is complete", () => {
    withTempInventory((root) => {
      write(root, {
        ...FIXTURE,
        scope: {
          ...COMPLETE_SCOPE,
          lexicon: { ...COMPLETE_SCOPE.lexicon, status: "partial" },
        },
      });
      expect(isExamInventoryComplete(loadExamInventory("spanish", "A1", root))).toBe(false);
      expect(listExamInventories(root)).toEqual([{ language: "spanish", level: "A1", complete: false }]);
    });
  });
});

describe("what the corpus actually covers", () => {
  it("pins A1 coverage, which is the number this project is judged on", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(loadExamInventory("spanish", "A1"), lessons);
    expect(coverage.inventoryComplete).toBe(false);

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
    // WHY THIS NUMBER JUST FELL FROM 100% TO 82%, AND WHY THAT IS THE FIX
    // Everything above is the history of the GRAMMAR dimension, which reached
    // 85/85. The comment ten lines up says this number must not move because a
    // point was added or a probe loosened, and warns the next reader to read
    // the inventory edit before re-pinning. This is that edit, so here is the
    // reasoning it asks for.
    //
    // The file used to enumerate ONE of the four HL20 content dimensions. It
    // now also enumerates the PCIC functional inventory (54 points), the
    // general and specific notions (36 + 77), and the orthography inventory
    // (21) -- 188 new points, each restated from the A1 column the source
    // publishes separately from A2. 138 of them map to atoms the corpus really
    // introduces; 50 have no corresponding atom and are null.
    //
    //     before: 85/85   = 100%, 0 unmapped
    //     after: 223/273  =  82%, 50 unmapped
    //
    // The denominator grew because the target got honest, not because the book
    // got worse -- no lesson was retired and no probe was loosened, and the
    // grammar dimension is still 85/85 inside the new total. A 100% that
    // measured a quarter of the construct was the flattering failure HL20 was
    // written to close, and 82% of a four-dimension target is a larger true
    // number than 100% of a one-dimension one.
    //
    // This is also why `percent` is pinned exactly rather than as a floor: it
    // is allowed to fall, but only for a reason stated here in prose.
    //
    // 223 -> 225, 50 -> 48 unmapped. HL23 §10 authors `saber` (chapter 389) and
    // maps `A1-F2-16` and `A1-F2-17` onto it. `percent` is unmoved at 82: two
    // points out of 273 is 0.7pp, which rounds away. That is worth saying out
    // loud — a headline percentage that does not move is not evidence that
    // nothing happened, which is exactly why `covered` and `unmapped` are pinned
    // beside it rather than the percentage alone.
    //
    // 225 -> 229, 48 -> 44 unmapped. HL23 §12.2's qualities rung closes the whole
    // `Nociones evaluativas` gap: chapters 397-399 author the source's own exponents
    // for `A1-NG6-03` (guapo, bonito — feo the corpus already had), `A1-NG6-08`
    // (interesante) and `A1-NG6-10` (fácil, difícil). `A1-NG6-09` is the odd one and
    // cost no authoring at all: its note claimed the corpus never introduces `saber`,
    // which stopped being true when chapter 389 authored it for `A1-F2-16`/`A1-F2-17`
    // two slices ago. The atom existed and nothing pointed at it.
    //
    // `percent` MOVES this time, 82 -> 84. Contrast the 223 -> 225 note above, where
    // two points rounded away: four points out of 273 is 1.5pp and survives rounding.
    // Both behaviours are correct and neither is evidence on its own, which is the
    // argument for pinning `covered` and `unmapped` beside it.
    expect(coverage.enumerated).toBe(273); // 85 grammar + 54 functions + 113 notions + 21 orthography
    expect(coverage.covered).toBe(229); // 85 grammar (unchanged) + 144 newly mapped // ...and 262-266 close the last four enumerated points. The inventory scope remains partial. // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250 // +6 ch251-256 // +4 ch257-261: the four rules the book had always demonstrated and never stated // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250 // +6 ch251-255: the half-taught sets finished, plus bastante which was already taught and merely unwired // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245 // +3 ch246-250: the stressed pronouns, the exclamative and the vocative // +3 ch221-225 // +4 ch226-229 // +4 ch230-235 // +2 ch236-240 // +2 ch241-245: the vosotros preterite and the imperfect plural, both promised in chapter 204 // +3: ch221-225 demonstratives // +4: ch226-229 degree words // +4: ch230-235 joining words // +2: ch236-240 the gerund and the personal a // +3: chapters 221-225 teach the demonstratives // +4: chapters 226-229 teach muy, bastante and mal // +4: chapters 230-235 teach al/del, quien, o and ni
    expect(coverage.percent).toBe(84); // 53 -> 56 -> 60 -> 64 -> 66 -> 68 -> 71 -> 77 -> 81 -> 85/85 grammar-only, then 223/273 across four dimensions
    expect(coverage.unmapped).toBe(44); // was 0 while only grammar was enumerated

    // Whole categories missing is a different failure from thin coverage, and
    // the report has to keep them distinguishable. These three are GRAMMAR
    // categories and are deliberately unchanged: the new points all landed in
    // new categories, so if one of these ever moves, a grammar point moved.
    expect(coverage.byCategory["Los demostrativos"]).toEqual({ enumerated: 3, covered: 3 }); // closed by chapters 221-225
    expect(coverage.byCategory["El sintagma adjetival"]).toEqual({ enumerated: 1, covered: 1 }); // closed by ch226-229: muy, poco and bastante are all taught now
    expect(coverage.byCategory["La oracion simple"]).toEqual({ enumerated: 6, covered: 6 });

    // The two categories that are now entirely absent from the book. Naming
    // them is the point of the per-category tally: "82%" is a mood, "the
    // orthography inventory is 2/21 and clothing is 0/3" is a work queue.
    expect(coverage.byCategory["Ortografia de letras y palabras"]).toEqual({ enumerated: 7, covered: 0 });
    expect(coverage.byCategory["Puntuacion"]).toEqual({ enumerated: 9, covered: 1 });
  });

  it("reports the shortfall in a form somebody can act on", () => {
    const { lessons } = loadEverything();
    const report = formatExamCoverage(
      measureExamCoverage(loadExamInventory("spanish", "A1"), lessons),
    );
    expect(report).toContain("spanish A1 (partial inventory): 229/273 points covered (84%)");
    expect(report).toContain("44 with no corresponding atom");
    // Worst category first, not alphabetical. This USED to be checkable against
    // the real corpus, whose emptiest category kept changing as the campaign
    // closed points — `El sintagma adjetival` at 0/1, then `Los cuantificadores`
    // at 1/4, then 2/4, then 3/4. Every ENUMERATED category is now at 100%, so
    // the ordering falls back to the alphabetical tie-break, and
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
    //
    // 20 -> 25: HL-C229 authored chapter 32 and took L'interrogation from 0/5 to
    // 5/5 -- the first time this loop closed end to end, with the plan naming the
    // gap, the inventory naming the five points, nine lessons teaching them, and
    // the probes then resolving against real atoms. The number moved because the
    // CORPUS changed, not because the target was edited.
    //
    // 25 -> 26: retiring hand-written chapter 6 closed A1-PRON-03, obligatory
    // liaison. The hand-written chapter mentioned the six/dix -s and the neuf
    // f-to-v in passing inside two `sounds` blocks; the generated chapter teaches
    // liaison as a named rule with its own atom, which is what a probe can
    // resolve against. Same rule here: the corpus changed, not the target.
    //
    // 26 -> 27: retiring hand-written chapter 8 closed A1-LEX-07, telling the
    // time. The hand-written chapter stopped at whole hours and named et quart,
    // et demie and moins le quart in one sentence while deferring them, so the
    // corpus could not have satisfied the point however the probe was written.
    // The chapter now teaches all three, and the probe lists all seven atoms
    // rather than a sample: a candidate asked for half past does not get partial
    // credit for o'clock.
    //
    // 27 -> 30: retiring hand-written chapter 1, the first chapter in the book,
    // closed three at once -- and all three were unmapped for the same reason,
    // which is the finding. A1-LEX-01 is "greetings and farewells" in a track
    // whose opening chapter is called Greetings: the farewells had atoms because
    // chapter 4 was generated, the greetings did not because chapter 1 was not,
    // so half the point existed and the whole point read as absent. A1-D-01 (the
    // definite article) and A1-A-01 (adjective agreement) are the two grammar
    // rules those greetings run on -- bon versus bonne is agreement, and le/la is
    // where the gender it agrees with becomes visible. Both were taught on page
    // one from the beginning and neither could be probed, because a hand-written
    // chapter's grammarlens owns no atom.
    //
    // 30 -> 31: retiring hand-written chapter 2 closed A1-V-12, reflexive verbs
    // with `se` in the present. Chapter 27 has conjugated s'asseoir and se lever
    // through the whole present since it was written, and chapter 2 has been
    // teaching je m'appelle -- but the CONSTRUCTION was owned by nobody: ch27's
    // atoms type the two verbs and their stems, and ch2 was hand-written, so a
    // corpus that fully teaches the point had no atom that named it. The probe
    // lists the rule and the two conjugated verbs, because the rule alone is not
    // the present tense and the verbs alone were typed as lexis.
    //
    // A1-P-04 was already covered and its probe is corrected in the same pass:
    // it read FR-GRAMMAR-PLEASE-REGISTER-04, chapter 19's s'il vous plait, which
    // DEMONSTRATES the tu/vous register without naming it. Chapter 2 owns the
    // point directly and both its atoms are added.
    //
    // 31 -> 32: A1-V-11, vouloir / pouvoir / devoir in the singular. Nothing was
    // authored for it. It was found while writing the A2 inventory -- chapter 33
    // has given each of the three its own lesson with je / tu / il printed since
    // it was written, and types the chain rule besides. The point was reading as
    // a content gap and would have sent an author to write what already exists,
    // which is the failure mode an inventory is supposed to PREVENT.
        // 27 -> 28: the chapter-9 split closed A1-LEX-06, days/months/seasons. This
    // one was deliberately held back through two earlier tranches: the days were
    // taught, but the track owned two headwords -- `les mois` and `les saisons`
    // -- for twelve months and four seasons, so any probe naming a month would
    // have been a claim the corpus could not support. Splitting chapter 9 into
    // three chapters taught all sixteen, and the probe resolves honestly.
    // Both closures were authored on separate branches from the same base of 27
    // and met in this merge, so the figure below is RE-MEASURED against the merged
    // tree rather than obtained by adding three and one to twenty-seven.
    // The two branches above met in this merge, each written from its own base,
    // so the figure below is RE-MEASURED against the merged tree by running the
    // suite, never obtained by adding the two branches' deltas.
    //
    // 31 -> 33: retiring chapters 17 and 19 gave `avoir` and `etre` a typed atom
    // per person instead of one lesson holding each whole paradigm, which is
    // what the two remaining verb points were waiting for.
    expect(coverage.covered).toBe(33);
    expect(coverage.byCategory["L'interrogation"]).toEqual({ enumerated: 5, covered: 5 });
    // The shape, not the score: vocabulary is still a strong column and the
    // sentence-level categories are still empty. No quantity of headwords moves
    // these -- only grammar chapters like the one that just closed the fourth.
    for (const empty of ["La phrase", "Le nom", "Les prepositions"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    expect(coverage.byCategory["Lexique de base"]!.covered).toBeGreaterThan(0);
  }, 60_000);
});

describe("the committed French A2 inventory", () => {
  const inventory = loadExamInventory("french", "A2");

  it("keeps every point's probe key, because a MISSING probe scores as covered", () => {
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
    // This caught a real one on the first run. A2-F-11 probed
    // FR-IDIOM-CA-MARCHE-AGREEMENT-01, which is a real, committed, correctly
    // spelled unit -- declared in `introduces_idioms`. `measureExamCoverage`
    // resolves against `introducedAtoms`, which reads `introduces.knowledge` and
    // the block directives and NOTHING else, so an idiom or a culture claim in a
    // probe is indistinguishable from a typo: the point silently reports
    // uncovered. The three namespaces are separate and only one of them is
    // probeable.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "french");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("mirrors the A1 file's categories, so the two read as one ladder", () => {
    // The A2 file is comparable to the A1 file BY CONSTRUCTION, not by accident:
    // same category names in the same order, plus `Actes de parole` at the front
    // (A2 is where the exam starts testing what you can DO with a paragraph) and
    // `Lexique` renamed from `Lexique de base` because it is no longer basic.
    // A future reader must be able to see a point move from one column to the
    // other; that only works if the columns are the same shape.
    const a1 = new Set(loadExamInventory("french", "A1").points.map((p) => p.category));
    const a2 = new Set(inventory.points.map((p) => p.category));
    const shared = [...a1].filter((c) => a2.has(c));
    expect(shared.sort()).toEqual([
      "L'adjectif", "L'adverbe", "L'interrogation", "La negation", "La phrase",
      "Le nom", "Le verbe", "Les determinants", "Les prepositions", "Les pronoms",
      "Prononciation et orthographe",
    ]);
    expect([...a2].filter((c) => !a1.has(c)).sort()).toEqual(["Actes de parole", "Lexique"]);
  });

  it("reports the gap as FUNCTION- and PAST-TENSE-shaped, which is the finding", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(104);
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    expect(coverage.covered).toBe(16);
    // The shape, and it is a sharper finding than the number. FIFTEEN of the
    // sixteen `Actes de parole` are uncovered, because A2 is the level at which
    // the exam stops asking for words and starts asking for a paragraph that
    // does something -- and this corpus is a vocabulary corpus with a grammar
    // spine. The one that is covered, grading an opinion, is covered by half:
    // the corpus can say `aimer` against `aimer bien` and cannot yet disagree.
    // The same story runs through the past: A2's construct is dominated by the
    // passe compose and the imparfait, and the two French chapters that carry
    // them are still HAND-WRITTEN, so neither owns an atom.
    expect(coverage.byCategory["Actes de parole"]).toEqual({ enumerated: 16, covered: 1 });
    for (const empty of ["Le nom", "Les determinants", "L'adjectif", "Les prepositions",
                         "L'adverbe", "La phrase", "La negation"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    // Where it IS strong is exactly where the retirement work has already been:
    // everyday verbs, the question system, and register.
    expect(coverage.byCategory["Lexique"]!.covered).toBe(7);
  }, 60_000);
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
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "german");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("reports the same grammar-shaped gap French does", () => {
    // German holds 123 atoms across 106 lessons and SIX of them are grammar. The
    // categories that stay empty are the ones a candidate is examined on: the
    // article system, questions, prepositions. Vocabulary is again the strongest
    // column. Two independent tracks, one shape — see HL-C226.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(70);
    expect(coverage.covered).toBe(21);
    for (const empty of ["Der Artikel", "Die Frage", "Die Praeposition"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    expect(coverage.byCategory["Grundwortschatz"]!.covered).toBeGreaterThan(0);
  }, 60_000);
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
describe("the committed Marathi A1 inventory", () => {
  const inventory = loadExamInventory("marathi", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key", () => {
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
    // The same rule the French and German blocks pin, and it matters MORE here.
    // Marathi covers 29% of its own target, so most points are uncovered; if a
    // guessed id were allowed to sit in a probe it would be indistinguishable
    // from the 213 honest gaps around it, and it would never flip to covered even
    // after the lesson was written, because the suffix would not match. `null`
    // plus a note is the only honest way to say "nothing here yet".
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "marathi");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("never lets an unmapped point read as 'nobody has looked yet'", () => {
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim(), `${point.id} is unmapped and must say why`).toBeTruthy();
    }
  }, 60_000);

  it("derives from the Spanish set TOTALLY, so nothing is dropped by accident", () => {
    // The property that makes a proxy auditable rather than a gesture. Every one
    // of Spanish's 273 points must be either (a) named by some Marathi point's
    // `derivedFrom`, or (b) listed in `proxy.notTransferred` with a reason. A
    // point that is silently absent is indistinguishable from one nobody thought
    // of — which is the failure mode the whole file exists to prevent — and
    // writing this assertion is what caught `A1-O1-06` going missing.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set<string>();
    for (const point of inventory.points) {
      for (const id of (point as unknown as { derivedFrom: string[] }).derivedFrom) derived.add(id);
    }
    const known = new Set(spanish.points.map((point) => point.id));
    // No dangling references in the other direction either: a `derivedFrom`
    // naming a Spanish point that does not exist is a typo that would quietly
    // shrink the audit.
    for (const id of derived) expect(known.has(id), `derivedFrom cites unknown Spanish point ${id}`).toBe(true);
    const unaccounted = [...known].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted).toEqual([]);
    // Dropping and deriving the same point would let a reader believe both.
    expect([...derived].filter((id) => dropped.has(id))).toEqual([]);
  }, 60_000);

  it("marks a point with no Spanish source as Marathi-specific, and means it", () => {
    // Devanagari orthography, the postpositions, the ergative and the
    // gender-marked present have no Spanish counterpart. Those points are honest
    // additions; what they must never be is padding that hides behind an empty
    // field. `marathiSpecific` has to agree with `derivedFrom` in both
    // directions, and such a point must still name a non-editorial anchor or an
    // explicit note.
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; marathiSpecific?: boolean };
      expect(cast.marathiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // A proxy is a scaffold, not a template: some points must be Marathi's own.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    // Two bodies could lend this file weight it has not earned: a real Marathi
    // examiner, and the awarding body behind the proxy. The file must disclaim
    // both, because "derived from the DELE A1 inventory" is one careless edit
    // away from reading as "DELE says this about Marathi".
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(
      /NOTHING IN THIS FILE MAY BE ATTRIBUTED TO THE MAHARASHTRA DIRECTORATE OF LANGUAGES/,
    );
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/THE SEARCH IS SETTLED, DO NOT REPEAT IT/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/no external Marathi A1 syllabus/i);
    // Marathi has no timed mocks, so the file must say what it used instead of
    // the artifact Hindi mined. An unstated substitution is an unauditable one.
    expect(inventory.source).toMatch(/NOTE ON METHOD/);
    expect(inventory.source).toMatch(/Marathi has no mocks/);
    // Partial in every dimension. Neither an editorial basis nor a sourced proxy
    // is a shortcut to `complete`; a proxy does not close a dimension.
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    // `anchorIds` answers "which of these did you read, and which did you
    // decide?". Without the KIND, a project-owned file, a sourced proxy and the
    // Council of Europe read the same on the page — and the weakest points, the
    // purely editorial ones, are the ones a reader most needs flagged.
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
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

  it("separates a content gap from a gap in the MEASUREMENT", () => {
    // The finding that nearly got written up wrong, twice, in two shapes. A probe
    // reads DECLARED atoms:
    //
    //   SCHEMA-V1 — 26 of Marathi's 205 lessons, the whole of chapters 9 to 12,
    //   declare no atoms while teaching mii, tuu/tumhii, maazhaM, kaay, kasaa,
    //   "what's your name?", "how are you?", kaam karne and raahne.
    //
    //   EMPTY-INTRODUCES — worse, because it hides inside schema-v2 where
    //   everything LOOKS declared. Four v2 lessons carry `introduces: []` while
    //   teaching new material: MR-R22-request-verbs and MR-R23-wellbeing-verbs
    //   drill five polite imperatives and a future while declaring only the
    //   infinitive atoms they review, and MR-A1M17/18 teach the guided and
    //   independent 30-to-40-word message — the A1 writing paper's second task —
    //   while declaring nothing at all.
    //
    // The first draft recorded several of both kinds as "untaught", which would
    // have sent an author to rewrite chapter 9 and chapter 24. No corpus-internal
    // metric can see either class; only a target list asks the question that
    // exposes them. This pins the marker so a future edit cannot quietly collapse
    // the distinction back into an undifferentiated "not covered".
    const marked = inventory.points.filter((point) => (point.note ?? "").includes("MEASUREMENT GAP"));
    expect(marked.length).toBeGreaterThanOrEqual(13);
    for (const point of marked) expect(point.probe, point.id).toBeNull();
    expect(inventory.probeSemantics).toMatch(/SCHEMA-V1 MEASUREMENT GAP/);
    expect(inventory.probeSemantics).toMatch(/EMPTY-INTRODUCES MEASUREMENT GAP/);
  });

  it("reports the gap as domain-shaped, which is the finding", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    //
    // The number moved 55/131 -> 88/301 when the point set was rebuilt from the
    // Spanish proxy rather than from CEFR descriptors, and BOTH halves of that
    // are the result. The numerator rose because the Spanish walk found taught
    // material an editorial list had never asked about — evaluative notions,
    // mental notions, the colon in a form label. The denominator nearly trebled
    // because it found twenty thematic domains nobody had enumerated: education,
    // work, leisure, media, housing, services, shopping, health, travel, money,
    // government, the arts, religion, the natural world. The corpus covers almost
    // none of them, and that is what an external boundary is FOR.
    //
    // The shape also differs from French and German, which are grammar-shaped
    // with vocabulary as their strongest column. Marathi's strongest columns are
    // its SCRIPT (15/24, after the previous tranche took closure 44 -> 0) and its
    // mental- and evaluative-notion verbs; the columns that carry an exam paper
    // are empty.
    //
    // 88 -> 111: the joining tranche (chapters 30-36). Coordination went 0/5 to
    // 5/5 and Subordination 1/7 to 5/7, and because a conjunction is a tool
    // rather than a topic, twelve further points fell in five other columns --
    // the negated sentence closed four function points, the polar particle three
    // more, and the two punctuation marks their own. Seventeen new words and
    // endings; twenty-three points. Coordination therefore leaves the
    // empty-category list below, which is the movement, and Demonstratives,
    // Temporal notions, Housing and Shopping stay in it, which is the remaining
    // work.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(301);
    expect(coverage.covered).toBe(111);
    expect(coverage.unmapped).toBe(190);
    // Zero partials is a property of the "existing atoms only" rule above, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    for (const empty of ["Demonstratives", "Temporal notions", "Housing", "Shopping"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    expect(coverage.byCategory["Coordination"]!.covered).toBe(5);
    expect(coverage.byCategory["Devanagari letters and signs"]!.covered).toBeGreaterThan(0);
    expect(coverage.byCategory["Sound system"]!.covered).toBeGreaterThan(0);
    expect(formatExamCoverage(coverage)).toContain(
      "marathi A1 (partial inventory): 111/301 points covered (37%)",
    );
  }, 60_000);
});

// ---------------------------------------------------------------------------
// Tamil (HL-C290 again, and the first Dravidian track to get one).
//
// The method is Marathi's and these tests are deliberately its tests, because
// the value of a method is that the second use is cheaper than the first. Two
// things differ and both are properties of the LANGUAGE rather than of the
// derivation:
//
//   1. Tamil is DIGLOSSIC, and `core/exam-levels.json` says so in the caveat it
//      carries for this track and for no other Dravidian one. The written and
//      spoken registers diverge sharply and this curriculum teaches spoken
//      Tamil first, which is a fact about what an exam could even ask. So the
//      inventory has a register column that no proxy-derived file has had, and
//      its first point — that the corpus never tells the learner any of this —
//      is uncovered.
//   2. Tamil's SCRIPT column is nearly closed rather than nearly empty. All 18
//      core consonants and 10 of 12 independent vowels are taught, and walking
//      every Tamil character the track prints against the set its script
//      lessons teach returns zero shown-but-untaught. That is the opposite of
//      the Marathi result and it is why the shape assertions below name
//      different empty columns.
// ---------------------------------------------------------------------------
describe("the committed Tamil A1 inventory", () => {
  const inventory = loadExamInventory("tamil", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name. A probe pointing at an id somebody
    // expects a future lesson to introduce resolves to "not introduced" forever
    // and sits in the report indistinguishable from the honest gaps around it.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "tamil");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point either derives into some Tamil point or is dropped
    // with a reason, and no point may be both. A source point that is silently
    // absent is indistinguishable from one nobody thought of, which is the
    // failure the whole exercise exists to prevent.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; tamilSpecific?: boolean };
      expect(cast.tamilSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // A proxy is a scaffold, not a template. Tamil's own points include the
    // rational/irrational split that governs all its agreement, the stacking
    // case suffix, the strong/weak verb sort, the two-way negative, the three
    // n letters, and the whole register column.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    // Three bodies could lend this file weight it has not earned: the two the
    // proficiency backbone names as the nearest thing to a Tamil ladder, and
    // the awarding body behind the proxy.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Singapore Ministry of Education/);
    expect(inventory.about).toMatch(/Tamil Nadu state syllabi/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    // The search is settled per HL-C287/HL-C290 and was deliberately not redone.
    // Saying so is what keeps "we did not look" from reading as "there is
    // nothing to find".
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Tamil has neither mocks nor task shapes, so the file must say what it
    // used instead of the artifacts Hindi and Marathi mined. An unstated
    // substitution is an unauditable one.
    expect(inventory.source).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no tamil\/task-shapes\/ and no tamil\/mocks\//);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
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

  it("reports a gap that is GRAMMAR-shaped, with the script column nearly closed", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    //
    // 155 -> 174 (HL-C304, chapters 74-81). The clause-joining tranche closed
    // the whole Iṇaittoḍar column plus the polar -ā, eṉ, eppōdu, the additive
    // -um, negative coordination, four communicative functions, both numeral
    // points — by declaring the atoms chapter 7 was already teaching — and the
    // diglossia point the file named as the most Tamil-specific one in it.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(262);
    expect(coverage.covered).toBe(174);
    expect(coverage.unmapped).toBe(88);
    // Zero partials is a property of the "existing atoms only" rule, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    // WAS THE FINDING, AND IS NOW THE PAYMENT. Tamil could not join two clauses
    // at all — no `-um ... -um`, no `aanaal`, no `alladu`, no quotative `enru`.
    // Chapters 74-81 teach all seven, which is what lets the well-taught verb
    // and lexis columns become sentences.
    expect(coverage.byCategory["Iṇaittoḍar (joining clauses)"]).toEqual({ enumerated: 7, covered: 7 });
    // The two columns that carry this track, and they are not the ones French
    // and German lead on.
    expect(coverage.byCategory["Vinaiccol (the verb)"]!.covered).toBeGreaterThan(15);
    expect(coverage.byCategory["Tamiḻ eḻuttu (script and orthography)"]!.covered).toBeGreaterThan(5);
    expect(formatExamCoverage(coverage)).toContain(
      "tamil A1 (partial inventory): 174/262 points covered (66%)",
    );
  }, 60_000);
});

describe("the committed Kannada A1 inventory", () => {
  const inventory = loadExamInventory("kannada", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name. A probe pointing at an id somebody
    // expects a future lesson to introduce resolves to "not introduced" forever
    // and sits in the report indistinguishable from the honest gaps around it.
    // This file was written while Kannada chapters 1, 2 and 4 were still
    // hand-written, so 37 of the atoms it probes did not exist when its first
    // draft was validated; the atom set was re-derived from the merged tree
    // before commit rather than trusted from the working branch.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "kannada");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point either derives into some Kannada point or is dropped
    // with a reason, and no point may be both. Fifteen were both in the first
    // draft, because restating a point around Kannada machinery FEELS like not
    // transferring it. Restating is deriving; `notTransferred` is now only the
    // two points that produce no Kannada point at all.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; kannadaSpecific?: boolean };
      expect(cast.kannadaSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    // Deliberately few, and that is a claim rather than an omission: nearly
    // every Kannada column answers a demand some Spanish point also makes, even
    // where the machinery is completely different -- a case suffix doing what a
    // preposition does, a dative subject doing what gustar does. Only four
    // points have no Spanish question behind them at all.
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id).sort()).toEqual(
      ["KA-A1-CASE-02", "KA-A1-N-06", "KA-A1-P-05", "KA-A1-REG-04"],
    );
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Kannada Sahitya Parishat/);
    expect(inventory.about).toMatch(/Karnataka's state school syllabi/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Kannada has neither mocks nor task shapes nor an assessment contract, so
    // the file must say what it used instead. An unstated substitution is an
    // unauditable one.
    expect(inventory.about).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no kannada\/task-shapes\/ and no kannada\/mocks\//);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
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

  it("reports a CLOSED joining column and a script still 19 characters short", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(258);
    expect(coverage.covered).toBe(193);
    expect(coverage.unmapped).toBe(65);
    expect(coverage.partial).toBe(0);
    // WHAT THIS ASSERTION USED TO SAY, and why it changed. The inventory landed
    // reporting 167/258 and an EMPTY joining column: `mattu`, `athava`,
    // `aadare`, `eekendare` and the quotative `anta`/`endu` occurred ZERO times
    // in 268 lessons and zero times in the generated book, and the only two
    // covered points were the -i participle and the turn-level connectives of
    // chapter 64 -- a chapter literally named JOIN that joins turns and not
    // clauses. Chapters 67 to 73 answer that finding directly: 27 headwords
    // chosen off this file's OWN uncovered list closed 26 points, of which nine
    // are this column. The denominator did not move and `partial` stayed at 0,
    // so nothing was reworded to make the number rise.
    const joining = coverage.byCategory["Samuccaya (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 11 });
    // DO NOT CARRY THE TAMIL SHAPE HERE. Tamil's script column came back 52 of
    // 52 characters taught. Kannada's is the opposite case and was measured
    // twice: 42 characters taught against 69 used in headwords when this
    // inventory was written, and 50 of 69 after chapters 67-73 taught the eight
    // most-used untaught characters. `ma` alone appears in 36 headwords and was
    // never taught. Nineteen characters remain, six of which have a sourced
    // ductus this project has not spent and thirteen of which have none.
    expect(coverage.byCategory["Lipi (script and orthography)"]!.covered).toBeLessThan(10);
    // The two columns that carry this track, and they are not the ones French
    // and German lead on.
    expect(coverage.byCategory["Kriyaapada (the verb)"]!.covered).toBeGreaterThan(12);
    expect(coverage.byCategory["Padakosha (lexicon by domain)"]!.covered).toBeGreaterThan(45);
    expect(formatExamCoverage(coverage)).toContain(
      "kannada A1 (partial inventory): 193/258 points covered (75%)",
    );
  }, 60_000);
});

describe("the committed Malayalam A1 inventory", () => {
  const inventory = loadExamInventory("malayalam", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name, and the reason this file was
    // generated from a table rather than hand-written: a probe pointing at an
    // id somebody expects a future lesson to introduce resolves to "not
    // introduced" forever and sits in the report indistinguishable from the
    // honest gaps around it. Every probe here was checked against the merged
    // tree's atom set before the file was emitted, and the first draft was
    // refused by that check for inventing four `ML-SCRIPT-DIGIT-*` ids where
    // the corpus actually has `ML-SCRIPT-DIGITS-1-3-07` and its three siblings.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "malayalam");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point derives into some Malayalam point, and none is
    // dropped. `notTransferred` is EMPTY on purpose: the points whose honest
    // Malayalam answer is "there is no such thing here" -- article
    // contractions, capital letters, written accents -- are enumerated as
    // points recording the absence rather than dropped from the walk, because
    // HL-C290 settled that restating a question around the target language's
    // machinery IS deriving it.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; malayalamSpecific?: boolean };
      expect(cast.malayalamSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    // Deliberately none. Every Malayalam column here answers a demand some
    // Spanish point also makes, even where the machinery is unrecognisable --
    // a dative subject doing what gustar does, a question particle doing what
    // inversion does, a verb that never agrees answering fourteen paradigm
    // points at once. If a later tranche adds a point with no Spanish question
    // behind it, this assertion is where it has to be declared.
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([]);
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Kerala Sahitya Akademi/);
    expect(inventory.about).toMatch(/Kerala's state school syllabi/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.about).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no malayalam\/task-shapes\/ and no malayalam\/mocks\//);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("says the register column was MEASURED here and not carried over from Tamil", () => {
    // The instruction this file was written under: where a track's
    // exam-levels.json entry carries no caveat, measure rather than importing
    // another track's shape. Tamil has a diglossia caveat and a register
    // column; Malayalam has no caveat, and the column it does get is a
    // DIFFERENT axis found by counting this corpus's own register fields.
    expect(inventory.about).toMatch(/THE REGISTER COLUMN IS MEASURED, NOT BORROWED/);
    expect(inventory.about).toMatch(/does not claim a literary\/spoken\s*\n?\s*diglossia/);
    const register = inventory.points.filter((point) =>
      point.category.startsWith("Bhaashaabhedam"),
    );
    expect(register).toHaveLength(5);
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
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

  it("reports a joining column of 2 out of 11, and a script 9 characters short", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(243);
    expect(coverage.covered).toBe(162);
    expect(coverage.unmapped).toBe(81);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE. Malayalam joins clauses with a clitic -um for "and", a
    // quotative ennu for "that", and participles for everything else, and the
    // corpus teaches none of them. Chapter 64 is called "Five Words That Join"
    // and four of its five words are adverbs -- pinne, udane, chilappol,
    // maathram -- so only ennaal ("but") is a real connective. The second
    // covered point is the -i participle inside the goodbye poyi varaam, which
    // the corpus teaches without ever naming it as a way of joining clauses.
    // Same shape as Kannada's chapter 64 finding, measured independently.
    const joining = coverage.byCategory["Samuchayam (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 2 });
    // DO NOT CARRY ANOTHER TRACK'S SCRIPT SHAPE HERE. Tamil came back 52 of 52,
    // Kannada 50 of 69. Malayalam was measured on its own and is 58 of the 67
    // distinct characters its headwords use -- 87 per cent. The nine open ones,
    // by the number of headwords needing them: sha (14), lla (10), chillu-rr
    // (8), nga (8), the ai sign (5), dha (5), ba (3), kha (3), cha (2).
    expect(coverage.byCategory["Lipi (script and orthography)"]!).toEqual({
      enumerated: 16,
      covered: 11,
    });
    // The two columns that carry this track.
    expect(coverage.byCategory["Kriya (the verb)"]!.covered).toBeGreaterThan(15);
    expect(coverage.byCategory["Vyavahaaram (communicative functions)"]!.covered).toBeGreaterThan(30);
    expect(formatExamCoverage(coverage)).toContain(
      "malayalam A1 (partial inventory): 162/243 points covered (67%)",
    );
  }, 60_000);
});

describe("the committed Punjabi A1 inventory", () => {
  const inventory = loadExamInventory("punjabi", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "punjabi");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; punjabiSpecific?: boolean };
      expect(cast.punjabiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([]);
  });

  it("names the A1 paper it measures against, and says the mocks are missing", () => {
    // Punjabi is the FIRST proxy-derived inventory that can point at a checked-in
    // task shape, so its `about` must not copy the "EXAM ENVELOPE: NONE EXISTS"
    // sentence the Malayalam and Kannada files carry. It says the opposite, and
    // the anchor that lets a point cite a paper part has to exist.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/EXAM ENVELOPE: AN A1 TASK SHAPE EXISTS, AND THE MOCKS DO NOT/);
    expect(inventory.about).not.toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/punjabi\/mocks\/ DOES NOT EXIST/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    const anchors = (inventory as unknown as { anchors: { id: string }[] }).anchors;
    expect(anchors.map((anchor) => anchor.id)).toContain("PA-TASK-SHAPES");
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("claims a tone column and a BINARY honorific, both measured here", () => {
    // Tone is Punjabi's own: no Spanish point comes near it, and the corpus
    // teaches four tone atoms plus the letter that writes tone without being
    // said. The honorific is deliberately two-way — `aap` appears once in 226
    // lessons and only as the HINDI word, so no three-way system is claimed.
    expect(inventory.about).toMatch(/TWO COLUMNS ARE PUNJABI'S OWN, AND BOTH WERE MEASURED HERE/);
    expect(inventory.about).toMatch(/BINARY: tu against tusi/);
    const tone = inventory.points.filter((point) => point.category.startsWith("Sur ("));
    expect(tone).toHaveLength(7);
    const register = inventory.points.filter((point) => point.category.startsWith("Bolchaal"));
    expect(register).toHaveLength(5);
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

  it("reports an EMPTY joining column, and a script closed over the corpus only", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(227);
    expect(coverage.covered).toBe(112);
    expect(coverage.unmapped).toBe(115);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE, and it is the starkest of the three tracks measured in this
    // series. ZERO of eleven. Not one of `te`/`ate`, `jaan`, `par`/`lekin`,
    // `kyunki`, `je`, the complementiser `ki`, `jadon` or `jo` occurs anywhere in
    // 226 lessons, in Gurmukhi or in romanisation — every apparent hit is a
    // script-drill syllable or a substring. The longest structure the track
    // teaches is a four-slot single clause, so `a1-writing-reader-purpose-message`
    // in the checked-in A1 task shape asks for a message this corpus cannot
    // produce. Malayalam's column came back 2/11 on the same walk; Punjabi's is
    // empty, and the difference was measured rather than assumed.
    const joining = coverage.byCategory["Jorr (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 0 });
    // Two demonstratives, neither taught — which is why nothing in the track can
    // be pointed at.
    expect(coverage.byCategory["Sanketak (demonstratives and deixis)"]!).toEqual({
      enumerated: 2,
      covered: 0,
    });
    // DO NOT READ THIS AS "the script is done". Closure over the CORPUS is
    // perfect — 50 of 50 characters used in headwords are taught — and closure
    // over the ALPHABET is not: seven akhar and six of the ten digits are never
    // taught. The one uncovered point in this column is exactly that distinction.
    expect(coverage.byCategory["Gurmukhi (script and orthography)"]!.covered).toBe(10);
    // The two columns that carry this track, and they are not the ones the
    // Dravidian tracks lead on.
    expect(coverage.byCategory["Faram (filling in a form)"]!).toEqual({ enumerated: 10, covered: 9 });
    expect(coverage.byCategory["Sur (tone and pronunciation)"]!.covered).toBe(6);
    expect(formatExamCoverage(coverage)).toContain(
      "punjabi A1 (partial inventory): 112/227 points covered (49%)",
    );
  }, 60_000);
});

describe("the committed Gujarati A1 inventory", () => {
  const inventory = loadExamInventory("gujarati", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "gujarati");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; gujaratiSpecific?: boolean };
      expect(cast.gujaratiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([]);
  });

  it("says an A1 task shape does NOT exist, unlike Punjabi's", () => {
    // The three inventories in this series have three different envelopes and
    // each `about` has to state its own. Malayalam: nothing at all. Punjabi: a
    // checked-in A1 paper and no mocks. Gujarati: a pre-A1 paper only, with
    // `assessment.json` pointing at an `a1.json` and fourteen mocks that are not
    // on disk. Copying a sibling's sentence here would have claimed an envelope
    // this track does not have.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(
      /EXAM ENVELOPE: A PRE-A1 TASK SHAPE EXISTS AND AN A1 ONE DOES NOT/,
    );
    expect(inventory.about).toMatch(/task-shapes\/a1\.json does not exist/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from lesson BODIES, because the frontmatter is uniformly neutral", () => {
    // The finding worth protecting. All 228 lessons declare `register: neutral`,
    // so a tool trusting frontmatter would report that this track makes no
    // register distinction at all. It teaches a tu/tame contrast in four coupled
    // places — the pronoun, the copula, the possessive and the farewell — none
    // of which is visible in any frontmatter field.
    expect(inventory.about).toMatch(
      /THE REGISTER COLUMN IS DERIVED FROM LESSON BODIES, NOT FROM FRONTMATTER/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Bhaashaashaili"));
    expect(register).toHaveLength(5);
    // Gender is the track's own column and its deepest grammar: five of the
    // corpus's eight grammar atoms are about it, and Gujarati keeps the Sanskrit
    // neuter that Hindi and Punjabi lost.
    expect(inventory.about).toMatch(/GENDER IS THIS TRACK'S OWN COLUMN/);
    const gender = inventory.points.filter((point) => point.category.startsWith("Ling ("));
    expect(gender).toHaveLength(7);
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

  it("reports an EMPTY joining column, a CLOSED gender column, and no digits at all", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(210);
    expect(coverage.covered).toBe(100);
    expect(coverage.unmapped).toBe(110);
    expect(coverage.partial).toBe(0);
    // The headline, and the second empty joining column in this series. `ane`
    // ("and") returns ZERO occurrences in 228 files — the word is simply not in
    // the corpus — and every raw match for `ke`, `jo` and `je` was checked in
    // context and is a substring of `kem`, `aavjo`, `jovu` or `aavje`. Punjabi's
    // column is also 0/11; Malayalam's came back 2/11. Each was measured.
    expect(coverage.byCategory["Jodaan (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 0,
    });
    // The other end. Gender is the one thing this track teaches deeply, and it
    // is the only FULL column in the file.
    expect(coverage.byCategory["Ling (grammatical gender, of which Gujarati has three)"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    // Script closure over the corpus is exact (43 of 43, holding even over
    // lesson bodies) while fourteen alphabet letters and ALL TEN DIGITS are
    // never taught. Reading the first number alone would say the script is
    // finished; the uncovered points in this column are that distinction.
    expect(coverage.byCategory["Lipi (script and orthography)"]!.covered).toBe(9);
    expect(formatExamCoverage(coverage)).toContain(
      "gujarati A1 (partial inventory): 100/210 points covered (48%)",
    );
  }, 60_000);
});
