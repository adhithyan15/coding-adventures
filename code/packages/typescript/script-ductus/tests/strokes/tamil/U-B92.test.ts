import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B92.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஒ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B92 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஒ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "ad20ab6c394a4673716136bcbfe0546ce9b7269148431c42631f6e83b2bd1e63",
    );
  });

  it("ஒ follows Frame 14's three movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "circle the small loop into the crown",
      "sweep the large loop and curl in",
      "back, then round the lower bowl",
    ]);
  });

  it("comes back along the tail into the lower bowl", () => {
    const [, loop, bowl] = segs;
    expect(end(loop).x).toBeGreaterThan(800);
    expect(bowl.path[0]).toEqual(end(loop));
    expect(bowl.path[1].x).toBeLessThan(bowl.path[0].x);
    expect(Math.abs(bowl.path[1].y - bowl.path[0].y)).toBeLessThan(10);
    expect(Math.min(...bowl.path.map((p) => p.y))).toBeLessThan(-270);
    expect(end(bowl).x).toBeLessThan(300);
    expect(end(bowl).y).toBeGreaterThan(-100);
  });

  it("ஒ's continuous order traces to Module 14 and Appendix I Frame 14", () => {
    const source = letter.source;
    expect(source.url).toContain("module-14");
    expect(source.citation).toMatch(
      /Module 14.*ஒ.*Appendix I.*Frame 14.*p\. 195/i,
    );
    expect(source.variation).toMatch(
      /short o.*three movements.*small left loop.*large right loop.*lower bowl.*one continuous stroke.*in order without lifting.*comes back along the tail.*HP Labs India.*99% of the 115.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
