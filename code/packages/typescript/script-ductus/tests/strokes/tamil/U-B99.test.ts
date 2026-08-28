import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B99.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ங"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B99 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ங");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "90e136140f68a338248f3c6c60d8df334c3cf10c0468372cd81cebb6170e78ea",
    );
  });

  it("Tamil ங keeps Frame 2's detached upright and joined body separate", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(letter.strokes[1].segments.map((segment) => segment.label)).toEqual([
      "climb the tall left body",
      "carry the top bar right and return inward",
      "descend into the rounded inner turn",
      "carry the low bar to the right",
      "return left and finish up the inner stem",
    ]);
  });

  it("ங's two-run order traces to Appendix I Frame 2", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 2.*ங.*p\. 191/i);
    expect(src.variation).toMatch(
      /detached descending upright.*five joined movements.*detached upright on the right.*two-run order/i,
    );
  });
});
