import { describe, it, expect } from "vitest";
import {
  isSyllabary,
  consonantGroups,
  unlockedConsonantCount,
  unlockedLetterIndices,
  ROW_MASTERY_BOX,
} from "../src/syllabary";
import { SCRIPTS } from "../src/data";

// A tiny fixture: 3 consonants × 3 vowels. A base syllable has ONE component (the
// bare consonant); a signed one has two — the boundary consonantGroups() reads.
function syl(base: boolean) {
  return { role: "syllable", components: base ? ["k"] : ["k", "i"] };
}
const LETTERS = [
  syl(true), syl(false), syl(false), // consonant 1: ka ki ku  (indices 0,1,2)
  syl(true), syl(false), syl(false), // consonant 2: ga gi gu  (3,4,5)
  syl(true), syl(false), syl(false), // consonant 3: ca ci cu  (6,7,8)
];
const boxes = (arr: number[]) => arr.map((box) => ({ box }));
const mastered = boxes(Array(9).fill(ROW_MASTERY_BOX));

describe("isSyllabary", () => {
  it("true only when every letter is a syllable", () => {
    expect(isSyllabary(LETTERS)).toBe(true);
    expect(isSyllabary([{ role: "consonant" }, { role: "syllable" }])).toBe(false);
    expect(isSyllabary([])).toBe(false);
  });
});

describe("consonantGroups", () => {
  it("segments a consonant-major syllabary at each bare consonant", () => {
    expect(consonantGroups(LETTERS)).toEqual([[0, 1, 2], [3, 4, 5], [6, 7, 8]]);
  });
});

describe("unlockedConsonantCount — the slow unlock", () => {
  const groups = consonantGroups(LETTERS);

  it("starts at 1 consonant (only ka's row) on a fresh, unmastered state", () => {
    expect(unlockedConsonantCount(groups, boxes([0, 0, 0, 0, 0, 0, 0, 0, 0]))).toBe(1);
  });

  it("CONTROL: the 2nd consonant stays LOCKED until the 1st row is fully mastered", () => {
    // Row 1 partially mastered (ku still at box 0) → still only 1 unlocked.
    const partial = boxes([ROW_MASTERY_BOX, ROW_MASTERY_BOX, 0, 0, 0, 0, 0, 0, 0]);
    expect(unlockedConsonantCount(groups, partial)).toBe(1);
    // Master the whole first row → the 2nd consonant unlocks.
    const rowOneDone = boxes([ROW_MASTERY_BOX, ROW_MASTERY_BOX, ROW_MASTERY_BOX, 0, 0, 0, 0, 0, 0]);
    expect(unlockedConsonantCount(groups, rowOneDone)).toBe(2);
  });

  it("a gap holds the rest locked — mastering row 3 without row 2 unlocks nothing extra", () => {
    const gap = boxes([ROW_MASTERY_BOX, ROW_MASTERY_BOX, ROW_MASTERY_BOX, 0, 0, 0, ROW_MASTERY_BOX, ROW_MASTERY_BOX, ROW_MASTERY_BOX]);
    expect(unlockedConsonantCount(groups, gap)).toBe(2); // row 2 not done → capped at 2
  });

  it("all rows mastered unlocks every consonant, capped at the count", () => {
    expect(unlockedConsonantCount(groups, mastered)).toBe(3);
  });
});

describe("unlockedLetterIndices", () => {
  const groups = consonantGroups(LETTERS);
  it("returns exactly the syllables of the unlocked consonants", () => {
    expect(unlockedLetterIndices(groups, 1)).toEqual([0, 1, 2]);
    expect(unlockedLetterIndices(groups, 2)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(unlockedLetterIndices(groups, 99)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]); // clamped
    expect(unlockedLetterIndices(groups, 0)).toEqual([0, 1, 2]); // floored to 1
  });
});

describe("against the real generated Telugu syllabary", () => {
  const telugu = SCRIPTS.find((s) => s.script === "telugu")!;

  it("is recognised as a syllabary and starts with a single-row unlock", () => {
    expect(isSyllabary(telugu.letters)).toBe(true);
    const groups = consonantGroups(telugu.letters);
    expect(groups.length).toBe(35); // 35 consonants
    expect(groups[0]!.length).toBe(10); // ka's row = 10 core vowels
    // Fresh learner: only the first consonant's 10 syllables are drillable.
    const fresh = telugu.letters.map(() => ({ box: 0 }));
    expect(unlockedConsonantCount(groups, fresh)).toBe(1);
    expect(unlockedLetterIndices(groups, 1)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });
});
