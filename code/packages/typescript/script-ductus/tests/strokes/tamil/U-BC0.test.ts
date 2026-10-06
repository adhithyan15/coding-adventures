import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BC0.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ீ"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BC0 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ீ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "048eaa4ad2da11291fdf44df27ac90fc167ea4ab9f10bda91c5dbb9f68fe70bd",
    );
  });

  it("ீ is one unbroken stroke of three movements", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["curl up from the tail's foot", "over the top to the right", "curl down and back into the small loop"]);
  });

  it("starts at the foot of the tail, turns over the top, and closes in the small loop", () => {
    const [curl, over, loop] = segs;
    expect(curl.path[0].y).toBeLessThan(560);
    expect(curl.path[1].y).toBeGreaterThan(curl.path[0].y);
    expect(Math.max(...over.path.map((p) => p.y))).toBeGreaterThan(800);
    expect(end(over).x).toBeGreaterThan(curl.path[0].x + 300);
    expect(end(loop).y).toBeGreaterThan(750);
    expect(end(loop).x).toBeLessThan(end(over).x);
  });

  it("ீ's order traces to native writers' pen traces in LipiTk class 38", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 38 \(ீ, ii sign\).*native Tamil writers.*MIT licence/,
    );
    expect(src.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil.*varies by writer/);
  });
});
