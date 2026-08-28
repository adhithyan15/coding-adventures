import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B89.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["உ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B89 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("உ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "3ea3cf76c84dd90a04a3b844f35cb3a44d6f9e40d33ded79c2c60d965ccd3e4c",
    );
  });

  it("Tamil உ keeps all three Frame 16 movements in one run", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(letter.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "sweep outward around the compact upper spiral",
      "descend through the broad outer curve and turn left onto the baseline",
      "carry the long baseline straight to the right",
    ]);
  });
});
