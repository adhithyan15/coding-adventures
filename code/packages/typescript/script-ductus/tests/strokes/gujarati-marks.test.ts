// Gujarati vowel signs, the anusvara and the visarga — each drawn by itself,
// without the consonant it attaches to.
//
// Their order, start, direction and pen lifts are cited to KanoAI's hand-made
// Gujarati barakhadi templates (one centre line per pen-down run, animated in
// the author's handwriting order). KanoAI's licence is ambiguous (MIT in its
// LICENSE file, GNU GPL in its README), so only those facts are used: every
// path below is fitted to the bundled Noto Sans Gujarati outline of the sign
// on its own, in font units with y pointing UP.
//
//     sign   runs   start                          then
//     ----   ----   -----------------------------  ------------------------------
//     ા      1      top of the bar                 down, into a foot to the right
//     િ      1      right tip of the top hook      up and over to the left, down the stem, foot
//     ી      1      left tip of the top hook       up and over to the right, down the stem, foot
//     ુ      1      under the consonant, at top    right, round the bowl, up to the left tip
//     ૂ      1      low inner tip                  left, up over the top, down to the right
//     ે      1      upper-left tip                 right, then down to the right
//     ૈ      2      lower flag, then upper flag    each from its upper-left tip, down to the right
//     ો      2      the ા bar, then the flag       as above
//     ૌ      3      bar, lower flag, upper flag    as above
//     ં      1      top of the dot                 anticlockwise round it
//     ઃ      2      upper dot, then lower dot      each from its top, anticlockwise
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

const sign = (glyph: string): LetterDuctus => DUCTUS[ductusKey("gujarati", glyph)];
const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const first = (letter: LetterDuctus, stroke = 0): Point => letter.strokes[stroke].segments[0].path[0];
const last = (letter: LetterDuctus, stroke = letter.strokes.length - 1): Point =>
  letter.strokes[stroke].segments.at(-1)!.path.at(-1)!;
const xs = (letter: LetterDuctus, stroke = 0): number[] => penPath(letter.strokes[stroke]).map((p) => p.x);
const ys = (letter: LetterDuctus, stroke = 0): number[] => penPath(letter.strokes[stroke]).map((p) => p.y);
const labels = (letter: LetterDuctus): string[][] =>
  letter.strokes.map((stroke) => stroke.segments.map((segment) => segment.label));
/** Twice the signed area of a closed pen path; positive means anticlockwise (y up). */
const signedArea = (path: Point[]): number =>
  path.slice(1).reduce((sum, b, index) => sum + path[index].x * b.y - b.x * path[index].y, 0);

// The exact data each sign owns. A deliberate path change moves its hash here.
const HASHES: Record<string, string> = {
  "ા": "2ba9da1e0f90c86b6dd0c361308c1d6f88ab3b9509aa6e0361b33c5799aebe69",
  "િ": "062eb98219eaccd6987bb5a690863e2a6680be6ba0380c03fd220a621ebf5458",
  "ી": "29ce10df48787fe11153aea58bbb55f5170858e7f5992a4500b925e37836ad2e",
  "ુ": "4314c34ef7c86ca322f49c992cb3540faf4e0350bc464771dcf302111eae83c2",
  "ૂ": "f9a338c0315d7a32a18e1a6f95a73502e6a737f3a0a077dea37358a13a1f4b19",
  "ે": "744d372740656cdd9e62c58ec235a1b505d46edb5bdfa2c5a700c138bc6b55e6",
  "ૈ": "5916181ca17d12d703c172553387c7926b948f1b4eed3c212be4a42663e62bbf",
  "ો": "27ef31391cfaadc39c0925ba6dfb31f65054e4a854ea147474676cff18a3fb62",
  "ૌ": "64f27a8eac773dd53ea9eb939232cb79ee3aa65c385aae3169b100d862c608fc",
  "ં": "0fcf302376f24c9d5b37382164cd80308806164cf6bfd9f0a15a09a265354a01",
  "ઃ": "38cbc39d1e5ce02dd0f564d3b1848ef9d961da2245612a94aec8626fdbc67e05",
};

