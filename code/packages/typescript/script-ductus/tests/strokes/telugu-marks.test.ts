// Telugu vowel signs, the anusvara and the virama — each drawn by itself,
// with no consonant.
//
// Their pen lifts, start and direction are cited to native writers' tablet
// pen traces in HP Labs India's LipiTk Telugu recognizer (lipi-reco-indic-char
// 4.0.0, MIT model). The writers wrote each sign alone, so the traces say how
// the SIGN is written, not when it is written against its consonant: the book
// draws these signs only in lessons that teach the sign by itself, and
// composes no Telugu word from them. The underlying data is licensed for
// research use only, so only counts and shares are cited, and every path below
// is fitted to the bundled Noto Sans Telugu outline of the sign on its own, in
// font units with y pointing UP.
//
//     sign  class  strokes (share)  start                  then
//     ----  -----  ---------------  ---------------------  ---------------------------------------
//     ం     14     1 (103/104)      top of the ring        anticlockwise round it (103/103)
//     ా     52     1 (308/312)      left end of the bar    right along the bar (302/308 start left),
//                                                          the loop clockwise (190/308; 117 the
//                                                          other way)
//     ి     53     1 (204/205)      tail's lower-left tip  anticlockwise round the loop (201/204),
//                                                          curling in to end inside it
//     ీ     54     1 (180/213)      tail's lower-left tip  ి's loop, then the hook on top (150/180
//                                                          reach the top in the second half)
//     ు     55     1 (405/416)      lower-left tip         down round the bowl, up to the upper tip
//                                                          (403/405 start left, 370 end at the top)
//     ూ     56     1 (482/517)      lower-left tip         ు, the bar, the loop clockwise (273/482;
//                                                          200 the other way)
//     ె     57     1 (210/210)      lower tip              round the right, back left along the bar
//                                                          (anticlockwise 207/210)
//     ే     58     2 (163/206)      lower tip              ె first (159/163); lift; the hook from its
//                                                          foot, clockwise (159/159)
//     ొ     60     1 (294/303)      foot of the left bowl  up and over, the dip, the second arch, the
//                                                          loop clockwise (278/294)
//     ో     61     1 (301/320)      foot of the left bowl  ొ, then up out of the loop into the hook
//                                                          (222/301 end at the top)
//     ్     62     1 (101/104)      right end, lower bar   clockwise up through both bowls (98/101),
//                                                          out along the top bar
//
// Left out, so its lesson stays undrawn: ై. The recognizer's ai class (59)
// stores only the length mark below (ౖ: a loop and a tail to the right), never
// the e hook above it, so nothing says in which order the two parts are
// written. ృ and ౌ have no class in the recognizer at all.
import { createHash } from "node:crypto";
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

const sign = (glyph: string): LetterDuctus => DUCTUS[ductusKey("telugu", glyph)];
const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const first = (letter: LetterDuctus, stroke = 0): Point => letter.strokes[stroke].segments[0].path[0];
const last = (letter: LetterDuctus, stroke = letter.strokes.length - 1): Point =>
  letter.strokes[stroke].segments.at(-1)!.path.at(-1)!;
const all = (letter: LetterDuctus): Point[] => letter.strokes.flatMap((stroke) => penPath(stroke));
const xs = (letter: LetterDuctus): number[] => all(letter).map((p) => p.x);
const ys = (letter: LetterDuctus): number[] => all(letter).map((p) => p.y);
const segment = (letter: LetterDuctus, index: number, stroke = 0): Point[] =>
  letter.strokes[stroke].segments[index].path;
const labels = (letter: LetterDuctus): string[][] =>
  letter.strokes.map((stroke) => stroke.segments.map((entry) => entry.label));
/**
 * Twice the signed area of a pen path closed back to its start; positive
 * means it turns anticlockwise on the page (y up).
 */
const signedArea = (open: Point[]): number => {
  const path = [...open, open[0]];
  return path.slice(1).reduce((sum, b, index) => sum + path[index].x * b.y - b.x * path[index].y, 0);
};
/** The index, in the stroke's pen path, of its highest / rightmost point. */
const indexOfMax = (points: Point[], key: "x" | "y"): number =>
  points.reduce((best, point, index) => (point[key] > points[best][key] ? index : best), 0);

