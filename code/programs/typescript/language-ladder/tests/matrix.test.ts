import { describe, it, expect } from "vitest";
import { buildSyllableMatrix } from "../src/matrix";
import { SCRIPTS } from "../src/data";

// A tiny rectangular fixture: 2 consonants × 3 vowels. A base syllable has ONE
// component (the bare consonant, inherent "a"); a signed one has two.
function base(sound: string) {
  return { sound, glyph: sound, role: "syllable", inherentVowel: "a", components: ["c"] };
}
function signed(sound: string) {
  return { sound, glyph: sound, role: "syllable", inherentVowel: "a", components: ["c", "v"] };
}
const RECT = [
  base("ka"), signed("ki"), signed("ku"), // consonant 1
  base("ga"), signed("gi"), signed("gu"), // consonant 2
];

describe("buildSyllableMatrix", () => {
  it("lays a rectangular syllabary out as consonant rows × vowel columns", () => {
    const m = buildSyllableMatrix(RECT)!;
    expect(m).not.toBeNull();
    // Columns read the vowels off the first consonant's row: ka→a, ki→i, ku→u.
    expect(m.vowels).toEqual(["a", "i", "u"]);
    // Rows are labelled by the consonant's inherent-"a" form, cells carry the
    // flat-list index so the UI can select the syllable.
    expect(m.rows.map((r) => r.label)).toEqual(["ka", "ga"]);
    expect(m.rows[0]!.cells.map((c) => c.sound)).toEqual(["ka", "ki", "ku"]);
    expect(m.rows[0]!.cells.map((c) => c.index)).toEqual([0, 1, 2]);
    expect(m.rows[1]!.cells.map((c) => c.index)).toEqual([3, 4, 5]);
  });

  it("CONTROL: a ragged syllabary (a consonant missing a vowel) yields NO matrix", () => {
    // Consonant 2 has only 2 syllables where consonant 1 has 3 — misaligned, so
    // a cell could sit under the wrong vowel header. Refuse to build the grid.
    const ragged = [base("ka"), signed("ki"), signed("ku"), base("ga"), signed("gi")];
    expect(buildSyllableMatrix(ragged)).toBeNull();
  });

  it("returns null on an empty list", () => {
    expect(buildSyllableMatrix([])).toBeNull();
  });
});

describe("against the real generated Telugu syllabary", () => {
  const telugu = SCRIPTS.find((s) => s.script === "telugu")!;

  it("is a full 35-consonant × 13-vowel grid with grounded vowel headers", () => {
    const m = buildSyllableMatrix(telugu.letters as never)!;
    expect(m).not.toBeNull();
    expect(m.rows.length).toBe(35);
    expect(m.vowels).toEqual(["a", "ā", "i", "ī", "u", "ū", "e", "ē", "o", "ō", "ai", "au", "r̥"]);
    // First row is the ka-series; every row spans all 13 vowels.
    expect(m.rows[0]!.label).toBe("ka");
    expect(m.rows.every((r) => r.cells.length === 13)).toBe(true);
    // The vocalic-R column header is ISO-15919 r̥ = r + U+0325 (ring below),
    // NOT the IAST dot-below ṛ (U+1E5B).
    expect([...m.vowels[12]!].map((c) => c.codePointAt(0))).toEqual([0x72, 0x325]);
  });
});
