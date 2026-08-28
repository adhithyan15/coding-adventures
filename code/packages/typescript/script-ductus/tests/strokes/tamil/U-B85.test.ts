import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B85.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["அ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B85 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("அ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "bb6e8911132b54e3ba34ed16de8a7b345e8589146dc4be24a1e05e6f4211da81",
    );
  });

  it("அ lifts once before its separate right upright (two strokes)", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
  });

  it("அ's stroke order traces to Frame 4 of the same primer", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*அ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });
});
