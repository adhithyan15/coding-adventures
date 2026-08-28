import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB3.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ள"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB3 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ள");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "129e7be9acdfc96ba0fee22f44e8d251ab85b8455ec83efcc977b516314bc44b",
    );
  });

  it("ள lifts between its three pen-down runs", () => {
    expect(penLifts(letter)).toBe(2);
    expect(letter.strokes).toHaveLength(3);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 2, 1,
    ]);
  });

  it("ள's three-run order traces to Frame 12", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 12.*ள.*p\. 195/);
    expect(src.variation).toMatch(
      /Module 12.*retroflex lateral.*six movements.*three pen-down runs/i,
    );
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|one attested/i);
  });
});
