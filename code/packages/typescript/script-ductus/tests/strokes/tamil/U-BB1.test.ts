import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB1.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ற"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB1 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ற");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "d39cdef30b0014f0baac70401c6a0487cb902dfa34e0019fd8e72df4e09d9b11",
    );
  });

  it("ற follows Frame 10's five movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "climb and arch to the middle",
      "descend the middle upright",
      "climb back up the same upright",
      "arch over, down the right side",
      "sweep left and drop",
    ]);
  });

  it("descends the shared middle upright and climbs back up the same ink", () => {
    const [, down, up, arch, sweep] = segs;
    expect(end(down).y).toBeLessThan(down.path[0].y - 300);
    expect(end(up).y).toBeGreaterThan(up.path[0].y + 300);
    for (const p of up.path) expect(Math.abs(p.x - down.path[0].x)).toBeLessThan(15);
    expect(Math.max(...arch.path.map((p) => p.x))).toBeGreaterThan(700);
    expect(end(sweep).y).toBeLessThan(-250);
  });

  it("ற's continuous order traces to Frame 10", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 10.*ற.*p\. 194/i);
    expect(src.variation).toMatch(
      /Frame 10 numbers five hand-movements.*one continuous stroke.*in order without lifting.*single shared middle upright.*movement 3 climbs back up.*HP Labs India.*99% of the 378.*single pen-down stroke/i,
    );
    // The note must not present one order as the only order.
    expect(src.variation).toMatch(/no single/i);
  });
});
