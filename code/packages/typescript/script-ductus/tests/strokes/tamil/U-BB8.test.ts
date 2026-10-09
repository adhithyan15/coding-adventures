// ஸ (Grantha sa): one continuous stroke, cited to native writers' pen traces
// in HP Labs India's LipiTk Tamil recognizer (class 30; MIT model, counts and
// shares only). 150 of the 153 stored prototypes are one stroke: they start at
// the small loop on the left and turn it clockwise (147), come down to the
// foot of the letter three times — loop, stem, right-hand bowl — (87), and end
// at the top right (142), at the tip of the rising tail. The path is fitted to
// the bundled Noto Sans Tamil outline, in font units with y pointing UP.
import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, penPath, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB8.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஸ"];
const segs = letter.strokes[0].segments;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");
/** Twice the signed area of an open path closed back to its start; > 0 is anticlockwise (y up). */
const signedArea = (open: Point[]): number => {
  const path = [...open, open[0]];
  return path.slice(1).reduce((sum, b, index) => sum + path[index].x * b.y - b.x * path[index].y, 0);
};

describe("Tamil U-BB8 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஸ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "06b193341dd9092ac823767efeb73ec14cbe725872c6743a3714f4ca7583df68",
    );
  });

  it("ஸ is one stroke: the loop, the big arch, down the stem and back, the second arch, the bowl and the tail", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "start at the tip inside the left loop and curve up over it",
      "come down its right side, round the bottom and up the left side",
      "climb over the big arch and down to the top of the stem",
      "draw the stem straight down to its foot",
      "go back up the stem and over the second arch",
      "come down its right side and round the bottom",
      "climb the right side and curl in to the tip",
    ]);
  });

  it("turns the small loop clockwise from its inner tip, and ends at the top right", () => {
    const path = penPath(letter.strokes[0]);
    const xs = path.map((p) => p.x);
    const ys = path.map((p) => p.y);
    const start = path[0];
    const end = path.at(-1)!;
    // Starts in the left third, inside the loop; the loop turns clockwise.
    expect(start.x).toBeLessThan(Math.min(...xs) + (Math.max(...xs) - Math.min(...xs)) / 3);
    expect(signedArea([...segs[0].path, ...segs[1].path])).toBeLessThan(0);
    // Ends in the top third and the right third, at the tail's tip.
    expect(end.y).toBeGreaterThan(Math.max(...ys) - (Math.max(...ys) - Math.min(...ys)) / 3);
    expect(end.x).toBeGreaterThan(Math.max(...xs) - (Math.max(...xs) - Math.min(...xs)) / 3);
  });

  it("comes down to the foot of the letter three times: the loop, the stem and the bowl", () => {
    const foot = (points: Point[]) => Math.min(...points.map((p) => p.y));
    expect(foot(segs[1].path)).toBeLessThan(60);
    expect(segs[3].path.at(-1)!.y).toBeLessThan(60);
    expect(foot(segs[5].path)).toBeLessThan(60);
    // Between them the pen is high: the big arch, and the second arch.
    expect(Math.max(...segs[2].path.map((p) => p.y))).toBeGreaterThan(480);
    expect(Math.max(...segs[4].path.map((p) => p.y))).toBeGreaterThan(480);
    // The stem is retraced: it goes down and comes back up the same x.
    const stemXs = [...segs[3].path, ...segs[4].path.slice(0, 3)].map((p) => p.x);
    expect(Math.max(...stemXs) - Math.min(...stemXs)).toBeLessThan(30);
  });

  it("ஸ's order traces to LipiTk's Tamil recognizer, counts and shares only", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toBe(
      "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 30 (ஸ, sa): stored online-handwriting prototypes from native writers (MIT licence, 2012)",
    );
    expect(src.variation).toMatch(
      /150 of the 153 stored prototypes of ஸ \(98%\) are one pen-down stroke.*147 turn that loop clockwise.*142 end in the top third.*87 of the 150 \(58%\).*Noto Sans Tamil outline.*research use only, so only counts and shares are cited.*no trace was copied.*varies by school\.$/,
    );
  });
});