// The exact data each sign owns. A deliberate path change moves its hash here.
const HASHES: Record<string, string> = {
  "ం": "1d2abe6c0e88fcd510877a68ece90da208155ddd198ea750e30320f9564d6580",
  "ా": "7d7ecdd47fc212333a25b188294aefb700dc145969ab0174d7e91c0281fa067c",
  "ి": "b09cbe39c8146629be3e78fc61e8a7cf8039ba549723dc0222087c17f1afe7a0",
  "ీ": "09f9d40f6ec0eccd7b6ba2f3813d3aedbe759313e697ac45b5ff6d23a0545857",
  "ు": "25532cc241a3ab742456104b717600b49e7fd3a4a4ee9731fa6da195167b0e37",
  "ూ": "7bd3fbe1adef8195bc7bcc69b0bdfbe063796e7f8419751c45eb7958f1190e11",
  "ె": "c7ead23f0cfd57552a24b746c2274c22e3944ea4355b7ed9e1dcdd15283b3201",
  "ే": "1c94a996926b9e587282b6aa7fa92296110489763baea93923a5e3d59e38e378",
  "ొ": "69d0cfa4995c7813d18ff224476dee19d6bb54a7dc47fa4fa7c50e5e81cc8a4e",
  "ో": "b2778abcb459d094f86b1391d42ca759be3477a6419265a20a57a0a46a87f1dd",
  "్": "f1be4488e7a604b9723eb0fc2936721daad4aca605c4a8849ea22c9a0f689bb9",
};

const CLASSES: Record<string, number> = {
  "ం": 14, "ా": 52, "ి": 53, "ీ": 54, "ు": 55, "ూ": 56, "ె": 57, "ే": 58, "ొ": 60, "ో": 61, "్": 62,
};

const LIFTS: Record<string, number> = {
  "ం": 0, "ా": 0, "ి": 0, "ీ": 0, "ు": 0, "ూ": 0, "ె": 0, "ే": 1, "ొ": 0, "ో": 0, "్": 0,
};

