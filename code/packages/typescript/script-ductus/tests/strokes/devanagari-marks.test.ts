// Devanagari vowel signs and other marks — each drawn by itself, with no
// consonant and no headline.
//
// Their pen lifts, start and direction are cited to native writers' tablet
// pen traces in HP Labs India's LipiTk Devanagari recognizer. The writers
// wrote each sign alone, so the traces say how the SIGN is written, not when
// it is written against its consonant or the shared headline: the book draws
// these signs only in lessons that teach the sign by itself, and composes no
// Devanagari word from them. The underlying data is licensed for research use
// only, so only counts and shares are cited, and every path below is fitted
// to the bundled Noto Sans Devanagari outline of the sign on its own, in font
// units with y pointing UP.
//
//     sign   class  strokes (share)   start                    then
//     ----   -----  ----------------  -----------------------  ------------------------------
//     ु      50     1 (79/83)         tip of the upper arm     right, round the bowl (clockwise), out left
//     ू      51     1 (83/83)         inner tip of the loop    left, over the top (clockwise), down the tail right
//     े      53     1 (162/165)       upper-left tip           right, then down to the right (116/165)
//     ं      57     1 (156/157)       top of the dot           anticlockwise round it (112/157)
//     ़      62     1 (82/83)         upper right of the dot   a short dab down to the left (82/83)
//     ्      61     1 (80/82)         upper-left end           down (79/82); the slant is the printed one
//     ृ      52     1 (81/83)         upper tip                left, round the bottom (anticlockwise), out right
//     ँ      59     2 (79/82)         crescent's left tip      crescent left to right, lift, then the dot (76/82)
//
// Left out, so their lessons stay undrawn:
//
//   * ा, ि, ी, ो and ः: Noto Sans Devanagari prints each with a short piece of
//     headline that the native traces do not draw (they were written without
//     one), so a path fitted to the traces leaves 3.5% to 37% of the printed ink
//     untraced, over the 2% the honesty check allows. ी and ो are also weak
//     majorities (43% and 51% for the drawn form).
//   * ै and ौ: the traces split. ै is two strokes in 92%, but each flag's
//     direction is a coin toss (34% draw both flags upper-left to lower-right);
//     ौ's most common form is three strokes in only 42%.
import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  DUCTUS,
  ductusKey,
  penLifts,
  penPath,
  type LetterDuctus,
  type Point,
} from "../../src/strokes";

const sign = (glyph: string): LetterDuctus => DUCTUS[ductusKey("devanagari", glyph)];
const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const first = (letter: LetterDuctus, stroke = 0): Point => letter.strokes[stroke].segments[0].path[0];
const last = (letter: LetterDuctus, stroke = letter.strokes.length - 1): Point =>
  letter.strokes[stroke].segments.at(-1)!.path.at(-1)!;
const xs = (letter: LetterDuctus, stroke = 0): number[] => penPath(letter.strokes[stroke]).map((p) => p.x);
const ys = (letter: LetterDuctus, stroke = 0): number[] => penPath(letter.strokes[stroke]).map((p) => p.y);
const labels = (letter: LetterDuctus): string[][] =>
  letter.strokes.map((stroke) => stroke.segments.map((segment) => segment.label));
/**
 * Twice the signed area of a pen path closed back to its start; positive
 * means it turns anticlockwise (y up). Closing it makes the sign independent
 * of where the origin is, which matters for open curves like ु.
 */
const signedArea = (open: Point[]): number => {
  const path = [...open, open[0]];
  return path.slice(1).reduce((sum, b, index) => sum + path[index].x * b.y - b.x * path[index].y, 0);
};

// The exact data each sign owns. A deliberate path change moves its hash here.
const HASHES: Record<string, string> = {
  "ु": "922f69e34ee729cea2998d180290f8652d7976ae422a05e7e5cc8924865e306c",
  "ू": "ac6ffefbabe664460dc871badac8720fcdecee9e95791d61755d56e552dd8d1d",
  "े": "c18cc325894240e864dbe6fbadb8cd0d587e41c00bc290182cd98ecfaf815139",
  "ं": "701fde20a6d076e06961e329bea4e162679b922c912a6f7d09a40779a4dbb7b7",
  "़": "0d741ced1d92b73731d983648f7df91ff49467e78a2344c7ffffc2988fc09b6f",
  "्": "a00b10150ac4dc00cd148ee3b48132c41cc63bf3ac26c21bcef809032cb27c9e",
  "ृ": "85540d72eb0058ca7ab8d7b9a65aa5c50357d9b29366bd57d350152aff67da31",
  "ँ": "524adfe04ec18814a5505893f7d97091b73cc95e91b5e9b8e6e89e584865ef0d",
};

const CLASSES: Record<string, number> = {
  "ु": 50, "ू": 51, "े": 53, "ं": 57, "़": 62, "्": 61, "ृ": 52, "ँ": 59,
};

const LIFTS: Record<string, number> = {
  "ु": 0, "ू": 0, "े": 0, "ं": 0, "़": 0, "्": 0, "ृ": 0, "ँ": 1,
};

