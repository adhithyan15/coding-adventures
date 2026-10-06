import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, penPath, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BCD.ts";
import { tamilOutline } from "../../support/font-fixtures";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["்"];
const segs = letter.strokes[0].segments;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BCD stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("்");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "1feed4e42bf401ddf0e6378246a257c4189db156b1bbe94ba41b21a6314c775e",
    );
  });

  it("் is one touch of the pen: a single short dab", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["dab the dot"]);
  });

  it("stays inside the printed disc, close to its centre", () => {
    // Noto Sans Tamil's puḷḷi is one filled disc; the dab sits inside it.
    const { bounds } = tamilOutline("்");
    const centre = { x: (bounds.x0 + bounds.x1) / 2, y: (bounds.y0 + bounds.y1) / 2 };
    const radius = (bounds.x1 - bounds.x0) / 2;
    const path: Point[] = penPath(letter.strokes[0]);
    for (const point of path) {
      expect(Math.hypot(point.x - centre.x, point.y - centre.y)).toBeLessThan(radius / 2);
    }
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("்'s order traces to Varai's recorded drawings of the 18 consonants with puḷḷi", () => {
    const src = letter.source;
    expect(src.url).toBe("https://github.com/abhinayaRajarajan/varai");
    expect(src.citation).toMatch(
      /Abhinaya Rajarajan, Varai.*Swift Student Challenge 2026.*18 Tamil consonants with pulli.*952294fa.*no licence: facts only/,
    );
    expect(src.variation).toMatch(
      /In all 18 the consonant body is one stroke and the puḷḷi is a separate second stroke, made after the body.*Info-farmer.*Confidence is medium.*not a claim.*Noto Sans Tamil.*varies by writer/,
    );
  });
});
