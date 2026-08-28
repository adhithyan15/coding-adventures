import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BA8.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ந"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BA8 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ந");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "0aedc429ceb51609297a5490a7345a02a08ef30dc6c7f3841acd0b8f7dfae2db",
    );
  });

  it("ந groups Frame 5's six movements into three pen-down runs", () => {
    expect(penLifts(letter)).toBe(2);
    expect(letter.strokes).toHaveLength(3);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 2, 2,
    ]);
  });

  it("ந's three-run stroke order traces to Frame 5's first row", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 5.*ந.*p\. 193/);
    expect(src.variation).toMatch(
      /Module 5.*dental nasal.*six movements.*three pen-down runs.*1.?2.*3.?4.*5.?6/i,
    );
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/varies|one attested/i);
  });
});
