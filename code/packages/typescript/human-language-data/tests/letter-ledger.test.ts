/**
 * letter-ledger.test.ts — HL11 section 4, the order a reader meets the letters.
 *
 * Every fixture here builds its glyphs from Unicode code points rather than
 * typing them, the same discipline `propose_letter_ledger.py` follows. A
 * maintainer who cannot read Tamil can still see what each test asserts, and a
 * fixture cannot drift into a lookalike character from another script — which is
 * exactly the failure the `ledger-foreign-glyph` check exists to catch, and
 * would be invisible if the test itself were vulnerable to it.
 */

import { describe, it, expect } from "vitest";
import {
  validateLetterLedger,
  summarizeLetterLedger,
  type LetterLedger,
  type LedgerLetter,
} from "../src/letter-ledger.js";
import { loadEverything, loadChapterPolicy } from "../src/loader.js";
import type { ParsedLesson } from "../src/parse.js";

// Tamil letters and marks, by code point. TAMIL LETTER KA, MA, NA, NNA, NNNA,
// RRA; TAMIL SIGN VIRAMA (the puḷḷi); TAMIL VOWEL SIGN AA.
const KA = "க";
const MA = "ம";
const NA = "ந";
const NNA = "ண";
const NNNA = "ன";
const RRA = "ற";
const VIRAMA = "்";
const SIGN_AA = "ா";
// DEVANAGARI LETTER KA, for the wrong-script test.
const DEV_KA = "क";

function codePointOf(glyph: string): string {
  return `U+${(glyph.codePointAt(0) ?? 0).toString(16).toUpperCase().padStart(4, "0")}`;
}

function letter(position: number, glyph: string, over: Partial<LedgerLetter> = {}): LedgerLetter {
  const isMark = glyph === VIRAMA || glyph === SIGN_AA;
  return {
    position,
    glyph,
    codePoint: codePointOf(glyph),
    unicodeName: `TAMIL TEST ${position}`,
    kind: isMark ? "vowel-sign" : "letter",
    family: null,
    familySource: null,
    unlocks: [],
    ...over,
  };
}

function ledgerOf(letters: LedgerLetter[]): LetterLedger {
  return {
    script: "tamil",
    version: 1,
    note: "fixture",
    tracks: ["tamil"],
    openingLessons: 40,
    openingWords: 10,
    letters,
  };
}

/** The corpus this ledger claims to describe. Only ids and language matter. */
function corpus(ids: string[], language = "tamil"): ParsedLesson[] {
  return ids.map((id) => ({ language, frontmatter: { id } }) as unknown as ParsedLesson);
}

const codes = (issues: { code: string }[]) => issues.map((i) => i.code);

describe("positions", () => {
  it("accepts a contiguous ledger", () => {
    const issues = validateLetterLedger(
      ledgerOf([letter(1, KA), letter(2, MA)]), corpus([]));
    expect(issues).toEqual([]);
  });

  it("rejects a position that does not match its index", () => {
    const issues = validateLetterLedger(
      ledgerOf([letter(1, KA), letter(3, MA)]), corpus([]));
    expect(codes(issues)).toContain("ledger-position-out-of-order");
  });

  it("rejects a glyph taught twice", () => {
    const issues = validateLetterLedger(
      ledgerOf([letter(1, KA), letter(2, KA)]), corpus([]));
    expect(codes(issues)).toContain("ledger-duplicate-glyph");
  });
});

describe("the glyphs belong to the script the ledger names", () => {
  it("rejects a letter from another script", () => {
    // Devanagari KA in a Tamil ledger. This is the failure that is hardest to
    // see by eye and easiest to make by copy-paste.
    const issues = validateLetterLedger(
      ledgerOf([letter(1, DEV_KA)]), corpus([]));
    expect(codes(issues)).toContain("ledger-foreign-glyph");
  });

  it("rejects a combining mark declared as a letter", () => {
    const bad = letter(1, KA);
    const issues = validateLetterLedger(
      ledgerOf([bad, { ...letter(2, VIRAMA), kind: "letter" }]), corpus([]));
    expect(codes(issues)).toContain("ledger-kind-mismatch");
  });

  it("rejects a base character declared as a vowel sign", () => {
    const issues = validateLetterLedger(
      ledgerOf([{ ...letter(1, KA), kind: "vowel-sign" }]), corpus([]));
    expect(codes(issues)).toContain("ledger-kind-mismatch");
  });

  it("rejects a ledger naming a script it has no pattern for", () => {
    const l = { ...ledgerOf([letter(1, KA)]), script: "elvish" };
    expect(codes(validateLetterLedger(l, corpus([])))).toContain("ledger-unknown-script");
  });
});