describe("Devanagari sign ductus records", () => {
  for (const glyph of Object.keys(HASHES)) {
    it(`${glyph} is a Devanagari owner with its cited lifts and exact data`, () => {
      const letter = sign(glyph);
      expect(letter.script).toBe("devanagari");
      expect(letter.glyph).toBe(glyph);
      expect(penLifts(letter)).toBe(LIFTS[glyph]);
      expect(sha256(JSON.stringify(letter))).toBe(HASHES[glyph]);
    });

    it(`${glyph}'s order traces to native writers in HP Labs India's LipiTk recognizer`, () => {
      const source = sign(glyph).source;
      expect(source.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
      expect(source.citation).toMatch(
        new RegExp(
          `^HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4\\.0, Devanagari recognizer, ` +
            `class ${CLASSES[glyph]} \\(${glyph}, [^)]+\\): stored online-handwriting prototypes ` +
            `from native writers \\(MIT licence, 2012\\)$`,
          "u",
        ),
      );
      expect(source.variation).toMatch(
        /^The recognizer stores native writers' tablet pen traces.*written by itself, with no consonant beside it.*not when it is written against its consonant or the headline.*Noto Sans Devanagari outline of the sign on its own.*research use only, so only counts and shares are cited.*varies by writer\.$/,
      );
    });
  }

  it("leaves out the signs whose evidence or printed form does not fit", () => {
    for (const glyph of ["ा", "ि", "ी", "ै", "ो", "ौ", "ः"]) {
      expect(DUCTUS[ductusKey("devanagari", glyph)], glyph).toBeUndefined();
    }
  });
});

describe("one-stroke signs", () => {
  it("ु starts at the upper arm's tip, swings right round the bowl, and leaves to the left", () => {
    const u = sign("ु");
    expect(labels(u)).toEqual([
      ["start at the upper arm's tip and swing to the right", "round the bowl and back to the left", "sweep out to the lower-left tip"],
    ]);
    expect(u.strokes[0].segments[0].path.at(-1)!.x).toBeGreaterThan(first(u).x + 150);
    expect(last(u).x).toBe(Math.min(...xs(u)));
    // Clockwise on the page: negative signed area with y up.
    expect(signedArea(penPath(u.strokes[0]))).toBeLessThan(0);
  });

  it("ू starts at the loop's inner tip, curls left and over the top, and ends down the tail to the right", () => {
    const uu = sign("ू");
    expect(labels(uu)).toEqual([
      ["start at the loop's inner tip and curl to the left", "climb and arch over the top", "sweep down to the right"],
    ]);
    expect(uu.strokes[0].segments[0].path.at(-1)!.x).toBeLessThan(first(uu).x - 150);
    expect(last(uu).x).toBe(Math.max(...xs(uu)));
    expect(last(uu).y).toBe(Math.min(...ys(uu)));
    expect(signedArea(penPath(uu.strokes[0]))).toBeLessThan(0);
  });

  it("े runs from its upper-left tip to the right and down, and draws no headline", () => {
    const e = sign("े");
    expect(labels(e)).toEqual([["arc to the right along the top", "curve down to the right"]]);
    expect(first(e).x).toBe(Math.min(...xs(e)));
    expect(last(e).x).toBe(Math.max(...xs(e)));
    expect(last(e).y).toBe(Math.min(...ys(e)));
    expect(e.strokes).toHaveLength(1);
  });

  it("ं is one small loop from its top, anticlockwise, that closes where it began", () => {
    const anusvara = sign("ं");
    expect(labels(anusvara)).toEqual([
      ["start at the top and curve down the left side", "round the bottom and up the right side to close the loop"],
    ]);
    expect(first(anusvara).y).toBe(Math.max(...ys(anusvara)));
    expect(last(anusvara)).toEqual(first(anusvara));
    expect(signedArea(penPath(anusvara.strokes[0]))).toBeGreaterThan(0);
  });

  it("़ is a short dab down to the left", () => {
    const nukta = sign("़");
    expect(labels(nukta)).toEqual([["dab the dot down to the left"]]);
    expect(last(nukta).x).toBeLessThan(first(nukta).x);
    expect(last(nukta).y).toBeLessThan(first(nukta).y);
  });

  it("् is one stroke drawn downward from its upper-left end", () => {
    const virama = sign("्");
    expect(labels(virama)).toEqual([["draw the stroke down to the right"]]);
    expect(first(virama).x).toBe(Math.min(...xs(virama)));
    expect(last(virama).y).toBe(Math.min(...ys(virama)));
  });

  it("ृ starts at its upper tip, curls left round the bottom, and runs out to the right", () => {
    const r = sign("ृ");
    expect(labels(r)).toEqual([
      ["start at the upper tip and curve to the left", "round down and along the bottom", "run out to the lower-right tip"],
    ]);
    expect(first(r).y).toBe(Math.max(...ys(r)));
    expect(r.strokes[0].segments[0].path.at(-1)!.x).toBeLessThan(first(r).x - 100);
    expect(last(r).x).toBe(Math.max(...xs(r)));
    expect(signedArea(penPath(r.strokes[0]))).toBeGreaterThan(0);
  });
});

describe("a sign written in two strokes", () => {
  it("ँ draws the crescent left to right, then lifts for the dot above it", () => {
    const candrabindu = sign("ँ");
    expect(labels(candrabindu)).toEqual([
      ["start at the crescent's left tip and curve down", "round the bottom and up to the right tip"],
      ["lift, then dab the dot above the crescent"],
    ]);
    expect(first(candrabindu, 0).x).toBe(Math.min(...xs(candrabindu, 0)));
    expect(last(candrabindu, 0).x).toBe(Math.max(...xs(candrabindu, 0)));
    expect(signedArea(penPath(candrabindu.strokes[0]))).toBeGreaterThan(0);
    expect(Math.min(...ys(candrabindu, 1))).toBeGreaterThan(Math.min(...ys(candrabindu, 0)));
    expect(last(candrabindu, 1).y).toBeLessThan(first(candrabindu, 1).y);
  });
});
