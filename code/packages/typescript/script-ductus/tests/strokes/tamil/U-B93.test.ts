import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../../src/scriptdata";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B93.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஓ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
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
      "6f2e53c736572608e6ef785885c995de601f3738ac6029ac29d0ba8a27eeb1da",
    );
  });

  it("uses the verified Tamil font", () => {
    expect(verifiedLetterFont("ஓ", letter.source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
  });

  it("ஓ follows Frame 15's three movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "circle the small loop into the crown",
      "sweep the large loop and curl in",
      "back, then round the hooked bowl",
    ]);
  });

  it("comes back along the tail into the hooked lower bowl", () => {
    const [, loop, bowl] = segs;
    expect(end(loop).x).toBeGreaterThan(800);
    expect(bowl.path[0]).toEqual(end(loop));
    expect(bowl.path[1].x).toBeLessThan(bowl.path[0].x);
    expect(Math.abs(bowl.path[1].y - bowl.path[0].y)).toBeLessThan(10);
    expect(Math.min(...bowl.path.map((p) => p.y))).toBeLessThan(-270);
    // It finishes turned inward, inside the hook.
    expect(end(bowl).x).toBeGreaterThan(380);
    expect(end(bowl).x).toBeLessThan(440);
    expect(end(bowl).y).toBeLessThan(-200);
  });

  it("ஓ's continuous order traces to Module 15 and Appendix I Frame 15", () => {
    const source = letter.source;
    expect(source.url).toContain("module-15");
    expect(source.citation).toMatch(
      /Module 15.*ஓ.*Appendix I.*Frame 15.*p\. 196/i,
    );
    expect(source.variation).toMatch(
      /long o.*three movements.*small left loop.*large right loop.*hooked lower bowl.*one continuous stroke.*in order without lifting.*comes back along the tail.*HP Labs India.*97% of the 29.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