describe("a vowel sign cannot arrive before a base letter", () => {
  it("rejects a mark in first position", () => {
    // These are abugidas: a mark MODIFIES a letter, so a ledger that opens on
    // one describes a lesson that cannot be written down.
    const issues = validateLetterLedger(
      ledgerOf([letter(1, VIRAMA), letter(2, KA)]), corpus([]));
    expect(codes(issues)).toContain("ledger-mark-before-letter");
  });

  it("accepts the same mark once a letter precedes it", () => {
    const issues = validateLetterLedger(
      ledgerOf([letter(1, KA), letter(2, VIRAMA)]), corpus([]));
    expect(codes(issues)).not.toContain("ledger-mark-before-letter");
  });
});

describe("families travel together", () => {
  const family = NNA + NNNA + NA + RRA;
  const source = "tamil.json notes: the flat top-bar family";

  it("accepts a contiguous family", () => {
    const letters = [NNA, NNNA, NA, RRA].map((g, i) =>
      letter(i + 1, g, { family, familySource: source }));
    expect(codes(validateLetterLedger(ledgerOf(letters), corpus([])))).toEqual([]);
  });

  it("reports a family split across the ledger", () => {
    // Splitting letters that share a shape trades a reading ramp for a writing
    // confusion, which is why payoff does not get to reorder them freely.
    const letters = [
      letter(1, NNA, { family, familySource: source }),
      letter(2, KA),
      letter(3, NNNA, { family, familySource: source }),
    ];
    expect(codes(validateLetterLedger(ledgerOf(letters), corpus([]))))
      .toContain("ledger-family-split");
  });

  it("warns when a family claim carries no source", () => {
    const letters = [
      letter(1, NNA, { family }),
      letter(2, NNNA, { family }),
    ];
    expect(codes(validateLetterLedger(ledgerOf(letters), corpus([]))))
      .toContain("ledger-family-unsourced");
  });
});

describe("claimed unlocks must come from real lessons", () => {
  const withUnlock = (lesson: string) =>
    ledgerOf([letter(1, KA, {
      unlocks: [{ word: KA, romanization: "ka", lesson }],
    })]);

  it("accepts an unlock naming a lesson that exists", () => {
    const issues = validateLetterLedger(
      withUnlock("TA-C01-vanakkam"), corpus(["TA-C01-vanakkam"]));
    expect(codes(issues)).toEqual([]);
  });

  it("rejects an unlock naming a lesson that does not", () => {
    // This is the drift check: the ledger keeps asserting a payoff long after
    // the lesson delivering it was renamed, and the claim quietly becomes
    // fiction that nothing else would notice.
    const issues = validateLetterLedger(
      withUnlock("TA-C01-renamed-away"), corpus(["TA-C01-vanakkam"]));
    expect(codes(issues)).toContain("ledger-unlock-missing-lesson");
  });

  it("rejects an unlock naming a lesson from another track", () => {
    // The corpus has to contain a lesson of THIS ledger's track, or the check
    // below skips itself by design. So both are present, and only the
    // cross-track claim is wrong.
    const mixed = [
      ...corpus(["TA-C01-vanakkam"], "tamil"),
      ...corpus(["HI-C01-namaste"], "hindi"),
    ];
    const issues = validateLetterLedger(withUnlock("HI-C01-namaste"), mixed);
    expect(codes(issues)).toContain("ledger-unlock-missing-lesson");
  });

  it("says so, loudly, when the check could not run at all", () => {
    // A partial corpus must not manufacture failures: "I cannot see the
    // lessons" and "the lessons are not there" are different statements. But it
    // must not be SILENT either -- the ledger supplies its own `tracks`, so one
    // renamed track name would make the only check for fictional unlock claims
    // vanish while the report still read zero.
    const issues = validateLetterLedger(withUnlock("TA-C01-anything"), corpus([]));
    expect(codes(issues)).toEqual(["ledger-unlocks-unverified"]);
    expect(issues[0]?.severity).toBe("warning");
  });

  it("stays quiet when there is nothing to verify in the first place", () => {
    const issues = validateLetterLedger(ledgerOf([letter(1, KA)]), corpus([]));
    expect(codes(issues)).toEqual([]);
  });
});

