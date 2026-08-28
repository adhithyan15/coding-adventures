import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B95.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["க"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B95 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("க");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "45a58320c97ce0b49338ce03dc23a8eae84e2d2de5e8d4dbb53a186945e230c7",
    );
  });

  it("க lifts between its upper frame and two lower bowls", () => {
    expect(penLifts(letter)).toBe(2);
    expect(letter.strokes).toHaveLength(3);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 2, 1,
    ]);
  });

  it("க's stroke order traces to Frame 3's final row", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*க/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });
});
