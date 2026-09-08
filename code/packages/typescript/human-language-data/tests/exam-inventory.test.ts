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
    //
    // 33 -> 42, AND NOT ONE LESSON CHANGED. Nine points were taught in full and
    // carried `probe: null`, which this module documents as "no atom in the
    // corpus corresponds to this point" -- a finding, scored uncovered. Here it
    // was not a finding: it was 41 points nobody had written a probe for, and the
    // chapters that closed nine of them were generated after the inventory was
    // authored. Each was confirmed by reading the lesson, not by an atom name
    // looking right:
    //
    //   A1-V-01  A1-V-02   chapter 19 gives etre ONE LESSON PER PERSON and
    //     chapter 17 does the same for avoir, so there are twelve atoms where an
    //     author looking for `FR-VERB-ETRE-PRESENT` finds none. That shape is not
    //     an accident: `maxNewGrammarCellsPerLesson` is 1, so a paradigm CANNOT be
    //     one atom, and an inventory that expects one will always read it as absent.
    //   A1-V-14  A1-V-15   the avoir-perfect (chapter 18) and the etre-perfect with
    //     its agreement (chapter 20), including FR-C16-accord-unifie, which shows
    //     the two agreement rules are one rule.
    //   A1-P-01   six subject pronouns across six lessons, plus the rule that makes
    //     them obligatory -- parle, parles and parlent are one sound.
    //   A1-N-02   FR-C01-le-la: "a grammatical gender baked into the noun ... the
    //     gender can't be guessed."
    //   A1-A-03   FR-C13-vin-rouge states the position rule AND its exception.
    //   A1-PRON-05  FR-W02-cedille.
    //   A1-LEX-08   le temps, il fait chaud, il pleut -- a whole weather lesson.
    //
    // The other 32 now each carry a note saying what the track holds and what is
    // missing, so the next author does not repeat the 232-lesson read.
    expect(coverage.covered).toBe(42);
    expect(coverage.byCategory["L'interrogation"]).toEqual({ enumerated: 5, covered: 5 });
    // Lexique de base closes outright: ten of ten. It is the column vocabulary
    // work moves, and it has now run out of room, which is the finding -- every
    // remaining point in this inventory is grammar or a function word.
    expect(coverage.byCategory["Lexique de base"]).toEqual({ enumerated: 10, covered: 10 });
    // The shape, not the score: the sentence-level categories are still empty.
    // No quantity of headwords moves these -- only grammar chapters like the one
    // that closed L'interrogation.
    for (const empty of ["La phrase", "Les prepositions"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
  }, 60_000);

  it("never lets an unmapped point read as 'nobody has looked yet'", () => {
    // The rule Marathi has had since HL-C290. Thirty-two of this inventory's 41
    // unmapped points carried no note, so "the corpus does not teach it" and
    // "nobody has checked" were the same JSON -- and nine of the 41 turned out to
    // be taught in full. A note is what stops that read being redone.
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim(), `${point.id} is unmapped and must say why`).toBeTruthy();
    }
  });
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
    const { lessons } = loadEverything();
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
    expect(coverage.covered).toBe(37);
    // Der Artikel leaves this list at 2/5 — der/die/das and ein/eine were always
    // taught. Questions and prepositions are genuinely empty: no lesson in the
    // track owns wer, wann, warum, welcher, or any preposition as a preposition.
    for (const empty of ["Die Frage", "Die Praeposition"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    expect(coverage.byCategory["Der Artikel"]).toEqual({ enumerated: 5, covered: 2 });
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

  // The ordinal point, named rather than left to the aggregate. The 162/301
  // total above would move if this probe were nulled, but it would move for a
  // hundred other reasons too; this pins WHICH eleven atoms close the point and
  // that every one of them is really taught. Both halves were falsified before
  // this was kept: adding a fabricated id fails the "probes only atoms that
  // EXIST" test above, and nulling the probe fails the coverage total.
  it("closes MR-A1-QU-04 on eleven atoms, ten words and one rule", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "marathi");
    const ordinals = inventory.points.find((point) => point.id === "MR-A1-QU-04");
    expect(ordinals?.probe).toEqual([
      // The set itself, taught on dusraa without a new word.
      "MR-GRAMMAR-ORDINAL-SET",
      // The four Marathi INHERITED. dusraa is absent on purpose: chapter 31
      // already taught MR-LEX-DUSRA, and the tranche re-opens it rather than
      // teaching it again, so claiming a new lexical atom for it would be a
      // fabrication.
      "MR-LEX-PAHILA",
      "MR-LEX-TISRA",
      "MR-LEX-CHAUTHA",
      // The seam, and the rule that starts at it.
      "MR-LEX-PACHVA",
      "MR-GRAMMAR-ORDINAL-VA",
      // The five it BUILDS.
      "MR-LEX-SAHAVA",
      "MR-LEX-SATVA",
      "MR-LEX-AATHVA",
      "MR-LEX-NAVVA",
      "MR-LEX-DAHAVA",
    ]);
    for (const atom of ordinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // And the word the tranche does NOT re-teach is taught all the same, in the
    // chapter the tranche points back to.
    expect(taught.has("MR-LEX-DUSRA")).toBe(true);
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
    // WHAT CHANGED, AND WHY THE ASSERTION IS NOW A DIFFERENT SHAPE.
    //
    // The count used to be pinned at 13-or-more, and a floor on a count of
    // KNOWN DEFECTS is a bad pin: it only ever rises, it rewards nobody for
    // clearing one, and it went stale in the flattering direction for the
    // AUTHOR rather than for the corpus. All 26 schema-v1 lessons were
    // migrated to v2 in the chapters 9-12 work, and for a whole tranche
    // afterwards this file still said the material was unmeasurable -- so
    // eight points read as content debt while the teaching sat in the book,
    // already done. That is the failure mode worth pinning against.
    //
    // The SCHEMA-V1 class is therefore asserted CLOSED, permanently: no point
    // note may carry that marker again, because there is no schema-v1 lesson
    // left in the track to carry it. EMPTY-INTRODUCES is the class that
    // survives, it hides inside v2, and at least one point must still name it
    // -- a zero there would mean somebody deleted the distinction rather than
    // fixed it.
    const marked = inventory.points.filter((point) => (point.note ?? "").includes("MEASUREMENT GAP"));
    expect(marked.length).toBeGreaterThanOrEqual(1);
    for (const point of marked) expect(point.probe, point.id).toBeNull();
    const staleClass = inventory.points.filter((point) =>
      (point.note ?? "").includes("SCHEMA-V1 MEASUREMENT GAP"),
    );
    expect(staleClass.map((point) => point.id)).toEqual([]);
    expect(marked.some((point) => (point.note ?? "").includes("EMPTY-INTRODUCES"))).toBe(true);
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
    // 111 -> 124: the asking-word tranche (chapters 37-40), and two different
    // kinds of movement inside one number, which is why they are reported
    // apart. SEVEN points were EARNED by the nine new items: the interrogative
    // pronouns and adverbs, the word-order rule behind them, asking about a
    // person or a place, the ithe/tithe pair, and the dental row, which closed
    // because one missing letter (थ) was the only thing keeping तिथे
    // unwritable. SIX more were already taught and only LOOKED uncovered,
    // because their notes still described chapters 9 to 12 as schema-v1 after
    // those chapters had been migrated -- the politeness contrast, asking a
    // name, asking how somebody is, asking for an evaluation, working
    // activity, and the imperative. Nothing was authored for those six; a
    // stale note was corrected. Work therefore leaves the empty-category list
    // -- on a note fix, not on a lesson, which is worth saying plainly.
    //
    // 157 -> 161: the numbers tranche (chapters 55-59). Nineteen items for
    // four points is the WORST ratio in this file, and it was taken anyway,
    // because MR-A1-QU-03 -- cardinals to a hundred -- is the most blocking
    // single point left: age, value and price, personal data on a form and the
    // interview's opening question all wait on a number above five, and none
    // of them can move until the count does. Six to twenty is the half of it
    // that fits in one tranche; the tens are the other half. Quantifiers 2/6
    // to 4/6, Shopping and "Money and the economy" off the empty-category
    // list, Devanagari letters and signs 15/24 to 17/24.
    //
    // 151 -> 157: the degree-and-amount tranche (chapters 53-54), which
    // teaches no adjective at all and multiplies the eight the previous one
    // taught. Seven items, six points, and the ratio is the point: khuup and
    // jaraa alone close three (ADJ-06, AP-01, ADV-03), because every one of
    // those points was blocked behind SPINE-DESCRIBE-QUALITIES rather than
    // behind its own vocabulary. The adjective 5/7 to 6/7, "The adjective
    // phrase" off the empty-category list at 1/1, Adverbs 6/8 to 7/8.
    //
    // 142 -> 151, which is exactly half. The adjective tranche (chapters
    // 49-52) closes SPINE-DESCRIBE-QUALITIES, an A1 CORE node this track had
    // never realized: forty-eight chapters could name a house, put a room in
    // it and say who it belonged to, and could not say one thing about what
    // it was like. Twelve items, nine points. The adjective went 2/7 to 5/7,
    // Evaluative notions 4/8 to 7/8, "The person: the body" to 3/3 and "The
    // person: character" off the empty-category list.
    //
    // 133 -> 142: the accompaniment tranche (chapters 45-48). Eleven items --
    // three "with" endings that English collapses into one, the pronoun's own
    // oblique, two ablatives and their question word, the animate object
    // marker in its two remaining jobs, and -kade, which is how a language
    // with no verb for HAVE says somebody has something. Case and
    // postpositions went 3/6 to 5/6, Spatial notions 4/7 to 6/7, and The verb
    // phrase 3/6 to 5/6. Only paryant keeps the postposition column open, and
    // it is blocked on the same reph that MR-A1-OR-21 is: one script lesson
    // pair closes both.
    //
    // 124 -> 133: the place tranche (chapters 41-44). Eleven items -- two nouns,
    // the oblique stem, five postpositions and three ordinary place words --
    // closed nine points, and the ratio comes from the STEM rather than from the
    // vocabulary: MR-A1-N-09 is a single rule that four other points were
    // sitting behind. Spatial notions went 1/7 to 4/7, Case and postpositions
    // 1/6 to 3/6, Existential notions 0/5 to 2/5, and Housing leaves the
    // empty-category list on its first lesson. Demonstratives, Temporal notions
    // and Shopping stay in it, which is the remaining work.
    //
    // 161 -> 162: the ordinal tranche (chapters 60-61) closes MR-A1-QU-04, the
    // last vocabulary point in the Quantifiers column and the one HL-C350
    // named as the weakest column in the corpus. ONE point for twelve lessons
    // is a poor ratio and it is the honest one: ordinals unlock nothing else in
    // this inventory, because MR-A1-NG3-06 wants ordering EXPONENTS -- aadhii,
    // nantar, mag -- rather than more ordinals, and none of the three is taught
    // anywhere in the track. Its note now says so rather than saying
    // "Untaught".
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(301);
    expect(coverage.covered).toBe(162);
    expect(coverage.unmapped).toBe(139);
    // Zero partials is a property of the "existing atoms only" rule above, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    for (const empty of ["Demonstratives", "Temporal notions", "Personal identity"]) {
      expect(coverage.byCategory[empty]?.covered, empty).toBe(0);
    }
    expect(coverage.byCategory["Coordination"]!.covered).toBe(5);
    expect(coverage.byCategory["Devanagari letters and signs"]!.covered).toBeGreaterThan(0);
    expect(coverage.byCategory["Sound system"]!.covered).toBeGreaterThan(0);
    expect(formatExamCoverage(coverage)).toContain(
      "marathi A1 (partial inventory): 162/301 points covered (54%)",
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
    expect(coverage.covered).toBe(175);
    expect(coverage.unmapped).toBe(87);
    // 174 -> 175: the HL-C354 ordinal tranche closed TA-A1-NUM-04 (chapters
    // 82-83). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    // The two still open in that column are TA-A1-NUM-03 (above twenty) and
    // TA-A1-NUM-07 (measures); neither is ordinal work.
    expect(coverage.byCategory["Eṇṇuppeyar (numerals and quantity)"]!).toEqual({
      enumerated: 8,
      covered: 6,
    });
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
      "tamil A1 (partial inventory): 175/262 points covered (67%)",
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
    expect(coverage.covered).toBe(194);
    expect(coverage.unmapped).toBe(64);
    expect(coverage.partial).toBe(0);
    // 193 -> 194: the HL-C354 ordinal tranche closed KA-A1-NUM-07 (chapters
    // 74-75). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    expect(coverage.byCategory["Sankhye (numerals and quantity)"]!).toEqual({
      enumerated: 8,
      covered: 7,
    });
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
      "kannada A1 (partial inventory): 194/258 points covered (75%)",
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
    expect(coverage.covered).toBe(163);
    expect(coverage.unmapped).toBe(80);
    expect(coverage.partial).toBe(0);
    // 162 -> 163: the HL-C354 ordinal tranche closed ML-A1-NUM-05 (chapters
    // 67-68). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    // The two still open in that column are ML-A1-NUM-04 (counting past
    // twenty) and ML-A1-NUM-08 (measures); neither is ordinal work.
    expect(coverage.byCategory["Sankhya (numerals and quantity)"]!).toEqual({
      enumerated: 9,
      covered: 7,
    });
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
      "malayalam A1 (partial inventory): 163/243 points covered (67%)",
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

  it("reports a joining column that is no longer empty, and a script closed over the corpus only", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(227);
    // 112 -> 136. Chapters 37-43 answer this file's own uncovered list.
    // 136 -> 137: the ordinal tranche closes PA-A1-NUM-05 (chapter 44). One
    // point for six lessons, and no more than one: ordinals unlock nothing else
    // here, because the count still stops at panj -- PA-A1-NUM-02 and -03 are
    // both open -- so nothing above fifth can be said at all.
    expect(coverage.covered).toBe(137);
    expect(coverage.unmapped).toBe(90);
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
    // THE HEADLINE HAS CHANGED, and this assertion records it. It read
    // `covered: 0` -- not one of te/ate, jaan, par/lekin, kyunki, je, the
    // complementiser ki, jadon or jo occurred anywhere in 226 lessons, so the
    // longest structure the track taught was a four-slot single clause and the
    // A1 writing paper asked for a message the corpus could not produce.
    //
    // The finding under the finding is why it was cheap: ELEVEN of the eleven
    // devices needed NO NEW SIGN. Every one is spelled in Gurmukhi the track
    // taught long ago. This was never a script debt -- nobody had written the
    // words down. The one new letter in seven chapters (tha) was bought for a
    // question word, not for a joining word.
    const joining = coverage.byCategory["Jorr (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 10 });
    // Two columns went FULL, and neither was the target: the clause pattern
    // closed negation, and the par/lekin doublet closed the register rule the
    // file said one lesson would close.
    expect(coverage.byCategory["Nanh (negation)"]!).toEqual({ enumerated: 5, covered: 5 });
    expect(coverage.byCategory["Bolchaal (register: familiar and respectful, Sanskritic and Perso-Arabic)"]!)
      .toEqual({ enumerated: 5, covered: 5 });
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
    // The ordinal point, named rather than left to the aggregate, so a nulled
    // probe or a fabricated id is caught here and not only by the total. Both
    // halves were falsified before this was kept.
    const ordinals = inventory.points.find((point) => point.id === "PA-A1-NUM-05");
    expect(ordinals?.probe).toEqual([
      // Second first, because duujaa still shows its doo; the set named beside it.
      "PA-LEX-DUJA-01",
      "PA-GRAMMAR-ORDINAL-SET-01",
      "PA-LEX-TIJA-01",
      "PA-LEX-CHAUTHA-01",
      // First arrives fourth: pahilaa keeps no letter of ikk.
      "PA-LEX-PAHILA-01",
      // And the seam, where an inherited word came to look like a sum.
      "PA-LEX-PANJVAN-01",
      "PA-GRAMMAR-ORDINAL-VAAN-01",
    ]);
    expect(
      coverage.points.find((point) => point.id === "PA-A1-NUM-05")?.missingAtoms,
    ).toEqual([]);
    // And the point it does NOT close, which is why the ratio is one for six:
    // the cardinals stop at five, so sixth upward cannot be said.
    expect(coverage.points.find((point) => point.id === "PA-A1-NUM-02")?.covered).toBe(false);
    expect(formatExamCoverage(coverage)).toContain(
      "punjabi A1 (partial inventory): 137/227 points covered (60%)",
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

  it("reports a joining column that is no longer empty, and the digits that still are", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(210);
    // 100 -> 120. Chapters 35-41 answer this file's own uncovered list: seven
    // chapters, five items each, one new item per lesson.
    // 120 -> 121. HL-C359 closes GU-A1-NUM-05, the ordinal point, with chapter
    // 42. Sankhya goes 1/5 to 2/5; the three still open are all the count
    // stopping at five, not the ordinal.
    // 121 -> 122. HL-C361 closes GU-A1-NUM-03, the cardinals six to ten, with
    // chapter 43. ONE point, and the total is the least of what moved: the
    // reachable count doubles (highest numeral 5 -> 10), the track's numeral
    // LESSONS go 1 -> 6, and the ordinal rule stops being a single worked
    // example and becomes productive above the exception list -- none of which
    // a coverage total can see, because every one of them deepens a tick that
    // was already there.
    expect(coverage.covered).toBe(122);
    expect(coverage.unmapped).toBe(88);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE HAS CHANGED, and this assertion is the record of it. It read
    // `covered: 0` and was the starkest finding in the file: `ane` returned ZERO
    // occurrences in 228 lessons, so a learner could not say "tea and milk"
    // although both words were taught on facing pages.
    //
    // The finding under the finding is why it was cheap to fix. TEN of the
    // eleven joining devices needed NO NEW SIGN -- ane, athava, pan, ke, kemke,
    // maate, tethi, jo, jyaare and je are all spelled in glyphs the track taught
    // before chapter 30. This was never a script debt. Nobody had written the
    // words down.
    expect(coverage.byCategory["Jodaan (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 10,
    });
    // The one left open is the distributive, and it needs no new glyph either --
    // it needs a paired correlative, which is a lesson rather than a script step.
    // Gender is unchanged and still the only FULL column in the file.
    expect(coverage.byCategory["Ling (grammatical gender, of which Gujarati has three)"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    // Unchanged, and deliberately so. Script closure over the corpus stays exact
    // -- now 44 of 44, because this tranche spent exactly ONE new letter, the
    // `pha` that `maaf karo` required -- while fourteen alphabet letters and ALL
    // TEN DIGITS are still never taught. Reading the first number alone would say
    // the script is finished; the uncovered points in this column are that
    // distinction, and seven chapters of joining did not touch it.
    expect(coverage.byCategory["Lipi (script and orthography)"]!.covered).toBe(9);
    // Unchanged again after chapter 43, and that is the point worth keeping: the
    // cardinal tranche spent exactly one new letter (44 of 44 glyphs taught
    // becomes 45 of 45) and did not touch a single DIGIT. GU-A1-NUM-08 is still
    // zero of ten, and is now the only reading gap left below ten.
    expect(
      inventory.points.find((point) => point.id === "GU-A1-NUM-08")?.probe,
    ).toBeNull();
    // The columns the tranche moved that it was not aiming at: the negator alone
    // closed the can't-say-I-don't-understand function, and the question family
    // closed four at once.
    expect(coverage.byCategory["Nakaar (negation)"]!).toEqual({ enumerated: 4, covered: 3 });
    expect(coverage.byCategory["Prashna (asking questions)"]!).toEqual({ enumerated: 10, covered: 8 });
    expect(formatExamCoverage(coverage)).toContain(
      "gujarati A1 (partial inventory): 122/210 points covered (58%)",
    );
  }, 60_000);

  // The ordinal point, named rather than left to the aggregate. Both halves
  // were falsified before this was kept: a fabricated id fails the "probes only
  // atoms that EXIST" test above, and nulling the probe fails the coverage
  // total two assertions up.
  it("closes GU-A1-NUM-05 on eight atoms, five words and two shapes", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "gujarati");
    const ordinals = inventory.points.find((point) => point.id === "GU-A1-NUM-05");
    expect(ordinals?.probe).toEqual([
      // The four the language HANDS DOWN, in the order the chapter teaches
      // them -- which runs by how much of the cardinal survives, not by number.
      "GU-LEX-PAHELU",
      "GU-LEX-BIJU",
      "GU-LEX-TRIJU",
      "GU-LEX-CHOTHU",
      // The one it BUILDS, and the suffix that builds it. This is the lesson
      // the chapter OPENS on, because the four above are exceptions to it.
      "GU-LEX-PANCHMU",
      "GU-GRAMMAR-ORDINAL-MU",
      // The shape beejun and treejun share, claimed only once there are two of
      // them: one word is not a shape.
      "GU-GRAMMAR-ORDINAL-IJU",
    ]);
    for (const atom of ordinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // The cardinal the rule stands on is taught, and is the reason the chapter
    // opens where it does.
    expect(taught.has("GU-LEX-NUMBERS-ONE-TO-FIVE")).toBe(true);
  }, 60_000);

  // The cardinal point, named rather than left to the aggregate. Both halves
  // were falsified before this was kept: a fabricated id fails the "probes only
  // atoms that EXIST" test above AND the per-atom loop below, and nulling the
  // probe fails the coverage total two tests up as well as this file's own
  // `toEqual` on the probe list.
  it("closes GU-A1-NUM-03 on five cardinals, one letter and a rule with both edges", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "gujarati");
    const cardinals = inventory.points.find((point) => point.id === "GU-A1-NUM-03");
    // In COUNTING order, because for six to ten that IS the construction order:
    // Wiktionary's -mun entry ends its exception list at chha, so the reader
    // crosses the boundary between exception and rule exactly once, between the
    // first lesson and the second. Ordering by cost was available -- four of the
    // five need no new sign -- and was rejected, because it buys nothing and
    // leaves the reader counting round a hole at eight.
    expect(cardinals?.probe).toEqual([
      "GU-LEX-CHHA-SIX",
      "GU-LEX-SAAT",
      "GU-LEX-AATH",
      "GU-LEX-NAV",
      "GU-LEX-DAS",
    ]);
    for (const atom of cardinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // The chapter's other two atoms are NOT in the probe, because neither is a
    // cardinal: the one new letter, and the rule that names both edges of the
    // -mun exception list. Both are taught, and the rule atom is the reason the
    // ordinals above sixth need no lesson of their own.
    expect(taught.has("GU-SCRIPT-TTHA-01")).toBe(true);
    expect(taught.has("GU-GRAMMAR-ORDINAL-MU-REACH")).toBe(true);
    // What is NOT claimed, asserted rather than left to be inferred: the
    // irregular sixth ordinal is printed once with its source and taught as no
    // atom at all, so no GU-LEX-* id for it exists anywhere in the corpus.
    expect(taught.has("GU-LEX-CHHATHTHU")).toBe(false);
    // And the count still stops at ten. GU-A1-NUM-04 wants agiyaar upward.
    expect(inventory.points.find((point) => point.id === "GU-A1-NUM-04")?.probe).toBeNull();
  }, 60_000);
});

// ---------------------------------------------------------------------------
// The first inventory for a track whose exam-levels entry carries NO CAVEAT.
//
// Every earlier file in this series had an external steer to answer. Japanese
// is told JLPT tests no production; Chinese that its CEFR correspondence is
// unpublished; Tamil that it is diglossic; Sanskrit that a traditional
// syllabus is not parallel to CEFR. Russian's entry says exam TORFL, basis
// published, mapping A1 to TEU — and stops.
//
// So the columns this file opens beyond the Spanish set had to be MEASURED
// into existence rather than imported. Walking the proxy's five noun points,
// eight pronoun points and six verb-phrase points against the corpus showed
// that every one of them resolved to a question about CASE, so case is a
// category here rather than a footnote; the same walk over the verb column
// resolved to ASPECT. Ten of the file's fourteen russianSpecific grammar
// points live in those two categories, and neither has any Spanish column.
//
// Two results are worth pinning as SHAPE rather than as size:
//
//   1. The joining column was 0 of 13 when this file was written — the sixth
//      track running, and the first that is neither Indo-Aryan nor Dravidian.
//      `i` ("and") was PRINTED as a conjunction in two lesson bodies and
//      introduced by no lesson at all, which was a hair better than Gujarati's
//      `ane` at zero occurrences and worse in one way: the word was on the page
//      doing work the reader was never told about.
//   2. The repair column was HALF closed, which no percentage would show. The
//      track taught `ya ne ponimayu` and `ya ne znayu` in full, and had no
//      word for sorry and no way to ask for a repeat: `izvinite`, `prostite`,
//      `povtorite` and `medlenno` were each zero across all 88 files.
//
// BOTH ARE NOW CLOSED, by chapters 16-22 (35 lessons, 57 atoms). Thirteen of
// thirteen joining devices are taught and the track can produce a two-clause
// sentence for the first time; the repair kit exists. The assertions below are
// re-pinned at the new figures and the shape claims are kept, in the past
// tense, because the finding they record is about how this corpus was authored
// rather than about where it stands today.
// ---------------------------------------------------------------------------
describe("the committed Russian A1 inventory", () => {
  const inventory = loadExamInventory("russian", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name, and the one two earlier agents broke.
    // A probe pointing at an id somebody expects a future lesson to introduce
    // resolves to "not introduced" forever and sits in the report
    // indistinguishable from the honest gaps around it.
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "russian");
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

  it("transfers the WHOLE proxy, which is the finding for an alphabetic Indo-European track", () => {
    // Tamil and Kannada each dropped two points and Marathi three. Russian
    // drops NONE, including A1-O1-06 (superscript letters in abbreviations),
    // which reads as untransferable Spanish typography and is really the demand
    // that a numeral carry a written grammatical ending — Russian's hyphenated
    // `1-y`. Restating a point around the target language's machinery IS
    // deriving it, so an empty notTransferred here is a claim, not an omission.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: unknown[]; note: string };
    }).proxy;
    expect(proxy.notTransferred).toEqual([]);
    expect(proxy.note).toMatch(/EMPTY ON PURPOSE/);
    expect(proxy.note).toMatch(/A1-O1-06/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; russianSpecific?: boolean };
      expect(cast.russianSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // Case, aspect, the two soundless letters, the cursive hand, vowel
    // reduction, the hard/soft contrast, the yery vowel, the impersonal plural,
    // the ty/vy choice as a social act, and the joining column measured as a
    // column. None of these has a Spanish point to derive from.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    // Russian is the first track in this series with a REAL exam and a
    // PUBLISHED CEFR mapping, which makes the borrowing risk higher here than
    // anywhere before it: a reader could take this file for a TORFL syllabus.
    expect(inventory.about).toMatch(/NOT A TRANSCRIPTION OF THAT EXAM'S SYLLABUS/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO THE TORFL SYSTEM/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/NO SEARCH WAS RUN, BY INSTRUCTION/);
    // The caveat that is not there, and what was done instead.
    expect(inventory.about).toMatch(/NO CAVEAT/);
    expect(inventory.about).toMatch(/THE CASE COLUMN WAS OPENED BY MEASUREMENT INSTEAD/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Russian, unlike Tamil and Sanskrit, HAS an assessment contract — and its
    // A1 half dangles. Saying "partial and dangling" is what stops the contract
    // from reading as evidence it is not.
    expect(inventory.source).toMatch(/EXAM ENVELOPE: PARTIAL AND DANGLING/);
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

  it("reports a CASE-shaped and JOINING-shaped gap, with the letters nearly closed", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(228);
    // 104 -> 107: HL-C350's numeral tranche (chapters 23-26) closes RU-A1-Q-01,
    // RU-A1-Q-02 and RU-A1-L-09.
    expect(coverage.covered).toBe(107);
    expect(coverage.unmapped).toBe(121);
    // Zero partials is a property of the "existing atoms only" rule, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    // THE HEADLINE. Six cases gate every Russian noun, adjective, pronoun and
    // numeral, and the track teaches two contrasts: the object pronouns
    // (menya/vas) and the genitive after `do`. Everything else — accusative of
    // nouns, dative, prepositional, instrumental, and the whole plural — is
    // absent, and the plural is absent outright rather than late.
    expect(coverage.byCategory["Padezh - case"]!).toEqual({ enumerated: 10, covered: 3 });
    // The sixth empty joining column in the series, and the first outside
    // South Asia — CLOSED. Both halves are now full: five coordinators
    // (i, ili, ni ... ni, no/a, odin ... drugoy) and eight subordination
    // points (the bare infinitive, chto, kotoryy, potomu chto, chtoby, kogda,
    // li, and the column measured as a column).
    expect(coverage.byCategory["Sochinenie - joining two clauses"]!).toEqual({
      enumerated: 5,
      covered: 5,
    });
    expect(coverage.byCategory["Podchinenie - subordination"]!).toEqual({
      enumerated: 8,
      covered: 8,
    });
    // The repair column, which had no word for sorry at all. What is still
    // open is asking for somebody by name at a door, and asking somebody to be
    // quiet.
    expect(coverage.byCategory["Vesti razgovor - managing the conversation, and repairing it"]!)
      .toEqual({ enumerated: 7, covered: 5 });
    // Punctuation went 0/7 to 4/7: the full stop, the closing-only question
    // mark, the dash that stands in for the absent copula, and the comma rule
    // that could not be stated until the track owned a subordinator.
    expect(coverage.byCategory["Punktuatsiya - punctuation"]!).toEqual({ enumerated: 7, covered: 4 });
    // And the other end: 29 of the 33 Cyrillic letters have their own writing
    // lesson, which is the closest thing this track has to a finished column.
    // Unmoved by the joining tranche, and deliberately: every one of the
    // thirteen joining devices, both apologies, the whole question family and
    // all four punctuation marks were checked against the union of taught
    // glyphs before a lesson was designed, and not one of them needed a new
    // sign. The three untaught lower-case letters — shcha, the hard sign and
    // e-oborotnoe — do not occur in any of them.
    // Reading that number alone would say the script is done; the five
    // uncovered points here — the letter names, the four untaught letters, the
    // cursive hand, the lower-case rule, the spelling rule — are the
    // distinction.
    // 5 -> 6: the written ordinal, 1-y, is the one letters point that was a
    // numerals point seen from the other side. Its derivation was written to
    // defend a transfer from Spanish superscripts and now pays off: the demand
    // is a numeral carrying a written grammatical ending, and Russian answers it
    // with a hyphen instead of a superscript.
    expect(coverage.byCategory["Kirillitsa - the letters"]!).toEqual({ enumerated: 10, covered: 6 });
    // THE COLUMN THAT HELD EXACTLY ONE NUMBER. odin was taught as half of the
    // odin ... drugoy joining pattern rather than as a numeral, so the track
    // could ask skolko and understand no answer to it. HL-C350 closes the
    // cardinals and the ordinals at NO new Cyrillic letter, and leaves the two
    // that are not downstream of a numeral.
    expect(coverage.byCategory["Chislitelnye i kolichestvo - numerals and quantity"]!)
      .toEqual({ enumerated: 5, covered: 2 });
    // The case government the numerals impose is NOT claimed. dva/tri/chetyre
    // take the genitive singular and pyat upward the genitive plural, and this
    // track has taught the genitive in one place, after `do`. RU-C25-practice
    // tells the reader so in as many words, and the atom below is the record of
    // their having been told -- which is a different thing from teaching it.
    const cardinals = inventory.points.find((point) => point.id === "RU-A1-Q-01")!;
    expect(cardinals.probe).toContain("RU-GRAMMAR-TSAT-TENS-01");
    expect(cardinals.probe).not.toContain("RU-GRAMMAR-NUMERAL-CASE-01");
    expect(cardinals.note).toMatch(/Counting THINGS is a genitive chapter/);
    expect(inventory.points.find((point) => point.id === "RU-A1-C-05")!.probe).toBeNull();
    // Nothing in the corpus can be described, because no adjective is taught.
    expect(coverage.byCategory["Prilagatelnoe - the adjective"]!.covered).toBe(0);
    expect(formatExamCoverage(coverage)).toContain(
      "russian A1 (partial inventory): 107/228 points covered (47%)",
    );
  }, 60_000);
});

// ---------------------------------------------------------------------------
// The first inventory for a track with THREE writing systems, and the first
// whose exam anchor cannot score half the construct it is anchoring.
//
// Two things forced this file's shape, and both are asserted here.
//
//   1. THE SCRIPT COLUMN HAD TO BE THREE COLUMNS. This corpus stands in three
//      completely different places: 31 of 46 hiragana signs taught, 2 of 46
//      katakana, and 3 kanji. A single "script" column averages those into a
//      number that describes nothing and hides the fact that a reader who can
//      decode a hiragana sentence still cannot read a menu, a name or a sign.
//      The mixed-script fact — which is what a single column would most
//      obviously lose — is its own point at JA-A1-HYO-01.
//
//   2. THE CAVEAT SAYS THE EXAM SCORES NO PRODUCTION AND NO INTERACTION.
//      `exam-levels.json` records that the published CEFR indication covers
//      "only the language knowledge, reading, and listening competence JLPT
//      tests". An inventory that quietly reported reading and listening
//      coverage would flatter this track exactly the way HL20 §1 warns about.
//      So the four production tasks named in `japanese/assessment-spec.md#a1`
//      are enumerated as points one for one, plus interaction. One of five is
//      covered — and it is interaction, which is the half JLPT cannot see.
//
// The result worth reporting is not the percentage. Japanese is one of only two
// tracks in the whole corpus with ZERO findings in every gentle-ramp queue, and
// its REPAIR column reads 7 of 8: it is the only track measured so far that
// holds the complete CEFR A1 repair kit — apologise, report the failure, ask
// for a repeat, ask for slower speech, point, and confirm the repair worked.
// Russian has half of it. Chinese has one move. Gujarati had none.
//
// And its joining column is still 0 of 8. THAT is the finding: the
// best-constructed track in the corpus has the same empty column as the worst,
// which is the clearest evidence available that the hole is structural rather
// than a symptom of neglect.
// ---------------------------------------------------------------------------
describe("the committed Japanese A1 inventory", () => {
  const inventory = loadExamInventory("japanese", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "japanese");
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

  it("names the orthography points with NO Japanese analogue, one reason each", () => {
    // The instruction this file was written under: say which of the proxy's
    // orthography points have no Japanese analogue AT ALL. Three do, and they
    // are dropped in two entries rather than one, because the reasons differ
    // and a reader checking the file needs to see which is which.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    const dropped = proxy.notTransferred.flatMap((entry) => entry.spanishPoints).sort();
    expect(dropped).toEqual(["A1-O1-04", "A1-O1-05", "A1-O1-06"]);
    const caseEntry = proxy.notTransferred.find((e) => e.spanishPoints.includes("A1-O1-04"))!;
    // Caseless in ALL THREE scripts — and the reason romaji does not rescue the
    // demand the way pinyin does for Chinese, which is the comparison that
    // stops the two files looking inconsistent.
    expect(caseEntry.why).toMatch(/caseless in all three of its scripts/i);
    expect(caseEntry.why).toMatch(/IT DOES NOT SURVIVE IN ROMAJI/);
    expect(caseEntry.why).toMatch(/Chinese inventory keeps the same two points/);
    // And the superscript entry names the track that DERIVED it.
    const abbrev = proxy.notTransferred.find((e) => e.spanishPoints.includes("A1-O1-06"))!;
    expect(abbrev.why).toMatch(/RU-A1-L-09/);
    expect(proxy.note).toMatch(/A1-O1-01 becomes THREE points, one per script/);
  });

  it("splits the script into three columns, and measures each separately", () => {
    // The point of the whole exercise. One column would report a number that
    // describes nothing; three report that hiragana is two thirds done,
    // katakana has barely started, and kanji is three characters.
    const categories = new Set(inventory.points.map((point) => point.category));
    expect([...categories].filter((c) => c.startsWith("Hiragana")).length).toBe(1);
    expect([...categories].filter((c) => c.startsWith("Katakana")).length).toBe(1);
    expect([...categories].filter((c) => c.startsWith("Kanji")).length).toBe(1);
    // No script point may be probed with a lexis atom, and no romanized
    // headword may stand in for a sign — the mechanical half of JA-A1-HYO-06.
    const script = inventory.points.filter((point) =>
      /^(Hiragana|Katakana|Kanji)/.test(point.category),
    );
    for (const point of script) {
      for (const atom of point.probe ?? []) {
        expect(atom.startsWith("JA-LEX-"), `${point.id} probes a lexis atom`).toBe(false);
      }
    }
    // And the mixed-script fact exists as a point of its own, because it is the
    // thing a single column would have lost.
    const mixed = inventory.points.find((point) => point.id === "JA-A1-HYO-01");
    expect(mixed, "the three-scripts-on-one-line point must exist").toBeDefined();
    expect(mixed!.probe).not.toBeNull();
  });

  it("enumerates the production tasks the exam anchor cannot score", () => {
    // The caveat is the reason this category exists, so the file must quote
    // what it says rather than merely act on it.
    expect(inventory.about).toMatch(/JLPT does not test production \(speaking and writing\) or interaction/);
    expect(inventory.about).toMatch(/THE EXAM ANCHOR CANNOT SCORE HALF THE CEFR\s+CONSTRUCT/);
    const production = inventory.points.filter((point) => point.category.startsWith("Sanshutsu"));
    // Four companion tasks from assessment-spec.md#a1, plus interaction.
    expect(production).toHaveLength(5);
    // Exactly one of the four TASKS is reachable, and it is the role-play.
    const tasks = production.filter((point) => /^JA-A1-PROD-0[1-4]$/.test(point.id));
    expect(tasks.filter((point) => point.probe !== null).map((point) => point.id)).toEqual([
      "JA-A1-PROD-04",
    ]);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; japaneseSpecific?: boolean };
      expect(cast.japaneseSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // The particles, politeness, in-group/out-group, the mora, the two kanji
    // readings, the mixed-script fact, the ha/wa spelling, the romaji decision,
    // the three script censuses, the five repair points and the five
    // production/interaction points. None has a Spanish point to derive from.
    expect(specific.length).toBeGreaterThanOrEqual(25);
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/NOT A TRANSCRIPTION OF THE JLPT SYLLABUS/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO\s+THE JAPAN FOUNDATION/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/NO SEARCH WAS RUN, BY INSTRUCTION/);
    // The number this file must never invent.
    expect(inventory.about).toMatch(/NO JLPT KANJI OR\s+VOCABULARY LIST IS CITED ANYWHERE IN THIS FILE/);
    // Its prose envelope IS real and was used, which is what makes the
    // production points bindable rather than invented.
    expect(inventory.source).toMatch(/EXAM ENVELOPE: PARTIAL AND DANGLING, BUT ITS PROSE HALF IS REAL AND WAS USED/);
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

  it("reports a FULL repair column, an empty joining column, and three scripts at three depths", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(179);
    expect(coverage.covered).toBe(66);
    expect(coverage.unmapped).toBe(113);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE, and it is a strength rather than a gap. This is the only
    // track measured so far that holds the complete CEFR A1 repair kit:
    // sumimasen, wakarimasen, mou ichido onegai shimasu, mou sukoshi yukkuri
    // itte kudasai, koko, wakarimashita. Russian has two of five moves and no
    // word for sorry; Chinese has one and no word for sorry; Gujarati had none.
    expect(coverage.byCategory["Kaiwa no un-ei - managing the conversation, and repairing it"]!).toEqual({
      enumerated: 8,
      covered: 7,
    });
    // AND THE SAME EMPTY COLUMN AS EVERY OTHER TRACK. Eighth in a row, third
    // outside South Asia. `demo`, `kara`, `node` and the quotative `to` return
    // zero occurrences in kana and in romaji. The best-built track in the
    // corpus — zero gentle-ramp findings of any kind — has Gujarati's joining
    // column, which is what makes the hole structural rather than neglect.
    expect(coverage.byCategory["Setsuzoku - joining two clauses"]!).toEqual({
      enumerated: 8,
      covered: 0,
    });
    // The three scripts, at three depths, which is the reason they are three
    // columns. Averaging 31/46, 2/46 and 3 would report nothing true.
    expect(coverage.byCategory["Hiragana - the first syllabary"]!.covered).toBe(2);
    expect(coverage.byCategory["Katakana - the second syllabary"]!).toEqual({ enumerated: 2, covered: 1 });
    expect(coverage.byCategory["Kanji - the third script, which is not a syllabary at all"]!.covered).toBe(3);
    // Particles are to Japanese what particles are to Mandarin, and the same
    // sentence is true of both tracks: one particle atom exists and it is about
    // spelling, not about the particle's job.
    expect(coverage.byCategory["Joshi - the particles, which do the work case endings do elsewhere"]!).toEqual({
      enumerated: 8,
      covered: 1,
    });
    // Fourteen body words and nine family words, and no word for "I".
    expect(coverage.byCategory["Daimeishi - pronouns, and the fact that Japanese avoids them"]!.covered).toBe(0);
    expect(coverage.byCategory["Karada - the body"]!.covered).toBe(2);
    // UNCHANGED at 66 by HL-C360, and that is the finding rather than an
    // oversight: chapters 14-15 taught ten cardinals and moved NO total,
    // because JA-A1-NUM-01 and JA-A1-NG2-01 were already ticked -- one on a
    // single numeral inside a phrase, the other on the vague half of counting.
    // The tranche deepened two ticks instead of adding one. A coverage total
    // cannot see that, which is why the named pin below exists.
    expect(formatExamCoverage(coverage)).toContain(
      "japanese A1 (partial inventory): 66/179 points covered (37%)",
    );
  }, 60_000);

  // The cardinal point, named rather than left to the aggregate, because the
  // aggregate did not move. Both halves were falsified before this was kept: a
  // fabricated id fails the "probes only atoms that EXIST" test above, and
  // nulling the probe drops the total to 65 and fails the assertion above.
  it("closes JA-A1-NUM-01 on all ten cardinals, not on one inside a phrase", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "japanese");
    const cardinals = inventory.points.find((point) => point.id === "JA-A1-NUM-01");
    expect(cardinals?.probe).toEqual([
      // The ten, in numerical order here and in NO other order in the book: the
      // chapters teach ichi, go, ni, san, yon, then nana, hachi, ku, roku, juu.
      "JA-LEX-ICHI",
      "JA-LEX-NI",
      "JA-LEX-SAN",
      "JA-LEX-YON",
      "JA-LEX-GO",
      "JA-LEX-ROKU",
      "JA-LEX-NANA",
      "JA-LEX-HACHI",
      "JA-LEX-KU",
      "JA-LEX-JUU",
      // What carries the point past ten: a numeral before juu multiplies it and
      // one after it is added, so ten words reach ninety-nine.
      "JA-GRAMMAR-JUU-COMPOUND",
      // The seam the counters will run along, opened at four and held at seven.
      "JA-GRAMMAR-KUN-IN-THE-COUNT",
    ]);
    for (const atom of cardinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // The numeral the old tick rested on is still taught, in the chapter-9
    // phrase the cardinal lesson takes apart.
    expect(taught.has("JA-LEX-ICHIDO")).toBe(true);
    // And the counters are still absent, which is what blocks the ordinals.
    expect(inventory.points.find((point) => point.id === "JA-A1-NUM-02")?.probe).toBeNull();
    expect(inventory.points.find((point) => point.id === "JA-A1-NUM-03")?.probe).toBeNull();
  }, 60_000);
});

// ---------------------------------------------------------------------------
// The first inventory for a track with NO ALPHABET, and the first with a TONE
// column at all.
//
// Two decisions in this file are larger than any point in it, so both are
// asserted here rather than left to a commit message.
//
//   1. SPANISH'S "ALPHABET" POINT IS ANSWERED BY THE STROKE, not by the
//      character. A1-O1-01 asks for the closed set of units a reader learns
//      once and reuses forever. Translating that as "the characters" would ask
//      a beginner's track for tens of thousands and report every Mandarin
//      course that has ever existed as failing; the strokes are the set the
//      question was really about, and the corpus teaches them, opening on `yi`
//      — one horizontal that is both a stroke and a whole character.
//
//   2. PINYIN IS A PRONUNCIATION CLAIM, NOT A SCRIPT CLAIM. The argument and
//      its three pieces of internal evidence are written into the file at
//      ZH-A1-PY-06 so a reader who disagrees can refile those points without
//      re-deriving it. The mechanical consequence is what this block checks:
//      the character column counts characters, so the 53 words the reader
//      knows by ear cannot inflate it.
//
// And one column exists because `exam-levels.json` said so. The chinese caveat
// records that GF0025-2021 defines its levels "across listening, speaking,
// reading, writing, and translation" — a fifth skill the PCIC inventories are
// monolingual by construction and cannot enumerate. `Fanyi` is in the file for
// that reason and comes back 0 of 2, with zero of the 175 lesson files
// declaring `mediation` in `modes`.
// ---------------------------------------------------------------------------
describe("the committed Chinese A1 inventory", () => {
  const inventory = loadExamInventory("chinese", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "chinese");
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

  it("drops exactly one point, and says why that one and not the neighbours", () => {
    // The alphabet, the case distinction and the written accent all LOOK
    // untransferable to a logographic script and all three restate — onto the
    // stroke, onto pinyin, and onto the tone marks. Only A1-O1-06 returns
    // nothing on either side of the script question, and the reason has to name
    // both sides or it is a guess.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    expect(proxy.notTransferred).toHaveLength(1);
    expect(proxy.notTransferred[0]!.spanishPoints).toEqual(["A1-O1-06"]);
    expect(proxy.notTransferred[0]!.why).toMatch(/unicameral/);
    expect(proxy.notTransferred[0]!.why).toMatch(/Beida/);
    // And it names the track that DERIVED the same point, which is what keeps
    // "nothing to superscript" from reading as "superscripts looked foreign".
    expect(proxy.notTransferred[0]!.why).toMatch(/RU-A1-L-09/);
    expect(proxy.note).toMatch(/A1-O1-01, the alphabet, becomes the STROKE/);
  });

  it("writes down the pinyin decision IN THE FILE, with its consequence", () => {
    // The instruction this file was written under: decide explicitly whether
    // pinyin coverage is a script claim or a pronunciation claim, and write
    // down which you chose. A decision recorded only in a commit message is a
    // decision the next author re-makes differently.
    const decision = inventory.points.find((point) => point.id === "ZH-A1-PY-06");
    expect(decision, "the pinyin decision point must exist").toBeDefined();
    expect(decision!.probe, "the decision records a choice, not a lesson").toBeNull();
    expect(decision!.note).toMatch(/THE DECISION: pronunciation/);
    expect(decision!.note).toMatch(/script-closure\.ts/);
    expect(decision!.note).toMatch(/ZH-ORTHO/);
    expect(decision!.note).toMatch(/THE CONSEQUENCE, STATED RATHER THAN HIDDEN/);
    expect(inventory.about).toMatch(/THIS FILE TREATS PINYIN AS PRONUNCIATION/);
    // The mechanical half of the decision: no pinyin point may be probed with a
    // SCRIPT atom, because that would file a pronunciation claim as script.
    const pinyin = inventory.points.filter((point) => point.category.startsWith("Pinyin"));
    expect(pinyin.length).toBeGreaterThanOrEqual(6);
    for (const point of pinyin) {
      for (const atom of point.probe ?? []) {
        expect(atom.startsWith("ZH-SCRIPT-"), `${point.id} probes a script atom`).toBe(false);
      }
    }
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; chineseSpecific?: boolean };
      expect(cast.chineseSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // Tone (5), the particles (2 of 4), measure words (2), aspect (2), the
    // phonetic component, the character census, simplified-against-traditional,
    // handwriting, dictionary lookup, topic-comment order, the compound, the
    // pinyin decision, three repair points and the two mediation points.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/NOT A TRANSCRIPTION OF ANY HSK SYLLABUS/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO HANBAN/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/NO SEARCH WAS RUN, BY INSTRUCTION/);
    // The caveat that DID steer this file, and the column it produced.
    expect(inventory.about).toMatch(/TRANSLATION AND MEDIATION/);
    expect(inventory.about).toMatch(/THAT CAVEAT WAS READ BEFORE ANY POINT WAS WRITTEN/);
    // The number this file must never invent. An HSK character or word count
    // would be the easiest thing in the world to assert and the hardest to
    // defend, given that no search was run.
    expect(inventory.source).toMatch(/NO HSK WORD LIST OR CHARACTER LIST IS CITED ANYWHERE IN THIS FILE/);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: PARTIAL AND DANGLING/);
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

  it("reports the tranche that broke the particle gap and the joining zero", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(191);
    // 70 -> 73: HL-C350's numeral tranche (chapters 20-21) closed ZH-A1-NUM-01,
    // ZH-A1-NUM-02 and ZH-A1-AB-02.
    // 73 -> 98: the grammar tranche of chapters 22-28. TEN CHARACTERS, and five
    // of the twenty-five points cost no character at all.
    expect(coverage.covered).toBe(98);
    expect(coverage.unmapped).toBe(93);
    expect(coverage.partial).toBe(0);

    // THE HEADLINE WHEN THIS FILE WAS WRITTEN, and it was not a vocabulary gap.
    // Mandarin carries almost all of its grammar in a handful of toneless
    // particles, and NOT ONE was taught: de, le, ma, ne, ba, guo and zhe each
    // returned zero occurrences across the 175 lesson files then in the track.
    //
    // 0/4 -> 1/4 -> 3/4. 吗 arrived with the asking chapter; 的 and 了 and 呢
    // arrive here. Four of the seven particles are now taught, and the two that
    // carry the most grammar between them — 的 and 了 — cost eight strokes and
    // two. PART-04 stays open because a CLASS point is not answered by four of
    // seven, and its note now names guo and zhe as what is left.
    expect(coverage.byCategory["Zhuci - the particles"]!).toEqual({ enumerated: 4, covered: 3 });

    // THE JOINING COLUMN COMES OFF ZERO. It had been flat in seven tracks
    // running, and J-08's note had already done the work of saying why and
    // which word was cheapest: every one of the nine joining words needs a
    // character the track does not teach, a character here costs a writing
    // lesson and a reading lesson and a source-verified stroke record and a
    // regeneration of the subset font, and `he` is the cheapest of the nine.
    // This tranche spent that budget. 和 is eight strokes, three of which are
    // 口, and it is a phono-semantic compound whose sounding half IS hé.
    expect(coverage.byCategory["Lianjie - joining two clauses"]!).toEqual({ enumerated: 8, covered: 1 });

    // FOUR COLUMNS CLOSE OUTRIGHT, and two of them were columns of one that no
    // lesson had ever said out loud: Mandarin has no article, and possession is
    // one particle between owner and owned.
    expect(coverage.byCategory["Guanci - the article"]!).toEqual({ enumerated: 1, covered: 1 });
    expect(coverage.byCategory["Lingshu - possession"]!).toEqual({ enumerated: 1, covered: 1 });
    expect(coverage.byCategory["Dongci - the verb, which does not inflect"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    expect(coverage.byCategory["Ti - aspect, which is what Mandarin has instead of tense"]!).toEqual({
      enumerated: 2,
      covered: 2,
    });
    expect(coverage.byCategory["Danju - the simple sentence"]!).toEqual({ enumerated: 4, covered: 4 });

    // The column the proxy has NO point for anywhere, and the corpus's best
    // work: tone is lexical, the five contours, third-tone sandhi taught on the
    // first word in the book, and bu sandhi on the commonest bu there is. What
    // is missing is tone across a phrase, and this tranche did not touch it.
    expect(coverage.byCategory["Shengdiao - tone, for which the proxy has no column at all"]!).toEqual({
      enumerated: 5,
      covered: 4,
    });
    // The fifth skill this track's own alignment names, measured and still
    // empty: no lesson in the track declares `mediation` in `modes`.
    expect(coverage.byCategory["Fanyi - translation and mediation"]!).toEqual({ enumerated: 2, covered: 0 });
    // UNTOUCHED, AND DELIBERATELY. Food and drink is a Spanish point and not one
    // Mandarin word, in a language whose learners eat on day one; punctuation is
    // seven points and not one mark taught in 244 lessons — which is why this
    // tranche writes no 。 and no ，either, and prints a dash at a seam where
    // Chinese writes a comma rather than smuggling one in.
    expect(coverage.byCategory["Yinshi - food and drink"]!).toEqual({ enumerated: 1, covered: 0 });
    expect(coverage.byCategory["Biaodian - punctuation"]!).toEqual({ enumerated: 7, covered: 0 });
    // The numeral work of the previous tranche, unchanged by this one.
    expect(coverage.byCategory["Shuci - numerals and quantity"]!)
      .toEqual({ enumerated: 5, covered: 2 });
    const zhDecimal = inventory.points.find((point) => point.id === "ZH-A1-NUM-02")!;
    expect(zhDecimal.probe).toContain("ZH-GRAMMAR-DECIMAL-TEENS");
    expect(zhDecimal.probe).not.toContain("ZH-LEX-SHISAN");
    expect(formatExamCoverage(coverage)).toContain(
      "chinese A1 (partial inventory): 98/191 points covered (51%)",
    );
  }, 60_000);

  it("closes five points with NO NEW CHARACTER, which is what a character costs here", () => {
    // A character in this track is the expensive unit — a writing lesson, a
    // reading lesson, a source-verified stroke record in data/scripts/chinese.json
    // and a regeneration of the vendored subset font — so the points that need
    // none are worth naming as a set rather than leaving inside a total.
    //
    // Three of the five are things the reader must STOP doing, which is why no
    // lesson had ever said them: an absence leaves no word to teach. The fourth
    // is a join between two things the track already had and never let meet, and
    // the fifth is a word ORDER.
    const free: Record<string, string> = {
      "ZH-A1-V-02": "ZH-GRAMMAR-VERB-INVARIANT-01",
      "ZH-A1-NP-03": "ZH-GRAMMAR-NO-AGREEMENT-01",
      "ZH-A1-VP-03": "ZH-GRAMMAR-NO-COMPLEMENT-AGREEMENT-01",
      "ZH-A1-ART-01": "ZH-GRAMMAR-NO-ARTICLE-01",
      "ZH-A1-ADJ-02": "ZH-LEX-ZHONGGUOREN-01",
      "ZH-A1-S-04": "ZH-GRAMMAR-TOPIC-COMMENT-01",
    };
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "chinese");
    for (const [pointId, atom] of Object.entries(free)) {
      const point = inventory.points.find((candidate) => candidate.id === pointId)!;
      expect(point.probe, pointId).toContain(atom);
      expect(taught.has(atom), atom).toBe(true);
      // None of them is probed with a SCRIPT atom, because none of them needed a
      // character. That is the mechanical form of the claim.
      for (const probed of point.probe ?? []) {
        expect(probed.startsWith("ZH-SCRIPT-"), `${pointId} probes a script atom`).toBe(false);
      }
    }
  }, 60_000);

  it("records the numeral blocker that had already lifted before this tranche looked", () => {
    // ZH-A1-NG5-04's note read "blocked on the numerals", and the numeral
    // tranche of chapters 20-21 had already lifted that block. Only 岁 was
    // missing. Pinned rather than merely fixed, because a note that decays into
    // agreement is indistinguishable from real debt and gets a lesson written
    // for a gap that is no longer there.
    const age = inventory.points.find((point) => point.id === "ZH-A1-NG5-04")!;
    expect(age.probe).toContain("ZH-LEX-SUI-01");
    expect(age.note).toMatch(/THE OLD NOTE'S BLOCKER IS GONE/);
    // And the point it unlocked in turn: name, nationality and age, where the
    // nationality half cost no character either.
    const personal = inventory.points.find((point) => point.id === "ZH-A1-F1-03")!;
    expect(personal.probe).toContain("ZH-LEX-ZHONGGUOREN-01");
    expect(personal.probe).toContain("ZH-GRAMMAR-AGE-NO-VERB-01");
  });

});