describe("unspent letters", () => {
  const unlocking = (position: number) =>
    letter(position, [KA, MA, NA, NNA, NNNA, RRA][position % 6] ?? KA, {
      unlocks: [{ word: "x", romanization: "x", lesson: "L" }],
    });

  it("reports a letter that unlocks nothing for a whole window", () => {
    const letters = [
      letter(1, KA),
      ...[2, 3, 4].map((p) => letter(p, [MA, NA, NNA][p - 2] ?? MA)),
      letter(5, NNNA),
      letter(6, RRA),
      { ...unlocking(7), position: 7, glyph: "ல" },
    ];
    const issues = validateLetterLedger(
      ledgerOf(letters), corpus(["L"]), { unspentWindow: 3 });
    expect(codes(issues)).toContain("ledger-unspent-letter");
  });

  it("does not judge a letter whose window runs past the end of the ledger", () => {
    // A letter in the last few positions has not been given its chance yet;
    // reporting it would be an artifact of where the list stops, not a fact
    // about the ramp.
    const letters = [letter(1, KA), letter(2, MA)];
    const issues = validateLetterLedger(
      ledgerOf(letters), corpus([]), { unspentWindow: 6 });
    expect(codes(issues)).not.toContain("ledger-unspent-letter");
  });
});

describe("summary", () => {
  it("names the first word that becomes writable, not a letter count", () => {
    const letters = [
      letter(1, KA),
      letter(2, MA, { unlocks: [{ word: KA + MA, romanization: "kama", lesson: "L" }] }),
    ];
    const s = summarizeLetterLedger(ledgerOf(letters));
    expect(s.firstWritablePosition).toBe(2);
    expect(s.firstWritableWord).toBe(KA + MA);
  });

  it("reports a cumulative curve, not a per-position count", () => {
    const letters = Array.from({ length: 20 }, (_, i) =>
      letter(i + 1, String.fromCodePoint(0x0b95 + i), {
        unlocks: i < 10 ? [{ word: "w" + i, romanization: "", lesson: "L" }] : [],
      }));
    const s = summarizeLetterLedger(ledgerOf(letters));
    expect(s.writableAfter.find((w) => w.position === 8)?.words).toBe(8);
    expect(s.writableAfter.find((w) => w.position === 16)?.words).toBe(10);
  });

  it("reports nulls for a ledger that unlocks nothing at all", () => {
    const s = summarizeLetterLedger(ledgerOf([letter(1, KA)]));
    expect(s.firstWritablePosition).toBeNull();
    expect(s.firstWritableWord).toBeNull();
  });
});

// --- The real ledgers --------------------------------------------------------

