/**
 * porter.test.ts — Porter stemmer tests.
 *
 * Test inputs lifted from Porter's original 1980 paper examples
 * + the canonical test vectors published at
 * https://tartarus.org/martin/PorterStemmer/voc.txt and the
 * Snowball project's English reference outputs.
 *
 * Only ASCII lowercase input is well-defined for Porter; the
 * stemmer is a documented English-only algorithm.
 */

import { describe, it, expect } from "vitest";
import { porterStem } from "../src/index.js";

describe("porterStem — short words pass unchanged", () => {
  it("empty string", () => expect(porterStem("")).toBe(""));
  it("1-char word", () => expect(porterStem("a")).toBe("a"));
  it("2-char word", () => expect(porterStem("at")).toBe("at"));
});

describe("porterStem — step 1a (plurals)", () => {
  it("caresses → caress", () => expect(porterStem("caresses")).toBe("caress"));
  it("ponies → poni", () => expect(porterStem("ponies")).toBe("poni"));
  it("ties → ti", () => expect(porterStem("ties")).toBe("ti"));
  it("caress → caress (ss stays)", () => expect(porterStem("caress")).toBe("caress"));
  it("cats → cat", () => expect(porterStem("cats")).toBe("cat"));
});

describe("porterStem — step 1b (past participles + -ing)", () => {
  it("feed → feed (eed with m=0 → no strip)", () => expect(porterStem("feed")).toBe("feed"));
  it("agreed → agre", () => expect(porterStem("agreed")).toBe("agre"));
  it("plastered → plaster", () => expect(porterStem("plastered")).toBe("plaster"));
  it("bled → bled (ed with no vowel in stem → no strip)", () => expect(porterStem("bled")).toBe("bled"));
  it("motoring → motor", () => expect(porterStem("motoring")).toBe("motor"));
  it("sing → sing (ing with no vowel in stem → no strip)", () => expect(porterStem("sing")).toBe("sing"));
});

describe("porterStem — step 1b' (post-strip adjustments)", () => {
  it("conflated → conflate (at → ate)", () => expect(porterStem("conflated")).toBe("conflat"));
  it("troubled → trouble (bl → ble)", () => expect(porterStem("troubled")).toBe("troubl"));
  it("sized → size (iz → ize)", () => expect(porterStem("sized")).toBe("size"));
  it("hopping → hop (double consonant stripped)", () => expect(porterStem("hopping")).toBe("hop"));
  it("tanned → tan", () => expect(porterStem("tanned")).toBe("tan"));
  it("falling → fall (LL preserved)", () => expect(porterStem("falling")).toBe("fall"));
  it("hissing → hiss (SS preserved)", () => expect(porterStem("hissing")).toBe("hiss"));
  it("fizzed → fizz (ZZ preserved)", () => expect(porterStem("fizzed")).toBe("fizz"));
  it("failing → fail (m=1 + *o → add e)", () => expect(porterStem("failing")).toBe("fail"));
  it("filing → file (m=1 + *o → add e)", () => expect(porterStem("filing")).toBe("file"));
});

describe("porterStem — step 1c (-y → -i)", () => {
  it("happy → happi", () => expect(porterStem("happy")).toBe("happi"));
  it("sky → sky (no vowel in stem → no strip)", () => expect(porterStem("sky")).toBe("sky"));
});

describe("porterStem — step 2 (-ational, -tional, etc.)", () => {
  it("relational → relate", () => expect(porterStem("relational")).toBe("relat"));
  it("conditional → condition", () => expect(porterStem("conditional")).toBe("condit"));
  it("valenci → valence", () => expect(porterStem("valenci")).toBe("valenc"));
  it("hesitanci → hesitance", () => expect(porterStem("hesitanci")).toBe("hesit"));
  it("digitizer → digitize", () => expect(porterStem("digitizer")).toBe("digit"));
  it("conformabli → conformable", () => expect(porterStem("conformabli")).toBe("conform"));
  it("radicalli → radical", () => expect(porterStem("radicalli")).toBe("radic"));
  it("differentli → different", () => expect(porterStem("differentli")).toBe("differ"));
  it("vileli → vile", () => expect(porterStem("vileli")).toBe("vile"));
  it("analogousli → analogous", () => expect(porterStem("analogousli")).toBe("analog"));
  it("vietnamization → vietnamize", () => expect(porterStem("vietnamization")).toBe("vietnam"));
  it("predication → predicate", () => expect(porterStem("predication")).toBe("predic"));
  it("operator → operate", () => expect(porterStem("operator")).toBe("oper"));
  it("feudalism → feudal", () => expect(porterStem("feudalism")).toBe("feudal"));
  it("decisiveness → decisive", () => expect(porterStem("decisiveness")).toBe("decis"));
  it("hopefulness → hopeful", () => expect(porterStem("hopefulness")).toBe("hope"));
  it("callousness → callous", () => expect(porterStem("callousness")).toBe("callous"));
  it("formaliti → formal", () => expect(porterStem("formaliti")).toBe("formal"));
  it("sensitiviti → sensitive", () => expect(porterStem("sensitiviti")).toBe("sensit"));
  it("sensibiliti → sensible", () => expect(porterStem("sensibiliti")).toBe("sensibl"));
});

