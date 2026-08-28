import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B86.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஆ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B86 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஆ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "6e7b78e4ae5e8ac4ca420fb7215fd5c4063b7fbfa0942ea0d739ce446e445fd2",
    );
  });

  it("ஆ lifts once, then joins its upright and long-vowel loop", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(letter.strokes[1].segments).toHaveLength(2);
  });

  it("ஆ's stroke order traces to the next row of Frame 4", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*ஆ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });
});
