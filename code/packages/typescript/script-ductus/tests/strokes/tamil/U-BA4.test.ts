import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BA4.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["த"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BA4 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("த");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "7cfc235bcbe3675c1b6d13045f399fc94a5ed62c3ad20194dac7176c4c272867",
    );
  });

  it("த groups seven movements into four source-verified pen-down runs", () => {
    expect(penLifts(letter)).toBe(3);
    expect(letter.strokes).toHaveLength(4);
    expect(
      letter.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      ["climb the short left upright", "carry the top bar to the right"],
      [
        "carry the short upper bar right",
        "curve down around the broad right bowl",
      ],
      [
        "turn around the compact left loop",
        "curl back to the central crossing",
      ],
      ["sweep the low tail left"],
    ]);
  });

  it("த's four-run order traces to Frame 3 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*த.*p\. 192/i);
    expect(src.variation).toMatch(
      /Module 3 identifies.*dental stop.*final Frame 3 row.*four separate pen-down runs.*1–2.*upper frame.*3–4.*right bowl.*5–6.*left loop.*movement 7.*leftward tail.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
  });
});
