import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../../src/scriptdata";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B93.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஓ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B93 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஓ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "a65e0df5348fde8e9973374736d1461c0db921543731caae49a16cfd0e056d2d",
    );
  });

  it("uses the verified Tamil font", () => {
    expect(verifiedLetterFont("ஓ", letter.source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
  });

  it("keeps the two upper movements joined before the lower bowl", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(
      letter.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "circle the small left loop and climb into the crown",
        "sweep through the large right loop and curl inward",
      ],
      ["sweep around the separate hooked lower bowl"],
    ]);
  });
});
