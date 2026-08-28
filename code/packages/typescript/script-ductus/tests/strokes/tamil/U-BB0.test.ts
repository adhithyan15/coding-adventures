import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB0.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ர"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB0 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ர");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "71f12988b4242b75eb0aa939e74f6c98172a98e0ef6472cd532ea2fdb2e96bd4",
    );
  });

  it("ர writes its uprights and cap before joining the angular tail", () => {
    expect(penLifts(letter)).toBe(2);
    expect(letter.strokes).toHaveLength(3);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      1, 1, 2,
    ]);
    expect(letter.strokes[2].segments.map((segment) => segment.label)).toEqual([
      "down the central upright",
      "around the short angular tail",
    ]);
  });

  it("ர's three-run order traces to Frame 3 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*ர/i);
    expect(src.variation).toMatch(
      /three-movement ஈ frame.*angular short fourth movement.*varies by school.*three-run order.*Noto Sans Tamil/i,
    );
  });
});
