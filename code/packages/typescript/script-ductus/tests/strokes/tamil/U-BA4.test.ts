import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BA4.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["த"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BA4 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("த");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "382cc25a6ca4ed13f28d03ba6e05c9536b77abd105d80339d152256ea8a6a3d4",
    );
  });

  it("த follows its seven numbered movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "climb the short left upright",
      "carry the top bar to the middle",
      "carry the short upper bar right",
      "back and down the right bowl",
      "back up, round the left loop",
      "curl back to the centre",
      "the bowl again, then the tail",
    ]);
  });

  it("keeps the numbered order: right bowl, then the retrace to the left loop, then the tail", () => {
    const [climb, , bar, bowl, loop, curl, tail] = segs;
    expect(end(climb).y).toBeGreaterThan(climb.path[0].y + 150);
    expect(end(bar).x).toBeGreaterThan(bar.path[0].x + 150);
    // The bowl turns back from the bar's right end and finishes at its foot.
    expect(bowl.path[0].x).toBeGreaterThan(600);
    expect(end(bowl).y).toBeLessThan(50);
    // The loop movement climbs back round the bowl and crosses to the middle left.
    expect(loop.path.some((p) => p.x < 220 && p.y > 250 && p.y < 330)).toBe(true);
    expect(end(curl).y).toBeGreaterThan(250);
    // The tail retraces the bowl down and ends low on the left.
    expect(end(tail).x).toBeLessThan(150);
    expect(end(tail).y).toBeLessThan(-250);
  });

  it("த's continuous order traces to Frame 3 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*த.*p\. 192/i);
    expect(src.variation).toMatch(
      /Module 3 identifies.*dental stop.*final Frame 3 row numbers seven hand-movements.*1–2.*upper frame.*3–4.*right bowl.*5–6.*left loop.*7.*leftward tail.*one continuous stroke.*in order without lifting.*retraces the right bowl.*HP Labs India.*88% of the 266.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
