import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BC8.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ை"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BC8 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ை");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "a548fc2a2fae57144d51760bc3586a5273b7c8e6b2728388c5fd6ad93726195c",
    );
  });

  it("ை is one unbroken stroke of three movements", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["circle the small loop on the left", "climb and arch over to the right", "up the middle, over the second arch"]);
  });

  it("circles the left loop, arches over, and ends down the second arch on the right", () => {
    const [loop, firstArch, secondArch] = segs;
    expect(loop.path[0].x).toBeLessThan(150);
    expect(Math.max(...loop.path.map((p) => p.x))).toBeLessThan(450);
    expect(Math.max(...firstArch.path.map((p) => p.y))).toBeGreaterThan(500);
    expect(end(firstArch).y).toBeLessThan(60);
    expect(Math.max(...secondArch.path.map((p) => p.x))).toBeGreaterThan(1000);
    expect(end(secondArch).y).toBeLessThan(60);
  });

  it("ை's order traces to native writers' pen traces in LipiTk class 43", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 43 \(ை, ai sign\).*native Tamil writers.*MIT licence/,
    );
    expect(src.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil.*varies by writer/);
  });
});
