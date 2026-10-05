import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BA9.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ன"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BA9 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ன");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "d583e29aabfece167076a47321a81863f6ee1f9d72afba6578f36c345e6f09e8",
    );
  });

  it("ன follows Frame 13's six movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl outward and climb the outer left",
      "arch over the left loop to the middle",
      "descend around the single inner arch",
      "turn through the bottom and climb inside",
      "carry the top bar right",
      "back, then down the right upright",
    ]);
  });

  it("carries the top bar out to the right edge, then comes back to draw the right upright down", () => {
    const bar = segs[4];
    const upright = segs[5];
    expect(end(bar).x).toBeGreaterThan(1190);
    expect(upright.path[0]).toEqual(end(bar));
    const corner = upright.path.findIndex((p) => p.x === 1004);
    expect(corner).toBeGreaterThan(0);
    for (const p of upright.path.slice(0, corner)) expect(p.y).toBe(518);
    for (const p of upright.path.slice(corner)) expect(p.x).toBe(1004);
    expect(end(upright).y).toBeLessThan(50);
  });

  it("ன's continuous order traces to Frame 13's first row", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 13.*ன.*p\. 195/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
    expect(src.variation).toMatch(
      /no single national stroke-order standard.*Frame 13 numbers six hand-movements for ன.*one continuous stroke.*in order without lifting.*comes back along it to the right upright.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*94% of the 335 stored prototypes of ன are a single pen-down stroke/i,
    );
  });
});
