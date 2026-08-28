import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B87.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["இ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B87 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("இ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "ff4fba5e23329c754b9f9f9b32264893516c6d0e0d71543805550cedee0f3ac5",
    );
  });

  it("இ lifts once before its joined outer climb and arch", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(letter.strokes[0].segments).toHaveLength(5);
    expect(letter.strokes[1].segments).toHaveLength(2);
  });

  it("இ's stroke order traces to Frame 4's third row", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*இ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });
});
