// ---------------------------------------------------------------------------
// Gurmukhi pen paths: geometry against the font, order against the source
// ---------------------------------------------------------------------------
//
// Two different questions are asked of every Gurmukhi letter here.
//
//   1. Does the path lie on the printed letter? `registerStrokeHonestyTests`
//      measures it against Noto Sans Gurmukhi at the DEFAULT tolerances (no
//      per-letter override): every stroke at least 97% on ink, joins closed,
//      at most 2% of the ink untraced.
//   2. Does the path say what the cited tracing lesson says? GNPS's Gurmukhi
//      Sikho app (Apache-2.0) orders each letter's strokes, and the facts
//      pinned below are the ones a reader of that lesson can check: the
//      headline comes first (or, for the five split-headline letters, there
//      is none), how many times the pen lifts, which way the body turns.
//
// Turning is the sum of the pen's heading changes along a stroke, in FONT
// coordinates (y up), where a left turn is positive. So a NEGATIVE sum is
// CLOCKWISE as seen on the page. A turn of half a circle on the spot (the pen
// going back the way it came) is skipped: it has no sense of rotation.
// ---------------------------------------------------------------------------

import { beforeAll, describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../src/scriptdata";
import {
  DUCTUS,
  ductusKey,
  penLifts,
  penPath,
  type LetterDuctus,
  type Point,
} from "../../src/strokes";
import { registerStrokeHonestyTests } from "../support/stroke-honesty";

const PA = (glyph: string): LetterDuctus => DUCTUS[ductusKey("gurmukhi", glyph)];
const GLYPHS = [
  "ਅ", "ਸ", "ਹ", "ਕ", "ਖ", "ਗ", "ਘ", "ਚ", "ਛ", "ਜ", "ਟ", "ਠ", "ਡ", "ਣ",
  "ਤ", "ਥ", "ਦ", "ਨ", "ਪ", "ਫ", "ਬ", "ਭ", "ਮ", "ਰ", "ਲ", "ਵ", "ੜ",
] as const;
/** The five letters whose Noto headline is printed in two parts. */
const SPLIT = ["ਅ", "ਖ", "ਘ", "ਪ", "ਮ"] as const;
/** The three letters joined down to the Omniglot copyists' most common count. */
const JOINED = ["ਛ", "ਨ", "ਬ"] as const;
const HEADLINE = "draw the headline from left to right";

const letters = (Object.values(DUCTUS) as LetterDuctus[]).filter(
  (letter) => letter.script === "gurmukhi",
);

/** Total turning of a polyline in degrees; > 0 is counterclockwise on the page. */
const turning = (points: Point[]): number => {
  let total = 0;
  for (let i = 0; i + 2 < points.length; i++) {
    const [a, b, c] = [points[i], points[i + 1], points[i + 2]];
    const h1 = Math.atan2(b.y - a.y, b.x - a.x);
    const h2 = Math.atan2(c.y - b.y, c.x - b.x);
    const d = ((h2 - h1 + 3 * Math.PI) % (2 * Math.PI)) - Math.PI;
    if (Math.abs(Math.abs(d) - Math.PI) < 0.05) continue;
    total += d;
  }
  return (total * 180) / Math.PI;
};
const labels = (letter: LetterDuctus): string[][] =>
  letter.strokes.map((stroke) => stroke.segments.map((segment) => segment.label));
const first = (letter: LetterDuctus, stroke = 0): Point => penPath(letter.strokes[stroke])[0];
const last = (letter: LetterDuctus, stroke = 0): Point => penPath(letter.strokes[stroke]).at(-1)!;

describe("Gurmukhi handwriting ductus", () => {
  // Default tolerances for every letter: no overrides.
  registerStrokeHonestyTests(letters);

  beforeAll(() => {
    for (const glyph of GLYPHS) {
      expect(verifiedLetterFont(glyph, PA(glyph).source.url)).toBe(
        "_fonts/NotoSansGurmukhi-Static.ttf",
      );
    }
  });

  it("authors exactly the 27 letters the Punjabi lessons print, in owner order", () => {
    expect(letters.map((letter) => letter.glyph)).toEqual([...GLYPHS]);
    expect(Object.keys(DUCTUS).filter((key) => key.startsWith("gurmukhi:"))).toEqual(
      GLYPHS.map((glyph) => `gurmukhi:${glyph}`),
    );
  });

  it("lifts the pen as often as the source, or as the copyists' ceiling where that is lower", () => {
    expect(Object.fromEntries(GLYPHS.map((glyph) => [glyph, penLifts(PA(glyph))]))).toEqual({
      "ਅ": 1, "ਸ": 2, "ਹ": 1, "ਕ": 1, "ਖ": 2, "ਗ": 2, "ਘ": 1, "ਚ": 1, "ਛ": 1,
      "ਜ": 2, "ਟ": 1, "ਠ": 1, "ਡ": 1, "ਣ": 2, "ਤ": 1, "ਥ": 3, "ਦ": 1, "ਨ": 2,
      "ਪ": 1, "ਫ": 1, "ਬ": 2, "ਭ": 1, "ਮ": 1, "ਰ": 1, "ਲ": 3, "ਵ": 2, "ੜ": 2,
    });
  });

  it("draws the headline first, left to right, wherever Noto prints it in one piece", () => {
    for (const glyph of GLYPHS.filter((g) => !(SPLIT as readonly string[]).includes(g))) {
      const letter = PA(glyph);
      expect(labels(letter)[0], glyph).toEqual([HEADLINE]);
      const bar = penPath(letter.strokes[0]);
      expect(bar[0].x, glyph).toBeLessThan(30);
      expect(bar.at(-1)!.x - bar[0].x, glyph).toBeGreaterThan(500);
      for (const point of bar) expect(point.y, glyph).toBe(586);
      expect(labels(letter)[1][0], glyph).toMatch(/^lift, then /);
    }
  });

  it("draws no separate headline where Noto splits it: the bar's parts open the first and last strokes", () => {
    for (const glyph of SPLIT) {
      const letter = PA(glyph);
      expect(letter.strokes.flatMap((stroke) => stroke.segments).map((s) => s.label)).not.toContain(HEADLINE);
      expect(labels(letter)[0][0], glyph).toBe("start at the top left and curve down");
      expect(first(letter)).toEqual({ x: 15, y: 586 });
      const lastStroke = letter.strokes.length - 1;
      expect(labels(letter)[lastStroke], glyph).toEqual(["lift, then run left along the top", "turn down the stem"]);
      const [bar, stem] = letter.strokes[lastStroke].segments;
      expect(bar.path[0].x, glyph).toBeGreaterThan(bar.path.at(-1)!.x);
      expect(stem.path.at(-1)!.y, glyph).toBeLessThan(50);
    }
  });

  it("turns each body the way the source's stroke turns", () => {
    // Body strokes the source turns through at least 150 degrees, and which way.
    const clockwise: Array<[string, number]> = [
      ["ਅ", 0], ["ਸ", 1], ["ਹ", 1], ["ਕ", 1], ["ਗ", 1], ["ਚ", 1], ["ਛ", 1], ["ਜ", 2],
      ["ਡ", 1], ["ਤ", 1], ["ਦ", 1], ["ਭ", 1], ["ਮ", 0], ["ਰ", 1], ["ੜ", 1],
    ];
    const counterclockwise: Array<[string, number]> = [["ਘ", 0], ["ਠ", 1], ["ਫ", 1]];
    for (const [glyph, stroke] of clockwise) {
      expect(turning(penPath(PA(glyph).strokes[stroke])), glyph).toBeLessThan(-150);
    }
    for (const [glyph, stroke] of counterclockwise) {
      expect(turning(penPath(PA(glyph).strokes[stroke])), glyph).toBeGreaterThan(150);
    }
    // ਅ and ਡ turn about two full turns: two loops, both clockwise.
    expect(turning(penPath(PA("ਅ").strokes[0]))).toBeLessThan(-600);
    expect(turning(penPath(PA("ਡ").strokes[1]))).toBeLessThan(-600);
  });

  it("ends each body where the source's stroke ends", () => {
    expect(last(PA("ਕ"), 1).y).toBeLessThan(50); // the foot, bottom right
    expect(last(PA("ਕ"), 1).x).toBeGreaterThan(500);
    expect(last(PA("ਹ"), 1).x).toBeLessThan(300); // the open end, upper middle
    expect(last(PA("ਤ"), 1).x).toBeLessThan(100); // the tail, bottom left
    expect(last(PA("ਭ"), 1).x).toBeLessThan(100);
    expect(last(PA("ਰ"), 1).y).toBeGreaterThan(300); // back at the stem, upper right
    expect(last(PA("ਗ"), 1).y).toBeGreaterThan(300); // back at the middle stem
    // ਠ closes its bowl where the short stem met it.
    expect(Math.abs(last(PA("ਠ"), 1).x - 330)).toBeLessThan(10);
    expect(last(PA("ਠ"), 1).y).toBeGreaterThan(400);
  });

  it("joins ਛ ਨ ਬ on the ink instead of lifting where the source restarts", () => {
    expect(labels(PA("ਛ"))[1].slice(-2)).toEqual(["run back along the bowl's top", "draw the middle stroke down"]);
    expect(labels(PA("ਬ"))[1]).toContain("run back left along the bar");
    // ਨ: the left leg begins where the stem ends, so the two are one stroke;
    // the right leg starts again from that same point.
    expect(labels(PA("ਨ"))[1]).toEqual(["lift, then come down the stem", "curve down the left leg"]);
    expect(Math.hypot(first(PA("ਨ"), 2).x - 332, first(PA("ਨ"), 2).y - 412)).toBeLessThan(5);
    for (const glyph of JOINED) {
      expect(PA(glyph).source.variation).toMatch(/ceiling on native lifts/);
    }
  });

  it("every Gurmukhi order cites GNPS's tracing lesson at a pinned commit, with Omniglot shares", () => {
    for (const glyph of GLYPHS) {
      const source = PA(glyph).source;
      expect(source.url).toMatch(
        /^https:\/\/github\.com\/codemanxdev\/gnps_learning_hub\/blob\/de1e56014cb0caf7e56de2320681781cae6a7e41\/lib\/data\/lessons\/lesson_tracing\.dart#L\d+$/,
      );
      expect(source.citation).toMatch(
        new RegExp(`^GNPS, Gurmukhi Sikho \\(GNPS Learning Hub\\), Alphabet Tracing lesson, ${glyph} \\(\\w+\\) checkpoints, strokes 1–\\d \\(lib/data/lessons/lesson_tracing\\.dart line \\d+, commit de1e560; Apache-2\\.0\\)$`),
      );
      expect(source.citation).toContain(`line ${source.url.split("#L")[1]},`);
      expect(source.variation).toMatch(/Omniglot \(Lake, Salakhutdinov & Tenenbaum.*\d+% \(\d+ of 20\)/);
      expect(source.variation).toMatch(/no checkpoint coordinate is copied.*Noto Sans Gurmukhi.*varies by writer\.$/);
      if ((SPLIT as readonly string[]).includes(glyph)) {
        expect(source.variation).toMatch(/top bar is split/);
      } else {
        expect(source.variation).toMatch(/headline first is the source's teaching order.*adding the headline last/);
      }
      // Omniglot never stands in as support for drawing the headline first.
      expect(source.variation).not.toMatch(/copyists?[^.]*headline first/);
    }
  });
});
