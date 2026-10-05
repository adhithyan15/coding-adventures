import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB4.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ழ"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB4 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ழ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "d03c4defab3548f30b48226dc76688641e90f86689e4208526b93ff774e170b0",
    );
  });

  it("Tamil ழ follows Frame 7's six movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "climb the left upright",
      "back down it",
      "along the bar and up the bowl",
      "left over the top, into the stem",
      "down the stem",
      "to the hook's tip and back",
    ]);
  });

  it("runs the low crossbar round the bowl, comes back over the top, and descends into the hook", () => {
    const [climb, down, bar, over, stem, hook] = segs;
    expect(end(climb).y).toBeGreaterThan(500);
    expect(end(down).y).toBeLessThan(60);
    // The print face's low crossbar runs straight into the bowl's foot.
    expect(Math.max(...bar.path.map((p) => p.x))).toBeGreaterThan(740);
    expect(end(bar).y).toBeGreaterThan(480);
    expect(end(over).x).toBeLessThan(end(bar).x - 150);
    expect(end(stem).y).toBeLessThan(-100);
    // Round the hook to its tip, then back round it into the short exit.
    const tip = hook.path.findIndex((p) => p.x > 200 && p.x < 300 && p.y > -120);
    expect(tip).toBeGreaterThan(0);
    expect(Math.min(...hook.path.slice(0, tip).map((p) => p.y))).toBeLessThan(-270);
    expect(Math.min(...hook.path.slice(tip).map((p) => p.y))).toBeLessThan(-270);
    expect(end(hook).x).toBeGreaterThan(650);
  });

  it("ழ's continuous order traces to Appendix I Frame 7", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 7.*ழ.*p\. 193/i);
    expect(src.variation).toMatch(
      /six movements.*1–3.*left body and bar.*4–5.*inner upright and broad right bowl.*6.*lower hook.*one continuous stroke.*in order without lifting.*Noto Sans Tamil.*low crossbar.*round the hook to its tip.*HP Labs India.*97% of the 182.*single pen-down stroke.*varies by school/i,
    );
  });
});