const LIFTS: Record<string, number> = {
  "ા": 0, "િ": 0, "ી": 0, "ુ": 0, "ૂ": 0, "ે": 0, "ૈ": 1, "ો": 1, "ૌ": 2, "ં": 0, "ઃ": 1,
};

describe("Gujarati sign ductus records", () => {
  for (const glyph of Object.keys(HASHES)) {
    it(`${glyph} is a Gujarati owner with its cited lifts and exact data`, () => {
      const letter = sign(glyph);
      expect(letter.script).toBe("gujarati");
      expect(letter.glyph).toBe(glyph);
      expect(penLifts(letter)).toBe(LIFTS[glyph]);
      expect(sha256(JSON.stringify(letter))).toBe(HASHES[glyph]);
    });

    it(`${glyph}'s order traces to KanoAI's barakhadi templates, facts only`, () => {
      const source = sign(glyph).source;
      expect(source.url).toBe(
        "https://github.com/gajjartejas/KanoAI/tree/9d3e2949a3f265448e430e15329093ea3aa516b8/interpolate-svg/svgs/barakhadi",
      );
      expect(source.citation).toMatch(
        /^Tejas Gajjar, KanoAI, Gujarati barakhadi centre-line stroke templates .*commit 9d3e294, 1_k\/\d+_k\w+\.svg, groups? g1.*drawn after the consonant/,
      );
      expect(source.variation).toMatch(
        /consonant rows.*hand-made centre lines.*LICENSE file says MIT while its README says GNU GPL.*no template path data was copied.*Noto Sans Gujarati.*varies by writer/,
      );
    });
  }
});

describe("one-run signs", () => {
  it("ા draws the bar down from the top and turns into a foot to the right", () => {
    const aa = sign("ા");
    expect(labels(aa)).toEqual([["draw the bar down", "turn into the foot to the right"]]);
    expect(first(aa).y).toBe(Math.max(...ys(aa)));
    expect(last(aa).x).toBe(Math.max(...xs(aa)));
    expect(last(aa).y).toBeLessThan(60);
  });

  it("િ starts at the hook's RIGHT tip, curls up and over to the left, then comes down", () => {
    const i = sign("િ");
    expect(labels(i)).toEqual([
      ["curl up and over to the left from the hook's tip", "draw the stem down", "turn into the foot to the right"],
    ]);
    expect(first(i).x).toBe(Math.max(...xs(i)));
    const hook = i.strokes[0].segments[0].path;
    expect(Math.max(...hook.map((p) => p.y))).toBeGreaterThan(first(i).y + 150);
    expect(hook.at(-1)!.x).toBeLessThan(first(i).x - 400);
    expect(last(i).y).toBeLessThan(60);
  });

  it("ી starts at the hook's LEFT tip and arches over to the right before the stem", () => {
    const ii = sign("ી");
    expect(labels(ii)).toEqual([
      ["rise from the hook's tip and arch over to the right", "draw the stem down", "turn into the foot to the right"],
    ]);
    expect(first(ii).x).toBe(Math.min(...xs(ii)));
    expect(ii.strokes[0].segments[0].path.at(-1)!.x).toBeGreaterThan(first(ii).x + 250);
    expect(last(ii).x).toBe(Math.max(...xs(ii)));
    expect(last(ii).y).toBeLessThan(60);
  });

  it("ુ starts at its top, swings right round the bowl, and ends at the left tip", () => {
    const u = sign("ુ");
    expect(labels(u)).toEqual([
      ["start under the consonant and swing to the right", "round the bowl and back to the left", "sweep up to the left tip"],
    ]);
    expect(first(u).y).toBeGreaterThan(-70);
    expect(u.strokes[0].segments[0].path.at(-1)!.x).toBeGreaterThan(first(u).x + 150);
    expect(last(u).x).toBe(Math.min(...xs(u)));
  });

  it("ૂ starts low inside, curls left and over the top, and sweeps down to the right", () => {
    const uu = sign("ૂ");
    expect(labels(uu)).toEqual([
      ["start low and curl to the left", "climb and arch over the top", "sweep down to the right"],
    ]);
    expect(first(uu).y).toBeLessThan(-200);
    expect(uu.strokes[0].segments[0].path.at(-1)!.x).toBeLessThan(first(uu).x - 150);
    expect(last(uu).x).toBe(Math.max(...xs(uu)));
    expect(last(uu).y).toBeLessThan(-150);
  });

  it("ે runs from its upper-left tip to the right and down", () => {
    const e = sign("ે");
    expect(labels(e)).toEqual([["arc to the right along the top", "curve down to the right"]]);
    expect(first(e).x).toBe(Math.min(...xs(e)));
    expect(last(e).x).toBe(Math.max(...xs(e)));
    expect(last(e).y).toBe(Math.min(...ys(e)));
  });

  it("ં is one small loop from its top, anticlockwise, that closes where it began", () => {
    const anusvara = sign("ં");
    expect(labels(anusvara)).toEqual([
      ["start at the top and curve down the left side", "round the bottom and up the right side to close the loop"],
    ]);
    expect(first(anusvara).y).toBe(Math.max(...ys(anusvara)));
    expect(last(anusvara)).toEqual(first(anusvara));
    expect(signedArea(penPath(anusvara.strokes[0]))).toBeGreaterThan(0);
  });
});