describe("porterStem — step 3 (-icate, -ative, -alize)", () => {
  it("triplicate → triplic", () => expect(porterStem("triplicate")).toBe("triplic"));
  it("formative → form", () => expect(porterStem("formative")).toBe("form"));
  it("formalize → formal", () => expect(porterStem("formalize")).toBe("formal"));
  it("electriciti → electric", () => expect(porterStem("electriciti")).toBe("electr"));
  it("electrical → electric", () => expect(porterStem("electrical")).toBe("electr"));
  it("hopeful → hope", () => expect(porterStem("hopeful")).toBe("hope"));
  it("goodness → good", () => expect(porterStem("goodness")).toBe("good"));
});

describe("porterStem — step 4 (-al, -ance, -ence, etc.)", () => {
  it("revival → reviv", () => expect(porterStem("revival")).toBe("reviv"));
  it("allowance → allow", () => expect(porterStem("allowance")).toBe("allow"));
  it("inference → infer", () => expect(porterStem("inference")).toBe("infer"));
  it("airliner → airlin", () => expect(porterStem("airliner")).toBe("airlin"));
  it("gyroscopic → gyroscop", () => expect(porterStem("gyroscopic")).toBe("gyroscop"));
  it("adjustable → adjust", () => expect(porterStem("adjustable")).toBe("adjust"));
  it("defensible → defens", () => expect(porterStem("defensible")).toBe("defens"));
  it("irritant → irrit", () => expect(porterStem("irritant")).toBe("irrit"));
  it("replacement → replac", () => expect(porterStem("replacement")).toBe("replac"));
  it("adjustment → adjust", () => expect(porterStem("adjustment")).toBe("adjust"));
  it("dependent → depend", () => expect(porterStem("dependent")).toBe("depend"));
  it("homologous → homolog", () => expect(porterStem("homologous")).toBe("homolog"));
  it("communism → commun", () => expect(porterStem("communism")).toBe("commun"));
  it("activate → activ", () => expect(porterStem("activate")).toBe("activ"));
  it("angulariti → angular", () => expect(porterStem("angulariti")).toBe("angular"));
  it("homologous (second variant) → homolog", () => expect(porterStem("homologous")).toBe("homolog"));
  it("effective → effect", () => expect(porterStem("effective")).toBe("effect"));
  it("bowdlerize → bowdler", () => expect(porterStem("bowdlerize")).toBe("bowdler"));
  it("adoption → adopt (ion after -t)", () => expect(porterStem("adoption")).toBe("adopt"));
  it("decision → decis (ion after -s)", () => expect(porterStem("decision")).toBe("decis"));
});

describe("porterStem — step 5a (-e cleanup)", () => {
  it("probate → probat", () => expect(porterStem("probate")).toBe("probat"));
  it("rate → rate", () => expect(porterStem("rate")).toBe("rate"));
  it("cease → ceas", () => expect(porterStem("cease")).toBe("ceas"));
});

describe("porterStem — step 5b (-ll → -l)", () => {
  it("controll → control", () => expect(porterStem("controll")).toBe("control"));
  it("roll → roll (m=0 → no strip)", () => expect(porterStem("roll")).toBe("roll"));
});

describe("porterStem — common docs words (sanity checks)", () => {
  it("running → run", () => expect(porterStem("running")).toBe("run"));
  it("ran → ran (no rule strips this — Porter is morphology-only)", () => expect(porterStem("ran")).toBe("ran"));
  it("happily → happili", () => expect(porterStem("happily")).toBe("happili"));
  it("indexing → index", () => expect(porterStem("indexing")).toBe("index"));
  it("indexed → index", () => expect(porterStem("indexed")).toBe("index"));
  it("indexes → index", () => expect(porterStem("indexes")).toBe("index"));
});

describe("porterStem — stack-overflow defence (MEDIUM finding fixed)", () => {
  // The original recursive isConsonantAt overflowed V8's stack
  // on inputs like "y".repeat(10000) — a single very-long
  // y-run in untrusted input would crash the indexer.  The
  // iterative implementation handles arbitrary lengths.
  it("10,000 'y' characters does not overflow stack", () => {
    const huge = "y".repeat(10000);
    expect(() => porterStem(huge)).not.toThrow();
  });
  it("50,000 'y' characters does not overflow stack", () => {
    const huge = "y".repeat(50000);
    expect(() => porterStem(huge)).not.toThrow();
  });
  it("alternating yyyy still classifies correctly (behaviour-preserved)", () => {
    // For a 4-character "yyyy" word: position 0 'y' = consonant,
    // position 1 'y' = vowel, position 2 'y' = consonant,
    // position 3 'y' = vowel.  containsVowel("yyyy") is therefore
    // true (positions 1 and 3).  The stemmer applies step1c
    // (y → i) when the stem has a vowel.  For "yyyy":
    // step1a / step1b are no-ops.  step1c sees ending "y", stem
    // "yyy" — containsVowel("yyy") is true (position 1).  So
    // "yyyy" → "yyyi".
    expect(porterStem("yyyy")).toBe("yyyi");
  });
});

describe("porterStem — determinism", () => {
  it("same input → identical output", () => {
    for (const w of ["running", "happiness", "national", "consign"]) {
      expect(porterStem(w)).toBe(porterStem(w));
    }
  });
});
