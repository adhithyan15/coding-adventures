import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BA3.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ண"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BA3 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ண");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "c67b215e4a55f2ebfa8293f28ee260ca0401d65367f835a5b7febaf1db9bf409",
    );
  });

  it("ண follows Frame 13's seven movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl outward and climb the outer left",
      "arch over to the first junction",
      "descend around the first inner arch",
      "turn through the bottom and climb inside",
      "sweep through the extra inner arch",
      "carry the top bar right",
      "back, then down the right upright",
    ]);
  });

  it("carries the top bar out to the right edge, then comes back to draw the right upright down", () => {
    const bar = segs[5];
    const upright = segs[6];
    expect(end(bar).x).toBeGreaterThan(1600);
    expect(upright.path[0]).toEqual(end(bar));
    const corner = upright.path.findIndex((p) => p.x === 1418);
    expect(corner).toBeGreaterThan(0);
    for (const p of upright.path.slice(0, corner)) expect(p.y).toBe(518);
    for (const p of upright.path.slice(corner)) expect(p.x).toBe(1418);
    expect(end(upright).y).toBeLessThan(50);
  });

  it("ண's continuous order traces to Frame 13's adjacent row", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 13.*ண.*p\. 195/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
    expect(src.variation).toMatch(
      /no single national stroke-order standard.*Frame 13 numbers seven hand-movements for ண.*one continuous stroke.*in order without lifting.*comes back along it to the right upright.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*93% of the 212 stored prototypes of ண are a single pen-down stroke/i,
    );
  });
});
