import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../../src/scriptdata";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B8F.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஏ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B8F stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஏ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "e3bb0b2fd9c8fa789254fdcba5740a3583283ec2868436ac75f7012922e71e16",
    );
  });

  it("uses the verified Tamil font", () => {
    expect(verifiedLetterFont("ஏ", letter.source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
  });

  it("keeps all six Frame 7 movements in one run", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(letter.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "sweep from the inner curl around the broad left loop",
      "climb the outer left side into the crown",
      "carry the top bar to the right",
      "retrace left and draw the right upright down",
      "continue down-left along the diagonal foot",
      "turn at the foot and finish back up-right",
    ]);
  });
});
