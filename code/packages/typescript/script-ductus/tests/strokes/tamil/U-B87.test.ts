import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B87.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["இ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B87 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("இ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "e52779602da69abd296480fc351629e545a6dd04c5abe020e70c82efbfc1a581",
    );
  });

  it("இ follows Frame 4's seven movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl around the inner loop",
      "sweep down the inner right curve",
      "carry left and turn through the lower loop",
      "climb back to the left crossing",
      "carry right and turn around the lower loop",
      "climb the diagonal and outer left side",
      "arch over the top and down the right",
    ]);
  });

  it("runs down one lower diagonal and back up the other without retracing", () => {
    const [curl, inner, lowerLeft, climb, lowerRight, outer, arch] = segs;
    expect(curl.path[0].y).toBeGreaterThan(480);
    expect(end(inner).x).toBeGreaterThan(700);
    // Movement 3 runs on down the diagonal into the lower-left loop ...
    expect(Math.min(...lowerLeft.path.map((p) => p.y))).toBeLessThan(-250);
    expect(end(lowerLeft).x).toBeLessThan(120);
    // ... and movement 4 climbs that loop's outer side to the left crossing.
    expect(end(climb).y).toBeGreaterThan(30);
    expect(end(climb).x).toBeLessThan(250);
    expect(Math.max(...lowerRight.path.map((p) => p.x))).toBeGreaterThan(840);
    expect(end(lowerRight).y).toBeLessThan(-250);
    // Movement 6 climbs the other diagonal straight into the outer left side.
    expect(outer.path[1].x).toBeLessThan(outer.path[0].x);
    expect(end(outer).x).toBeLessThan(150);
    expect(end(outer).y).toBeGreaterThan(300);
    expect(Math.max(...arch.path.map((p) => p.y))).toBeGreaterThan(780);
    expect(end(arch).x).toBeGreaterThan(950);
    expect(end(arch).y).toBeLessThan(60);
  });

  it("இ's continuous order traces to Frame 4", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*இ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
    expect(src.variation).toMatch(
      /no single national stroke-order standard.*Frame 4 numbers seven hand-movements for இ.*one continuous stroke.*in order without lifting.*one unbroken line.*no movement retraces ink.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*100% of the 30 stored prototypes of இ are a single pen-down stroke/i,
    );
  });
});
