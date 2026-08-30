import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B90.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஐ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B90 stroke evidence", () => {
  registerStrokeHonestyTests([letter], { ஐ: 0.9 });

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஐ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "737210ab9811d1dceaf3d047950058e5dedcaabc9969f4f5d1b740afaf07b5ad",
    );
  });

  it("ஐ's five-run order traces to the Commons animation and Frame 11", () => {
    expect(letter.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Writing_Tamil_10.gif",
    );
    expect(letter.source.citation).toMatch(
      /Info-farmer.*Writing Tamil 10.*ஐ.*CC BY-SA 3\.0.*Frame 11.*p\. 194/i,
    );
    expect(letter.source.variation).toMatch(
      /13-frame.*five separate runs.*spiral.*upright.*upper-right loop.*lower-left bowl.*lower-right bowl.*seven movements.*Noto Sans Tamil.*varies by school/i,
    );
    expect(penLifts(letter)).toBe(4);
  });
});
