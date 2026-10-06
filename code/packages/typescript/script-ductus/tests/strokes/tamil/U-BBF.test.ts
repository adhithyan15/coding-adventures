import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BBF.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ி"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BBF stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ி");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "44481f7585f91f303e3ed6e454ff5af87aef64ac7a6aed0e73a7bb504a94ab96",
    );
  });

  it("ி is one unbroken stroke of three movements", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["curl up from the hook's tip", "over the top to the right", "down the stem to the line"]);
  });

  it("starts at the hook tip, turns over the top to the right, and ends at the foot of the stem", () => {
    const [curl, over, stem] = segs;
    expect(curl.path[1].y).toBeGreaterThan(curl.path[0].y);
    expect(curl.path[1].x).toBeLessThan(curl.path[0].x);
    expect(Math.max(...over.path.map((p) => p.y))).toBeGreaterThan(780);
    expect(end(over).x).toBeGreaterThan(curl.path[0].x + 250);
    expect(end(stem).y).toBeLessThan(40);
  });

  it("ி's order traces to native writers' pen traces in LipiTk class 37", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 37 \(ி, i sign\).*native Tamil writers.*MIT licence/,
    );
    expect(src.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil.*varies by writer/);
  });
});
