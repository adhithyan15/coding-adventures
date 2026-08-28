import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../../src/scriptdata";
import { DUCTUS, penLifts, penPath } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BAE.ts";
import {
  registerStrokeHonestyTests,
  distanceToPath,
  fontForDuctus,
  fractionOnInk,
  inkPoints,
  makeInInk,
} from "../../support/stroke-honesty";

const letter = DUCTUS["ம"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BAE stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ம");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "417841305336ba5887f9ca7bbf633d96eae2d2481f4a171a9abbdd9c05d845df",
    );
  });

  it("uses the verified Tamil font", () => {
    expect(verifiedLetterFont("ம", letter.source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
  });

  it("CONTROL: a Tamil stroke pushed off its glyph fails the on-ink check", () => {
    const reference = letter;
    const inInk = makeInInk(fontForDuctus(reference).glyphFor("ம")!.contours);
    const shifted = penPath(reference.strokes[0]).map((point) => ({
      x: point.x + 400,
      y: point.y,
    }));
    expect(fractionOnInk(shifted, inInk)).toBeLessThan(0.9);
  });

  it("CONTROL: dropping the Tamil arch leaves much of the glyph untraced", () => {
    const reference = letter;
    const ink = fontForDuctus(reference).glyphFor("ம")!;
    const points = inkPoints(ink.contours);
    const onlyFirstTwo = {
      segments: reference.strokes[0].segments.slice(0, 2),
    };
    const path = penPath(onlyFirstTwo);
    const strayed = points.filter(([x, y]) => distanceToPath(x, y, path) > 130);
    expect(strayed.length / points.length).toBeGreaterThan(0.1);
  });

  it("ம is written without lifting the pen (one stroke)", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
  });

  it("ம's stroke order traces to the UT Austin primer, and records Tamil's variation", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I|Frame 1/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
  });
});
