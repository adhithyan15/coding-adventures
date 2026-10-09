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
//     ा      47     1 (55/81); 2      top of the stem          down (54/55); the 2-stroke form
//                   (23/81)                                    adds a top stroke, left to right
//                                                              (20/23): the headline piece Noto
//                                                              prints, drawn last, as in आ
//     ु      50     1 (79/83)         tip of the upper arm     right, round the bowl (clockwise), out left
//     ू      51     1 (83/83)         inner tip of the loop    left, over the top (clockwise), down the tail right
//     े      53     1 (162/165)       upper-left tip           right, then down to the right (116/165)
//     ं      57     1 (156/157)       top of the dot           anticlockwise round it (112/157)
//     ़      62     1 (82/83)         upper right of the dot   a short dab down to the left (82/83)
//     ्      61     1 (80/82)         upper-left end           down (79/82); the slant is the printed one
//     ृ      52     1 (81/83)         upper tip                left, round the bottom (anticlockwise), out right
//     ँ      59     2 (79/82)         crescent's left tip      crescent left to right, lift, then the dot (76/82)
//     ी      49     1 body (55/91)    lower tip of the hook    up, over the top, down the stem: arch first
//                                                              (49/91), and of those in one run (44/49)
//     ो      55     2 (42/83)         top of the stem          down; lift (58/83 stem down, then a lift);
//                                                              the flag from its upper-left tip (41/58), as े
//     ः      60     2 (77/81)         top of the upper dot     anticlockwise loop; lift; the lower dot the
//                                                              same way (upper first 76/77, both anticlockwise 66/77)
//
// ी, ो and ः are printed with a short piece of headline (a stub) that the
// writers, who wrote each sign without a headline, do not draw: it is the
// word's headline, which they draw last across the whole word. Their paths
// leave it undrawn, and the coverage check excuses exactly that rectangle
// (HEADLINE_STUBS in devanagari.test.ts).
//
// ā is the one sign whose PLACE in a word is cited too (its mark record's
// compositionSource: the cited आ draws the same bar after its body and before
// its headline), so it is also the one sign a composed word may hold
// (headline-word.ts). Its path draws the piece of headline the printed sign
// carries, so nothing of the printed sign is left untraced.
//
// Left out, so their lessons stay undrawn — no form wins a majority:
//
//   * ि: of 75 prototypes, 31 draw the stem down, lift and add the arch; 22
//     draw one run from the arch's right tip, over and down the stem; 20 draw
//     one run up the stem and over to the right tip.
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
  "ा": "e31d04eb552dd0afc2504818e5db625d210ec455476aa8ec067e64abd1c49ecf",
  "ु": "922f69e34ee729cea2998d180290f8652d7976ae422a05e7e5cc8924865e306c",
  "ू": "ac6ffefbabe664460dc871badac8720fcdecee9e95791d61755d56e552dd8d1d",
  "े": "c18cc325894240e864dbe6fbadb8cd0d587e41c00bc290182cd98ecfaf815139",
  "ं": "701fde20a6d076e06961e329bea4e162679b922c912a6f7d09a40779a4dbb7b7",
  "़": "0d741ced1d92b73731d983648f7df91ff49467e78a2344c7ffffc2988fc09b6f",
  "्": "a00b10150ac4dc00cd148ee3b48132c41cc63bf3ac26c21bcef809032cb27c9e",
  "ृ": "85540d72eb0058ca7ab8d7b9a65aa5c50357d9b29366bd57d350152aff67da31",
  "ँ": "524adfe04ec18814a5505893f7d97091b73cc95e91b5e9b8e6e89e584865ef0d",
  "ी": "25c57ed2f5beead731c6838e9ed6f90566e046b9842f4e0d86a5bb98f5f01376",
  "ो": "5669ec37768dc82b0ad412cb188ad9cfe77a109006c0b122331cbb038b5d3a8c",
  "ः": "ac7d75993c235a73df7e095c817df3a4f20857e174e818ee4d81985e84cf72bc",
};

const CLASSES: Record<string, number> = {
  "ा": 47, "ु": 50, "ू": 51, "े": 53, "ं": 57, "़": 62, "्": 61, "ृ": 52, "ँ": 59,
  "ी": 49, "ो": 55, "ः": 60,
};

