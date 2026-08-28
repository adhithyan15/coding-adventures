import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB4.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ழ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB4 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ழ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "4267ccf24cf6d8fb66a95e6483c80dd54ffb7ce12d253ee156574d2aa97c1258",
    );
  });

  it("Tamil ழ groups six movements into three source-verified pen-down runs", () => {
    expect(penLifts(letter)).toBe(2);
    expect(letter.strokes).toHaveLength(3);
    expect(
      letter.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb the outer left upright",
        "retrace down the left upright",
        "carry the low crossbar right",
      ],
      [
        "retrace left into the inner upright",
        "descend and sweep around the broad right bowl",
      ],
      ["turn through the detached lower hook"],
    ]);
  });

  it("ழ's three-run stroke order traces to Appendix I Frame 7", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 7.*ழ.*p\. 193/i);
    expect(src.variation).toMatch(
      /six movements.*three pen-down runs.*1–3.*left body and bar.*4–5.*inner upright and broad right bowl.*movement 6.*detached lower hook.*Noto Sans Tamil.*low crossbar.*varies by school.*three-run order/i,
    );
  });
});