describe("signs written in several runs", () => {
  const flagRunsDownRight = (letter: LetterDuctus, stroke: number): void => {
    expect(first(letter, stroke).x).toBe(Math.min(...xs(letter, stroke)));
    expect(last(letter, stroke).x).toBe(Math.max(...xs(letter, stroke)));
    expect(last(letter, stroke).y).toBe(Math.min(...ys(letter, stroke)));
  };

  it("ૈ draws the lower flag first, then the upper one", () => {
    const ai = sign("ૈ");
    expect(ai.strokes).toHaveLength(2);
    expect(first(ai, 0).y).toBeLessThan(first(ai, 1).y);
    flagRunsDownRight(ai, 0);
    flagRunsDownRight(ai, 1);
  });

  it("ો draws the ા bar first, then the flag above it", () => {
    const o = sign("ો");
    expect(o.strokes).toHaveLength(2);
    expect(o.strokes[0]).toEqual(sign("ા").strokes[0]);
    expect(Math.min(...ys(o, 1))).toBeGreaterThan(Math.max(...ys(o, 0)));
    flagRunsDownRight(o, 1);
  });

  it("ૌ draws the bar, then the lower flag, then the upper flag", () => {
    const au = sign("ૌ");
    expect(au.strokes).toHaveLength(3);
    expect(au.strokes[0]).toEqual(sign("ા").strokes[0]);
    expect(first(au, 1).y).toBeLessThan(first(au, 2).y);
    flagRunsDownRight(au, 1);
    flagRunsDownRight(au, 2);
  });

  it("ઃ draws the upper dot first, then the lower, each from its top, anticlockwise", () => {
    const visarga = sign("ઃ");
    expect(visarga.strokes).toHaveLength(2);
    expect(Math.min(...ys(visarga, 0))).toBeGreaterThan(Math.max(...ys(visarga, 1)));
    for (const stroke of [0, 1]) {
      expect(first(visarga, stroke).y).toBe(Math.max(...ys(visarga, stroke)));
      expect(last(visarga, stroke)).toEqual(first(visarga, stroke));
      expect(signedArea(penPath(visarga.strokes[stroke]))).toBeGreaterThan(0);
    }
  });
});