const LIFTS: Record<string, number> = {
  "ा": 1, "ु": 0, "ू": 0, "े": 0, "ं": 0, "़": 0, "्": 0, "ृ": 0, "ँ": 1,
  "ी": 0, "ो": 1, "ः": 1,
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

  it("leaves out the signs whose traces have no majority form", () => {
    for (const glyph of ["ि", "ै", "ौ"]) {
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

  it("ी starts at the hook's lower tip, arches over the top and runs down the stem, leaving the stub undrawn", () => {
    const ii = sign("ी");
    expect(labels(ii)).toEqual([
      ["start at the hook's lower tip and climb", "arch over the top and down to the right", "draw the stem straight down"],
    ]);
    // Starts at the hook's lower tip — the lowest point left of the stem — and
    // ends at the foot of the stem.
    const hook = penPath(ii.strokes[0]).filter((point) => point.x < 0);
    expect(first(ii).y).toBe(Math.min(...hook.map((point) => point.y)));
    expect(first(ii).x).toBeLessThan(-100);
    expect(last(ii).y).toBe(Math.min(...ys(ii)));
    // The arch: up the left side before it turns right, and its top is the
    // run's highest point.
    expect(ii.strokes[0].segments[0].path.at(-1)!.y).toBeGreaterThan(first(ii).y + 150);
    expect(Math.max(...ys(ii))).toBeGreaterThan(850);
    // Clockwise on the page (over the top from left to right): negative signed area.
    expect(signedArea(penPath(ii.strokes[0]))).toBeLessThan(0);
    // The stem is straight down; no movement runs along the headline stub.
    const stem = ii.strokes[0].segments[2].path;
    expect(Math.max(...stem.map((p) => p.x)) - Math.min(...stem.map((p) => p.x))).toBeLessThanOrEqual(2);
    expect(Math.max(...xs(ii))).toBeLessThan(170);
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

describe("signs written in two strokes", () => {
  it("ा draws its stem down from the top, then lifts for its piece of headline, left to right", () => {
    const aa = sign("ा");
    expect(labels(aa)).toEqual([["draw the stem straight down"], ["lift, then draw the shirorekha rightward"]]);
    // The stem: straight down from inside the headline piece to the foot.
    expect(new Set(xs(aa, 0)).size).toBe(1);
    expect(first(aa, 0).y).toBe(Math.max(...ys(aa, 0)));
    expect(last(aa, 0).y).toBe(Math.min(...ys(aa, 0)));
    // The piece of headline: level, left to right, last, at the letters' height.
    expect(new Set(ys(aa, 1))).toEqual(new Set([585]));
    expect(xs(aa, 1)).toEqual([...xs(aa, 1)].sort((a, b) => a - b));
  });


  it("ो draws its stem down, then lifts for the flag from its upper-left tip, as े is drawn", () => {
    const o = sign("ो");
    expect(labels(o)).toEqual([
      ["draw the stem straight down"],
      ["lift, then arc to the right along the top", "curve down to the top of the stem"],
    ]);
    expect(new Set(xs(o, 0)).size).toBe(1);
    expect(first(o, 0).y).toBe(Math.max(...ys(o, 0)));
    expect(last(o, 0).y).toBe(Math.min(...ys(o, 0)));
    // The flag: from its upper-left tip, right and down to where the stem began.
    expect(first(o, 1).x).toBe(Math.min(...xs(o, 1)));
    expect(last(o, 1).y).toBe(Math.min(...ys(o, 1)));
    expect(Math.hypot(last(o, 1).x - first(o, 0).x, last(o, 1).y - first(o, 0).y)).toBeLessThan(15);
    // Like े's run: it ends lower and to the right of where it began.
    expect(last(o, 1).x).toBeGreaterThan(first(o, 1).x);
    expect(last(o, 1).y).toBeLessThan(first(o, 1).y);
    // No movement runs along the headline stub.
    expect(Math.max(...xs(o, 0), ...xs(o, 1))).toBeLessThan(170);
  });

  it("ः draws the upper dot, lifts, then the lower dot, each a loop from its top, anticlockwise", () => {
    const visarga = sign("ः");
    expect(labels(visarga)).toEqual([
      ["start at the top and curve down the left side", "round the bottom and up the right side to close the loop"],
      ["lift, then down the lower dot's left side", "round the bottom and up the right side to close the loop"],
    ]);
    for (const stroke of [0, 1]) {
      expect(first(visarga, stroke).y).toBe(Math.max(...ys(visarga, stroke)));
      expect(last(visarga, stroke)).toEqual(first(visarga, stroke));
      expect(signedArea(penPath(visarga.strokes[stroke]))).toBeGreaterThan(0);
    }
    expect(Math.min(...ys(visarga, 0))).toBeGreaterThan(Math.max(...ys(visarga, 1)));
    // Neither loop reaches the headline stub above the dots.
    expect(Math.max(...ys(visarga, 0))).toBeLessThan(551);
  });

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