describe("the committed ledgers against the real corpus", () => {
  const { lessons, letterLedgers } = loadEverything();
  const policy = loadChapterPolicy();

  it("loads one ledger per script that has one", () => {
    expect(letterLedgers.length).toBeGreaterThanOrEqual(5);
    expect(letterLedgers.map((l) => l.script).sort()).toEqual(
      ["devanagari", "kannada", "malayalam", "tamil", "telugu"]);
  });

  it("does not let a ledger masquerade as a script inventory", () => {
    // Both files live in data/scripts and both carry the same `script` key, so
    // reading them into one map would have had one silently overwrite the other.
    const { scripts } = loadEverything();
    expect(scripts["tamil"]?.letters?.length ?? 0).toBeGreaterThan(0);
    expect((scripts["tamil"] as unknown as LetterLedger).letters[0]).not.toHaveProperty(
      "position");
  });

  for (const script of ["tamil", "telugu", "kannada", "malayalam", "devanagari"]) {
    it(`${script}'s ledger holds against the corpus`, () => {
      const ledger = letterLedgers.find((l) => l.script === script);
      expect(ledger, `${script} ledger`).toBeDefined();
      const issues = validateLetterLedger(ledger!, lessons, {
        unspentWindow: policy.letterLedgerUnspentWindow ?? 6,
      });
      expect(issues.map((i) => `${i.code}: ${i.message}`)).toEqual([]);
    });
  }

  it("every ledger makes a real word writable inside its first eight positions", () => {
    // HL11's promise, measured rather than asserted: the drizzle has to pay off
    // early or it is an alphabet course with extra steps.
    for (const ledger of letterLedgers) {
      const s = summarizeLetterLedger(ledger);
      expect(s.firstWritablePosition, `${ledger.script}`).not.toBeNull();
      expect(s.firstWritablePosition!, `${ledger.script}`).toBeLessThanOrEqual(8);
    }
  });

  it("every ledger reaches a quarter of its opening vocabulary within 24 positions", () => {
    for (const ledger of letterLedgers) {
      const s = summarizeLetterLedger(ledger);
      const at24 = s.writableAfter.find((w) => w.position === 24)?.words ?? 0;
      expect(at24 / s.openingWords, `${ledger.script}`).toBeGreaterThan(0.25);
    }
  });
});

// --- Rows must be auditable by someone who cannot read the script -----------
//
// The ledger's whole claim to reviewability is that a maintainer can check a
// row without trusting the rendered glyph. A rendered glyph is not an audit
// surface: it can be a lookalike from another script, and it can carry code
// points that render as nothing at all.

describe("a row is pinned numerically, not by how it looks", () => {
  it("rejects a glyph carrying extra code points", () => {
    // The two Unicode checks are satisfied by different parts of a longer
    // string: the script test is unanchored so any in-script code point passes
    // it, and the combining test is anchored so it only ever sees the first.
    // A row could declare itself TAMIL LETTER KA while carrying Latin text.
    const smuggled = { ...letter(1, KA), glyph: KA + "evil@example.com" };
    expect(codes(validateLetterLedger(ledgerOf([smuggled]), corpus([]))))
      .toContain("ledger-glyph-not-one-code-point");
  });

  it("rejects a glyph carrying an invisible bidi override", () => {
    const smuggled = { ...letter(1, KA), glyph: KA + "‮" };
    expect(codes(validateLetterLedger(ledgerOf([smuggled]), corpus([]))))
      .toContain("ledger-glyph-not-one-code-point");
  });

  it("rejects a code point that does not match the glyph", () => {
    const wrong = { ...letter(1, KA), codePoint: "U+0BAE" };
    expect(codes(validateLetterLedger(ledgerOf([wrong]), corpus([]))))
      .toContain("ledger-code-point-mismatch");
  });

  it("rejects a name from the wrong script", () => {
    const wrong = { ...letter(1, KA), unicodeName: "DEVANAGARI LETTER KA" };
    expect(codes(validateLetterLedger(ledgerOf([wrong]), corpus([]))))
      .toContain("ledger-name-wrong-script");
  });

  it("does not turn an inherited property name into a crash", () => {
    // `SCRIPT_PATTERN` is an object literal, so a bare lookup on "constructor"
    // returns a function, sails past the guard, and throws on `.test` -- turning
    // an intended report into a dead validation run.
    for (const script of ["constructor", "toString", "__proto__", "hasOwnProperty"]) {
      const l = { ...ledgerOf([letter(1, KA)]), script };
      expect(() => validateLetterLedger(l, corpus([])), script).not.toThrow();
      expect(codes(validateLetterLedger(l, corpus([]))), script)
        .toContain("ledger-unknown-script");
    }
  });
});
