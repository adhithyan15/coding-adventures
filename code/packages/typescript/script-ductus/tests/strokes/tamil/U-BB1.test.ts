import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB1.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ற"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB1 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ற");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "9dcca15de18b84da6e956ea5c3ef3d1c1bcfd614acdfb32f57ff296023830e22",
    );
  });

  it("ற lifts between its three pen-down runs", () => {
    expect(penLifts(letter)).toBe(2);
    expect(letter.strokes).toHaveLength(3);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      2, 1, 2,
    ]);
  });

  it("ற's stroke order traces to Frame 10", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 10.*ற/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });
});
