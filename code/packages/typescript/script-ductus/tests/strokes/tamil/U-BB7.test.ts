import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB7.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஷ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB7 stroke evidence", () => {
  registerStrokeHonestyTests([letter], { ஷ: 0.9 });

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஷ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "6e8740e77143179712334e9f68b12f6029bb16bc12bdf5492f5b46b1be8af32c",
    );
  });

  it("traces four numbered runs to Narale's Granthakshar diagram", () => {
    expect(letter.source.url).toBe(
      "https://tamilnavarasam.in/Books/Others/Tamil_eng_hindi.pdf",
    );
    expect(letter.source.citation).toMatch(
      /Narale.*Learn Tamil Through English\/Hindi.*Third Tamil Granthakshar ஷ.*p\. 13/i,
    );
    expect(letter.source.variation).toMatch(
      /four separate pen-down runs.*Noto Sans Tamil.*varies by school/i,
    );
    expect(penLifts(letter)).toBe(3);
  });
});