describe("Telugu sign ductus records", () => {
  beforeAll(() => {
    for (const glyph of Object.keys(HASHES)) {
      expect(verifiedLetterFont(glyph, sign(glyph).source.url)).toBe("_fonts/NotoSansTelugu-Static.ttf");
    }
  });

  for (const glyph of Object.keys(HASHES)) {
    it(`${glyph} is a Telugu owner with its cited lifts and exact data`, () => {
      const letter = sign(glyph);
      expect(letter.script).toBe("telugu");
      expect(letter.glyph).toBe(glyph);
      expect(penLifts(letter)).toBe(LIFTS[glyph]);
      expect(sha256(JSON.stringify(letter))).toBe(HASHES[glyph]);
    });

    it(`${glyph}'s order traces to native writers in HP Labs India's LipiTk recognizer`, () => {
      const source = sign(glyph).source;
      expect(source.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
      expect(source.citation).toMatch(
        new RegExp(
          `^HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4\\.0, Telugu recognizer, ` +
            `class ${CLASSES[glyph]} \\(${glyph}, [^)]+\\): stored online-handwriting prototypes ` +
            `from native writers \\(MIT licence, 2012\\)$`,
          "u",
        ),
      );
      expect(source.variation).toMatch(
        /^The recognizer stores native writers' tablet pen traces.*\d+ of the \d+ stored prototypes|All \d+ stored prototypes/,
      );
      expect(source.variation).toMatch(
        /written by itself, with no consonant beside it.*composes no Telugu word from it.*Noto Sans Telugu outline of the sign on its own.*research use only, so only counts and shares are cited.*varies by writer\.$/,
      );
    });
  }

  it("leaves out ై, whose prototypes hold only its lower part, and the signs the recognizer lacks", () => {
    for (const glyph of ["ై", "ృ", "ౌ"]) {
      expect(DUCTUS[ductusKey("telugu", glyph)], glyph).toBeUndefined();
    }
  });
});

describe("one-stroke signs", () => {
  it("ం is one ring from its top, anticlockwise, that closes where it began", () => {
    const anusvara = sign("ం");
    expect(labels(anusvara)).toEqual([
      ["start at the top of the ring and curve down its left side", "round the bottom and come up the right side to close the ring"],
    ]);
    expect(first(anusvara).y).toBe(Math.max(...ys(anusvara)));
    expect(last(anusvara)).toEqual(first(anusvara));
    expect(signedArea(all(anusvara))).toBeGreaterThan(0);
  });

  it("ా draws the bar from its left end, then turns the loop clockwise to the tip under the bar", () => {
    const aa = sign("ా");
    expect(labels(aa)).toEqual([
      [
        "start at the left end of the bar and draw it to the right",
        "curve down the right side of the loop",
        "round the bottom and come up the left side to the tip under the bar",
      ],
    ]);
    expect(first(aa).x).toBe(Math.min(...xs(aa)));
    const bar = segment(aa, 0);
    expect(Math.max(...bar.map((p) => p.y)) - Math.min(...bar.map((p) => p.y))).toBeLessThan(20);
    expect(bar.at(-1)!.x).toBeGreaterThan(first(aa).x + 500);
    // The loop: down the right side first, then back up the left — clockwise.
    const loop = [...segment(aa, 1), ...segment(aa, 2)];
    expect(signedArea(loop)).toBeLessThan(0);
    expect(last(aa).x).toBeLessThan(Math.max(...xs(aa)) - 100);
    expect(last(aa).y).toBeLessThan(first(aa).y);
  });

  it("ి starts at the tail's lower-left tip, turns anticlockwise and ends inside the loop", () => {
    const i = sign("ి");
    expect(labels(i)).toEqual([
      [
        "start at the tail's lower-left tip and run right along the bottom",
        "climb the right side and arch over the top to the left",
        "come down the left side and curl in to the tip inside the loop",
      ],
    ]);
    expect(first(i).x).toBeLessThan(-370);
    expect(first(i).y).toBeLessThan(500);
    expect(signedArea(all(i))).toBeGreaterThan(0);
    // The curl ends inside the loop, clear of every edge of the sign.
    expect(last(i).x).toBeGreaterThan(Math.min(...xs(i)) + 150);
    expect(last(i).x).toBeLessThan(Math.max(...xs(i)));
    expect(last(i).y).toBeGreaterThan(Math.min(...ys(i)) + 150);
    expect(last(i).y).toBeLessThan(Math.max(...ys(i)) - 50);
  });

  it("ీ draws ి's loop first, then runs back to the hook and ends at its tip", () => {
    const ii = sign("ీ");
    expect(labels(ii)[0].slice(0, 3)).toEqual(labels(sign("ి"))[0]);
    expect(labels(ii)[0].slice(3)).toEqual([
      "run back along the top of the loop to the foot of the hook",
      "climb the hook's left side, over the top and down to its tip",
    ]);
    const loop = [...segment(ii, 0), ...segment(ii, 1), ...segment(ii, 2)];
    expect(signedArea(loop)).toBeGreaterThan(0);
    // The highest point is on the hook, drawn last, and the stroke ends to
    // the right of it, at the hook's tip.
    const path = penPath(ii.strokes[0]);
    const top = indexOfMax(path, "y");
    expect(top).toBeGreaterThan(path.length / 2);
    expect(Math.max(...segment(ii, 4).map((p) => p.y))).toBe(Math.max(...ys(ii)));
    expect(last(ii).x).toBeGreaterThan(path[top].x);
    expect(Math.max(...loop.map((p) => p.y))).toBeLessThan(Math.max(...ys(ii)) - 100);
  });

  it("ు starts at the lower-left tip, passes the bottom of the bowl and ends at the upper tip", () => {
    const u = sign("ు");
    expect(labels(u)).toEqual([
      [
        "start at the lower-left tip and curve down to the right",
        "round the bowl and climb its right side",
        "curve up and to the left, to the upper tip",
      ],
    ]);
    expect(first(u).x).toBe(Math.min(...xs(u)));
    expect(last(u).y).toBe(Math.max(...ys(u)));
    const path = all(u);
    const bottom = path.findIndex((p) => p.y === Math.min(...ys(u)));
    expect(bottom).toBeGreaterThan(0);
    expect(bottom).toBeLessThan(path.length - 1);
  });

  it("ూ draws ు first, then the bar and the loop on the right, clockwise", () => {
    const uu = sign("ూ");
    expect(labels(uu)).toEqual([
      [
        "start at the lower-left tip and curve down to the right",
        "round the bowl and climb its right side",
        "curve up and to the left, to the top of the bowl",
        "draw the bar to the right along the top",
        "round the loop on the right, down, under and up, to the tip under the bar",
      ],
    ]);
    expect(segment(uu, 0)).toEqual(segment(sign("ు"), 0));
    expect(segment(uu, 1)).toEqual(segment(sign("ు"), 1));
    expect(first(uu).x).toBe(Math.min(...xs(uu)));
    expect(Math.max(...segment(uu, 4).map((p) => p.x))).toBe(Math.max(...xs(uu)));
    expect(signedArea(segment(uu, 4))).toBeLessThan(0);
  });

  it("ె runs from its lower tip round the right and back left along the top bar", () => {
    const e = sign("ె");
    expect(labels(e)).toEqual([
      ["start at the lower tip and curve to the right", "curve up and back to the left along the top bar"],
    ]);
    expect(first(e).y).toBe(Math.min(...ys(e)));
    expect(last(e).x).toBe(Math.min(...xs(e)));
    expect(last(e).y).toBeGreaterThan(first(e).y + 150);
    expect(signedArea(all(e))).toBeGreaterThan(0);
  });

  it("ొ starts at the foot of the left bowl and ends inside the loop on the right, turned clockwise", () => {
    const o = sign("ొ");
    expect(labels(o)).toEqual([
      [
        "start at the foot of the left bowl and curve round its left side",
        "climb over the top and down into the dip",
        "rise over the second arch and along the top",
        "round the loop on the right, down, under and up, to the tip under the bar",
      ],
    ]);
    expect(first(o).x).toBeLessThan(Math.min(...xs(o)) + (Math.max(...xs(o)) - Math.min(...xs(o))) / 3);
    expect(first(o).y).toBe(Math.min(...segment(o, 0).map((p) => p.y)));
    // The dip between the arches is lower than both arches.
    const dip = segment(o, 1).at(-1)!;
    expect(dip.y).toBeLessThan(Math.max(...segment(o, 1).map((p) => p.y)) - 50);
    expect(dip.y).toBeLessThan(Math.max(...segment(o, 2).map((p) => p.y)) - 50);
    expect(Math.max(...segment(o, 3).map((p) => p.x))).toBe(Math.max(...xs(o)));
    expect(signedArea(segment(o, 3))).toBeLessThan(0);
  });

  it("ో is ొ's run, then up out of the loop into the hook, ending at its tip", () => {
    const oo = sign("ో");
    expect(labels(oo)[0].slice(0, 3)).toEqual(labels(sign("ొ"))[0].slice(0, 3));
    expect(labels(oo)[0].slice(3)).toEqual([
      "round the loop on the right, down, under and up its left side",
      "climb into the hook, over its top and down to its tip",
    ]);
    for (const index of [0, 1, 2]) expect(segment(oo, index)).toEqual(segment(sign("ొ"), index));
    expect(signedArea(segment(oo, 3))).toBeLessThan(0);
    expect(Math.max(...segment(oo, 4).map((p) => p.y))).toBe(Math.max(...ys(oo)));
    expect(last(oo).x).toBeGreaterThan(first(oo).x + 500);
    expect(last(oo).y).toBeGreaterThan(Math.max(...segment(oo, 3).map((p) => p.y)));
  });

  it("్ starts at the lower bar, turns clockwise through both bowls and ends at the top bar's right end", () => {
    const virama = sign("్");
    expect(labels(virama)).toEqual([
      [
        "start at the right end of the lower bar and curve left round the lower bowl",
        "run out along the middle prong and back",
        "curve up round the upper bowl",
        "draw the top bar to the right",
      ],
    ]);
    expect(first(virama).y).toBe(Math.min(...ys(virama)));
    expect(last(virama).x).toBe(Math.max(...xs(virama)));
    expect(last(virama).y).toBeGreaterThan(750);
    expect(signedArea(all(virama))).toBeLessThan(0);
    // The prong is run out and back: it ends near where it began.
    const prong = segment(virama, 1);
    expect(Math.max(...prong.map((p) => p.x))).toBeGreaterThan(prong[0].x + 150);
    expect(Math.hypot(prong.at(-1)!.x - prong[0].x, prong.at(-1)!.y - prong[0].y)).toBeLessThan(60);
  });
});

describe("the sign written in two strokes", () => {
  it("ే draws ె first, lifts, then the hook from its foot on the bar, clockwise to its tip", () => {
    const ee = sign("ే");
    expect(labels(ee)).toEqual([
      ["start at the lower tip and curve to the right", "curve up and back to the left along the top bar"],
      ["lift, then start at the hook's foot on the bar and climb to the left", "arch over the top and down to the hook's tip"],
    ]);
    // Noto prints the same e hook under both signs, so the first stroke is ె's.
    expect(ee.strokes[0]).toEqual(sign("ె").strokes[0]);
    const hook = penPath(ee.strokes[1]);
    expect(Math.min(...hook.map((p) => p.y))).toBeGreaterThan(600);
    expect(last(ee, 1).x).toBeGreaterThan(first(ee, 1).x);
    expect(signedArea(hook)).toBeLessThan(0);
    expect(Math.max(...hook.map((p) => p.y))).toBe(Math.max(...ys(ee)));
  });
});
