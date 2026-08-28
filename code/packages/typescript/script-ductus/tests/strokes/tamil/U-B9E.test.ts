import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B9E.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஞ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B9E stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஞ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "751cd6e3691ad02b14a608e2cbb022d89112ffb363f75c048dd056cda395d923",
    );
  });

  it("Tamil ஞ groups Frame 8's eight movements into four runs", () => {
    expect(penLifts(letter)).toBe(3);
    expect(letter.strokes).toHaveLength(4);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 2, 3,
    ]);
    expect(letter.source.citation).toMatch(
      /Tamil Script Learners Manual.*Frame 8.*ஞ.*p\. 194/i,
    );
    expect(letter.source.variation).toMatch(
      /1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*four-run order.*Noto Sans Tamil/i,
    );
  });

  it("ஞ's four-run order traces to Appendix I Frame 8", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 8.*ஞ.*p\. 194/i);
    expect(src.variation).toMatch(
      /eight movements.*1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
  });
});
