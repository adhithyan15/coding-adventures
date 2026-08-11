// ---------------------------------------------------------------------------
// ductusview.test.ts — does the picture say what the data says?
// ---------------------------------------------------------------------------
//
// `strokes.test.ts` already proves the pen path is TRUE: every point on real
// ink, every join tight, the whole letter traced. That leaves exactly one thing
// for this file to prove — that the picture is a faithful rendering of that
// already-true data. Concretely:
//
//   • the glyph outline drawn is the FONT's path, character for character;
//   • the letter and the pen share ONE flip, so they cannot disagree on "up";
//   • the frames advance — each shows strictly more of the stroke than the last;
//   • a letter with no authored ductus produces nothing, rather than a guess;
//   • anything that reaches an attribute is escaped.
// ---------------------------------------------------------------------------

import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parseFont, boundsOf } from "../src/truetype";
import { DUCTUS, penPathD, type LetterDuctus } from "../src/strokes";
import {
  ductusFilmstrip,
  ductusFor,
  ductusFrame,
  ductusSteps,
  escapeXml,
  isSafeName,
  segmentEndFractions,
  svgMarkup,
  viewBoxFor,
  wrapCaption,
  type GlyphOutline,
  type SvgNode,
} from "../src/ductusview";

const FONT_DIR = resolve(__dirname, "../../../../learning/human-languages/_fonts");
const load = (name: string) => {
  const b = readFileSync(resolve(FONT_DIR, name));
  return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer;
};

// The glyph shape comes from the shipped font, exactly as the app gets it.
// Nothing in this file draws a letter.
const tamilOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoSansTamil-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const naskhOutline = (character: string): GlyphOutline => {
  const g = parseFont(load("NotoNaskhArabic-Static.ttf")).glyphFor(character)!;
  return { path: g.path, bounds: boundsOf(g.contours) };
};

const MA = DUCTUS["ம"];
const outline = tamilOutline("ம");
const A = DUCTUS["அ"];
const aOutline = tamilOutline("அ");
const AA = DUCTUS["ஆ"];
const aaOutline = tamilOutline("ஆ");
const I = DUCTUS["இ"];
const iOutline = tamilOutline("இ");
const KA = DUCTUS["க"];
const kaOutline = tamilOutline("க");
const VA = DUCTUS["வ"];
const vaOutline = tamilOutline("வ");
const LA = DUCTUS["ல"];
const laOutline = tamilOutline("ல");
const RRA = DUCTUS["ற"];
const rraOutline = tamilOutline("ற");
const NNA = DUCTUS["ன"];
const nnaOutline = tamilOutline("ன");
const RETROFLEX_NNA = DUCTUS["ண"];
const retroflexNnaOutline = tamilOutline("ண");
const DENTAL_NA = DUCTUS["ந"];
const dentalNaOutline = tamilOutline("ந");
const PERSIAN_ALEF = DUCTUS["ا"];
const persianAlefOutline = naskhOutline("ا");
const ARABIC_ALEF = ductusFor("ا", "arabic")!;
const arabicAlefOutline = naskhOutline("ا");
const ARABIC_BAA = ductusFor("ب", "arabic")!;
const arabicBaaOutline = naskhOutline("ب");
const ARABIC_TAA = ductusFor("ت", "arabic")!;
const arabicTaaOutline = naskhOutline("ت");
const ARABIC_JEEM = ductusFor("ج", "arabic")!;
const arabicJeemOutline = naskhOutline("ج");
const ARABIC_HAA = ductusFor("ح", "arabic")!;
const arabicHaaOutline = naskhOutline("ح");
const ARABIC_KHAA = ductusFor("خ", "arabic")!;
const arabicKhaaOutline = naskhOutline("خ");
const ARABIC_DAAL = ductusFor("د", "arabic")!;
const arabicDaalOutline = naskhOutline("د");
const ARABIC_RAA = ductusFor("ر", "arabic")!;
const arabicRaaOutline = naskhOutline("ر");
const ARABIC_SEEN = ductusFor("س", "arabic")!;
const arabicSeenOutline = naskhOutline("س");
const URDU_ALEF = ductusFor("ا", "urdu-nastaliq")!;
const urduAlefOutline = naskhOutline("ا");
const URDU_JIM = ductusFor("ج", "urdu-nastaliq")!;
const urduJimOutline = naskhOutline("ج");
const URDU_RE = ductusFor("ر", "urdu-nastaliq")!;
const urduReOutline = naskhOutline("ر");
const URDU_SIN = ductusFor("س", "urdu-nastaliq")!;
const urduSinOutline = naskhOutline("س");
const URDU_SHIN = ductusFor("ش", "urdu-nastaliq")!;
const urduShinOutline = naskhOutline("ش");
const URDU_KAF = ductusFor("ک", "urdu-nastaliq")!;
const urduKafOutline = naskhOutline("ک");
const URDU_LAM = ductusFor("ل", "urdu-nastaliq")!;
const urduLamOutline = naskhOutline("ل");
const URDU_MIM = ductusFor("م", "urdu-nastaliq")!;
const urduMimOutline = naskhOutline("م");
const URDU_NUN = ductusFor("ن", "urdu-nastaliq")!;
const urduNunOutline = naskhOutline("ن");
const URDU_GHUNNA = ductusFor("ں", "urdu-nastaliq")!;
const urduGhunnaOutline = naskhOutline("ں");
const URDU_HE = ductusFor("ہ", "urdu-nastaliq")!;
const urduHeOutline = naskhOutline("ہ");
const URDU_YE = ductusFor("ی", "urdu-nastaliq")!;
const urduYeOutline = naskhOutline("ی");
const URDU_BARI_YE = ductusFor("ے", "urdu-nastaliq")!;
const urduBariYeOutline = naskhOutline("ے");
const PERSIAN_BEH = DUCTUS["ب"];
const persianBehOutline = naskhOutline("ب");
const PERSIAN_TEH = DUCTUS["ت"];
const persianTehOutline = naskhOutline("ت");
const PERSIAN_SIN = DUCTUS["س"];
const persianSinOutline = naskhOutline("س");
const PERSIAN_LAM = DUCTUS["ل"];
const persianLamOutline = naskhOutline("ل");
const PERSIAN_MIM = DUCTUS["م"];
const persianMimOutline = naskhOutline("م");
const PERSIAN_NUN = DUCTUS["ن"];
const persianNunOutline = naskhOutline("ن");
const PERSIAN_WAW = DUCTUS["و"];
const persianWawOutline = naskhOutline("و");
const PERSIAN_HEH = DUCTUS["ه"];
const persianHehOutline = naskhOutline("ه");

/** Walk a node tree, collecting every node the predicate accepts. */
function collect(node: SvgNode, pick: (n: SvgNode) => boolean, out: SvgNode[] = []): SvgNode[] {
  if (pick(node)) out.push(node);
  for (const c of node.children ?? []) collect(c, pick, out);
  return out;
}

const byTag = (node: SvgNode, tag: string) => collect(node, (n) => n.tag === tag);

describe("ductusFor — only cited letters have a ductus", () => {
  it("finds eleven Tamil letters, nine Persian letters, nine Arabic letters, and thirteen Urdu letters", () => {
    expect(ductusFor("ம")?.glyph).toBe("ம");
    expect(ductusFor("அ")?.glyph).toBe("அ");
    expect(ductusFor("ஆ")?.glyph).toBe("ஆ");
    expect(ductusFor("இ")?.glyph).toBe("இ");
    expect(ductusFor("க")?.glyph).toBe("க");
    expect(ductusFor("வ")?.glyph).toBe("வ");
    expect(ductusFor("ல")?.glyph).toBe("ல");
    expect(ductusFor("ற")?.glyph).toBe("ற");
    expect(ductusFor("ன")?.glyph).toBe("ன");
    expect(ductusFor("ண")?.glyph).toBe("ண");
    expect(ductusFor("ந")?.glyph).toBe("ந");
    expect(ductusFor("ا")?.glyph).toBe("ا");
    expect(ductusFor("ب")?.glyph).toBe("ب");
    expect(ductusFor("ت")?.glyph).toBe("ت");
    expect(ductusFor("س")?.glyph).toBe("س");
    expect(ductusFor("ل")?.glyph).toBe("ل");
    expect(ductusFor("م")?.glyph).toBe("م");
    expect(ductusFor("ن")?.glyph).toBe("ن");
    expect(ductusFor("و")?.glyph).toBe("و");
    expect(ductusFor("ه")?.glyph).toBe("ه");
    expect(ductusFor("ا", "arabic")?.glyph).toBe("ا");
    expect(ductusFor("ب", "arabic")?.glyph).toBe("ب");
    expect(ductusFor("س", "arabic")?.glyph).toBe("س");
    expect(ductusFor("ا", "urdu-nastaliq")?.glyph).toBe("ا");
    expect(ductusFor("ج", "urdu-nastaliq")?.glyph).toBe("ج");
    expect(ductusFor("ج", "perso-arabic")).toBeUndefined();
    expect(ductusFor("ر", "urdu-nastaliq")?.glyph).toBe("ر");
    expect(ductusFor("ر", "perso-arabic")).toBeUndefined();
    expect(ductusFor("س", "urdu-nastaliq")?.glyph).toBe("س");
    expect(ductusFor("ش", "urdu-nastaliq")?.glyph).toBe("ش");
    expect(ductusFor("ش", "perso-arabic")).toBeUndefined();
    expect(ductusFor("ک", "urdu-nastaliq")?.glyph).toBe("ک");
    expect(ductusFor("ک", "perso-arabic")).toBeUndefined();
    expect(ductusFor("ل", "urdu-nastaliq")?.glyph).toBe("ل");
    expect(ductusFor("ل", "perso-arabic")?.glyph).toBe("ل");
    expect(ductusFor("م", "urdu-nastaliq")?.glyph).toBe("م");
    expect(ductusFor("م", "perso-arabic")?.glyph).toBe("م");
    expect(ductusFor("ن", "urdu-nastaliq")?.glyph).toBe("ن");
    expect(ductusFor("ن", "perso-arabic")?.glyph).toBe("ن");
    expect(ductusFor("ں", "urdu-nastaliq")?.glyph).toBe("ں");
    expect(ductusFor("ہ", "urdu-nastaliq")?.glyph).toBe("ہ");
    expect(ductusFor("ی", "urdu-nastaliq")?.glyph).toBe("ی");
    expect(ductusFor("ے", "urdu-nastaliq")?.glyph).toBe("ے");
  });

  it("keeps the shared Arabic, Persian, and Urdu ا independently addressable", () => {
    const arabic = ductusFor("ا", "arabic");
    const persian = ductusFor("ا", "perso-arabic");
    const urdu = ductusFor("ا", "urdu-nastaliq");
    expect(arabic?.script).toBe("arabic");
    expect(persian?.script).toBe("perso-arabic");
    expect(urdu?.script).toBe("urdu-nastaliq");
    expect(arabic?.source.url).not.toBe(persian?.source.url);
    expect(arabic?.source.url).not.toBe(urdu?.source.url);
    expect(persian?.source.url).not.toBe(urdu?.source.url);
  });

  it("keeps the shared Arabic and Persian ب independently addressable", () => {
    const arabic = ductusFor("ب", "arabic");
    const persian = ductusFor("ب", "perso-arabic");
    expect(arabic?.script).toBe("arabic");
    expect(persian?.script).toBe("perso-arabic");
    expect(arabic?.source.url).not.toBe(persian?.source.url);
  });

  it("keeps the shared Arabic, Persian, and Urdu س independently addressable", () => {
    const arabic = ductusFor("س", "arabic");
    const persian = ductusFor("س", "perso-arabic");
    const urdu = ductusFor("س", "urdu-nastaliq");
    expect(arabic?.script).toBe("arabic");
    expect(persian?.script).toBe("perso-arabic");
    expect(urdu?.script).toBe("urdu-nastaliq");
    expect(arabic?.source.url).not.toBe(persian?.source.url);
    expect(arabic?.source.url).not.toBe(urdu?.source.url);
    expect(persian?.source.url).not.toBe(urdu?.source.url);
  });

  it("keeps the shared Persian and Urdu م independently addressable", () => {
    const persian = ductusFor("م", "perso-arabic");
    const urdu = ductusFor("م", "urdu-nastaliq");
    expect(persian?.script).toBe("perso-arabic");
    expect(urdu?.script).toBe("urdu-nastaliq");
    expect(persian?.source.url).not.toBe(urdu?.source.url);
    expect(ductusFor("م", "arabic")).toBeUndefined();
  });

  it("keeps the shared Persian and Urdu ن independently addressable", () => {
    const persian = ductusFor("ن", "perso-arabic");
    const urdu = ductusFor("ن", "urdu-nastaliq");
    expect(persian?.script).toBe("perso-arabic");
    expect(urdu?.script).toBe("urdu-nastaliq");
    expect(persian?.source.url).not.toBe(urdu?.source.url);
    expect(ductusFor("ن", "arabic")).toBeUndefined();
  });

  it("returns undefined for a letter nobody has authored a stroke order for", () => {
    // Persian پ is deferred inventory work, not a starter entry or authored
    // pen path. It must come back empty rather than borrow ب's or Tamil ம's.
    expect(ductusFor("پ")).toBeUndefined();
    expect(ductusFor("A")).toBeUndefined();
    expect(ductusFor("")).toBeUndefined();
  });

  it("does not mistake inherited Object properties for letters", () => {
    // DUCTUS is a plain object, so `DUCTUS["toString"]` is a FUNCTION, not a
    // letter. A naive lookup would hand that to the renderer and crash.
    expect(ductusFor("toString")).toBeUndefined();
    expect(ductusFor("constructor")).toBeUndefined();
  });
});

describe("segment fractions — where each part ends along its stroke", () => {
  const fractions = segmentEndFractions(MA.strokes[0]);

  it("has one entry per labelled part", () => {
    expect(fractions).toHaveLength(MA.strokes[0].segments.length);
  });

  it("ascends and finishes at the end of the stroke", () => {
    for (let i = 1; i < fractions.length; i++) {
      expect(fractions[i]).toBeGreaterThan(fractions[i - 1]);
    }
    expect(fractions[0]).toBeGreaterThan(0);
    expect(fractions[fractions.length - 1]).toBeCloseTo(1, 10);
  });

  it("a zero-length stroke reports every part already complete", () => {
    const flat = { segments: [{ label: "nowhere", path: [{ x: 5, y: 5 }, { x: 5, y: 5 }] }] };
    expect(segmentEndFractions(flat)).toEqual([1]);
  });
});

describe("ductusSteps — the frames, in writing order", () => {
  const steps = ductusSteps(MA);

  it("gives one step per labelled part, numbered from 1", () => {
    expect(steps).toHaveLength(5);
    expect(steps.map((s) => s.number)).toEqual([1, 2, 3, 4, 5]);
    expect(steps[0].label).toBe("down the left upright");
    expect(steps[4].label).toBe("down the middle");
  });

  it("marks ம as never lifting the pen", () => {
    expect(steps.every((s) => s.startsAfterLift === false)).toBe(true);
    expect(steps.every((s) => s.strokeIndex === 0)).toBe(true);
  });
});

describe("the drawn frame", () => {
  const frame = ductusFrame(MA, outline, ductusSteps(MA)[2]);

  it("draws the FONT's outline, not a redrawn one", () => {
    const glyphPath = byTag(frame, "path").find((p) => p.attrs.class === "ductus__glyph")!;
    expect(glyphPath.attrs.d).toBe(outline.path);
    // Sanity: that really is a font path — quadratics and closed contours.
    expect(String(glyphPath.attrs.d)).toMatch(/^M/);
    expect(String(glyphPath.attrs.d)).toContain("Z");
  });

  it("emits the pen path straight from penPathD at the step's fraction", () => {
    const step = ductusSteps(MA)[2];
    const pen = byTag(frame, "path").find((p) => p.attrs.class === "ductus__pen")!;
    expect(pen.attrs.d).toBe(penPathD(MA.strokes[0], step.fraction));
    expect(pen.attrs.fill).toBe("none");
  });

  it("puts a pen dot at the end of what has been drawn", () => {
    const dot = byTag(frame, "circle")[0];
    const d = String(byTag(frame, "path").find((p) => p.attrs.class === "ductus__pen")!.attrs.d);
    const last = d.trim().split(/(?=[ML])/).pop()!.slice(1).trim().split(/\s+/).map(Number);
    expect(Number(dot.attrs.cx)).toBeCloseTo(last[0], 1);
    expect(Number(dot.attrs.cy)).toBeCloseTo(last[1], 1);
  });

  it("labels the step for a screen reader as well as on screen", () => {
    expect(String(frame.attrs["aria-label"])).toContain("up the right side");
    expect(byTag(frame, "title")[0].text).toContain("up the right side");
    expect(byTag(frame, "tspan").map((t) => t.text).join(" ")).toBe("3. up the right side");
  });

  it("carries an intrinsic size matching its viewBox, so it never renders squashed", () => {
    const [, , w, h] = String(frame.attrs.viewBox).split(" ").map(Number);
    expect(Number(frame.attrs.height) / Number(frame.attrs.width)).toBeCloseTo(h / w, 2);
  });
});

describe("captions wrap instead of running off the panel", () => {
  it("breaks on whole words at the width available", () => {
    // 900 units wide at 92-unit text ≈ 18 characters a line.
    expect(wrapCaption("1. down the left upright", 900, 92)).toEqual(["1. down the left", "upright"]);
    expect(wrapCaption("4. over the top", 900, 92)).toEqual(["4. over the top"]);
  });

  it("never chops a word in half, even one too long to fit", () => {
    const lines = wrapCaption("Rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr", 200, 92);
    expect(lines).toEqual(["Rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"]);
  });

  it("returns one empty line for empty text rather than no lines at all", () => {
    expect(wrapCaption("   ", 900, 92)).toEqual([""]);
  });

  it("makes the box taller when the captions need two lines", () => {
    const short = viewBoxFor(MA, outline, { captionSize: 20 }).height;
    const tall = viewBoxFor(MA, outline, { captionSize: 200 }).height;
    expect(tall).toBeGreaterThan(short);
  });
});

// ---------------------------------------------------------------------------
// The flip. This is the invariant the whole module exists to guarantee: the
// glyph and the pen path are both in font units (y-up), SVG is y-down, and they
// are flipped together exactly once — so they cannot end up disagreeing.
// ---------------------------------------------------------------------------
describe("one shared y-flip", () => {
  const frame = ductusFrame(MA, outline, ductusSteps(MA)[4]);

  it("uses exactly one scale(1,-1) group", () => {
    const flips = collect(frame, (n) => String(n.attrs.transform ?? "").includes("scale(1,-1)"));
    expect(flips).toHaveLength(1);
  });

  it("puts the glyph, every pen path and the pen dot inside that one group", () => {
    const flip = collect(frame, (n) => String(n.attrs.transform ?? "").includes("scale(1,-1)"))[0];
    expect(byTag(flip, "path").length).toBe(byTag(frame, "path").length);
    expect(byTag(flip, "circle").length).toBe(byTag(frame, "circle").length);
    expect(byTag(flip, "path").length).toBeGreaterThan(1); // glyph + pen
  });

  it("keeps text OUT of the flip, because mirrored text is unreadable", () => {
    const flip = collect(frame, (n) => String(n.attrs.transform ?? "").includes("scale(1,-1)"))[0];
    expect(byTag(flip, "text")).toHaveLength(0);
    expect(byTag(frame, "text")).toHaveLength(1);
  });

  it("negates the vertical range in the viewBox, as the flip requires", () => {
    const box = viewBoxFor(MA, outline);
    const b = outline.bounds;
    // Top of the letter (largest font y) becomes the SMALLEST svg y.
    expect(box.minY).toBeLessThan(0);
    expect(box.minY).toBeCloseTo(-(b.y1 + 70), 5);
    // The box is wide enough for the ink, and taller by the caption band.
    expect(box.width).toBeGreaterThanOrEqual(b.x1 - b.x0);
    expect(box.height).toBeGreaterThan(b.y1 - b.y0);
  });

  it("keeps the whole flipped glyph inside the viewBox", () => {
    const box = viewBoxFor(MA, outline);
    const b = outline.bounds;
    // Flip every corner of the glyph box and check containment.
    for (const x of [b.x0, b.x1]) {
      for (const y of [b.y0, b.y1]) {
        expect(x).toBeGreaterThanOrEqual(box.minX);
        expect(x).toBeLessThanOrEqual(box.minX + box.width);
        expect(-y).toBeGreaterThanOrEqual(box.minY);
        expect(-y).toBeLessThanOrEqual(box.minY + box.height);
      }
    }
  });

  it("CONTROL: an unflipped viewBox would cut the letter off entirely", () => {
    // If someone "simplified" the box to raw font coordinates, the flipped
    // glyph would sit at negative y and fall completely outside it. This is the
    // failure the negation prevents; assert it really is a failure.
    const b = outline.bounds;
    const naiveTop = b.y0;
    expect(-b.y1).toBeLessThan(naiveTop);
  });
});

describe("the build-up advances", () => {
  const strip = ductusFilmstrip(MA, outline);

  it("has one frame per step", () => {
    expect(strip.frames).toHaveLength(strip.steps.length);
    expect(strip.frames).toHaveLength(5);
  });

  it("draws strictly more of the stroke in each successive frame", () => {
    const drawn = strip.frames.map(
      (f) => String(byTag(f, "path").find((p) => p.attrs.class === "ductus__pen")!.attrs.d).length,
    );
    for (let i = 1; i < drawn.length; i++) {
      expect(drawn[i]).toBeGreaterThan(drawn[i - 1]);
    }
  });

  it("the last frame is the complete stroke", () => {
    const last = strip.frames[strip.frames.length - 1];
    const pen = byTag(last, "path").find((p) => p.attrs.class === "ductus__pen")!;
    expect(pen.attrs.d).toBe(penPathD(MA.strokes[0], 1));
  });

  it("says in words how many strokes and lifts there are", () => {
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });
});

describe("அ — a real cited two-stroke filmstrip", () => {
  const steps = ductusSteps(A);
  const strip = ductusFilmstrip(A, aOutline);

  it("places the only pen lift before the separate right upright", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
  });

  it("reports the source-backed movement, stroke, and lift counts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });

  it("keeps the completed body visible while drawing the upright", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(A.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(A.strokes[1], 1));
  });
});

describe("ஆ — the upright and long-vowel loop stay connected", () => {
  const steps = ductusSteps(AA);
  const strip = ductusFilmstrip(AA, aaOutline);

  it("places one lift before the upright and none before its loop", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("finishes the connected upright-and-loop stroke in the last frame", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(AA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(AA.strokes[1], 1));
  });
});

describe("இ — a real cited seven-movement filmstrip", () => {
  const steps = ductusSteps(I);
  const strip = ductusFilmstrip(I, iOutline);

  it("places one lift before the outer climb and joins that climb to the arch", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1, 1]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("finishes the joined outer climb-and-arch stroke in the last frame", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(I.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(I.strokes[1], 1));
  });
});

describe("க — a real cited three-stroke filmstrip", () => {
  const steps = ductusSteps(KA);
  const strip = ductusFilmstrip(KA, kaOutline);

  it("places lifts before each lower bowl", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible while drawing the right bowl", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([penPathD(KA.strokes[0], 1), penPathD(KA.strokes[1], 1)]);
    expect(pen.attrs.d).toBe(penPathD(KA.strokes[2], 1));
  });
});

describe("வ — a real cited unbroken five-movement filmstrip", () => {
  const steps = ductusSteps(VA);
  const strip = ductusFilmstrip(VA, vaOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0]);
  });

  it("reports five movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 5 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(VA.strokes[0], 1));
  });
});

describe("ல — a real cited unbroken four-movement filmstrip", () => {
  const steps = ductusSteps(LA);
  const strip = ductusFilmstrip(LA, laOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports four movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });

  it("finishes the sole stroke without any completed-stroke overlay", () => {
    const last = strip.frames[3];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(0);
    expect(pen.attrs.d).toBe(penPathD(LA.strokes[0], 1));
  });
});

describe("ற — a real cited three-stroke five-movement filmstrip", () => {
  const steps = ductusSteps(RRA);
  const strip = ductusFilmstrip(RRA, rraOutline);

  it("marks exactly the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 2]);
  });

  it("reports five movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 5 movements");
  });

  it("keeps both completed strokes visible while drawing the joined sweep", () => {
    const last = strip.frames[4];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([penPathD(RRA.strokes[0], 1), penPathD(RRA.strokes[1], 1)]);
    expect(pen.attrs.d).toBe(penPathD(RRA.strokes[2], 1));
  });
});

describe("ன — a real cited two-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(NNA);
  const strip = ductusFilmstrip(NNA, nnaOutline);

  it("joins the loop, inner arch, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 1]);
  });

  it("reports six movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });

  it("keeps the completed loop-and-bar stroke visible while drawing the upright", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(NNA.strokes[1], 1));
  });
});

describe("ண — a real cited two-stroke seven-movement filmstrip", () => {
  const steps = ductusSteps(RETROFLEX_NNA);
  const strip = ductusFilmstrip(RETROFLEX_NNA, retroflexNnaOutline);

  it("joins the loop, both inner arches, and top bar before the sole lift", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, false, false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0, 1]);
  });

  it("reports seven movements in two strokes with one lift", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });

  it("keeps the completed double-arch stroke visible while drawing the upright", () => {
    const last = strip.frames[6];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[0], 1));
    expect(pen.attrs.d).toBe(penPathD(RETROFLEX_NNA.strokes[1], 1));
  });
});

describe("ந — a real cited three-stroke six-movement filmstrip", () => {
  const steps = ductusSteps(DENTAL_NA);
  const strip = ductusFilmstrip(DENTAL_NA, dentalNaOutline);

  it("marks the two source-backed lift transitions", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false, true, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
  });

  it("reports six movements in three strokes with two lifts", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });

  it("keeps both completed strokes visible during the right-hand descent", () => {
    const last = strip.frames[5];
    const done = byTag(last, "path").filter((path) => path.attrs.class === "ductus__done");
    const pen = byTag(last, "path").find((path) => path.attrs.class === "ductus__pen")!;
    expect(done).toHaveLength(2);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(DENTAL_NA.strokes[0], 1),
      penPathD(DENTAL_NA.strokes[1], 1),
    ]);
    expect(pen.attrs.d).toBe(penPathD(DENTAL_NA.strokes[2], 1));
  });
});

describe("Persian ا — the first cited right-to-left-script filmstrip", () => {
  const steps = ductusSteps(PERSIAN_ALEF);
  const strip = ductusFilmstrip(PERSIAN_ALEF, persianAlefOutline);

  it("keeps the source's top-to-bottom stem in one pen-down run", () => {
    expect(steps).toHaveLength(1);
    expect(steps[0].label).toBe("down");
    expect(steps[0].startsAfterLift).toBe(false);
    expect(steps[0].strokeIndex).toBe(0);
    const path = PERSIAN_ALEF.strokes[0].segments[0].path;
    expect(path[0].y).toBeGreaterThan(path.at(-1)!.y);
  });

  it("reports one movement with no pen lift", () => {
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the vendored Noto Naskh outline behind the complete path", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_ALEF.strokes[0], 1),
    );
  });
});

describe("Arabic ا — an independent, script-scoped filmstrip", () => {
  const steps = ductusSteps(ARABIC_ALEF);
  const strip = ductusFilmstrip(ARABIC_ALEF, arabicAlefOutline);

  it("shows one downward movement with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual(["down"]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(ARABIC_ALEF.strokes[0].segments[0].path[0].y).toBeGreaterThan(
      ARABIC_ALEF.strokes[0].segments[0].path.at(-1)!.y,
    );
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the vendored Noto Naskh outline behind the sourced path", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_ALEF.strokes[0], 1),
    );
  });
});

describe("Arabic ب — a script-scoped bowl-and-dot filmstrip", () => {
  const steps = ductusSteps(ARABIC_BAA);
  const strip = ductusFilmstrip(ARABIC_BAA, arabicBaaOutline);

  it("shows the right-to-left bowl before the lifted dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(ARABIC_BAA.strokes[0].segments[0].path[0].x).toBeGreaterThan(
      ARABIC_BAA.strokes[0].segments[0].path.at(-1)!.x,
    );
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and retains the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicBaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_BAA.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_BAA.strokes[1], 1),
    );
  });
});

describe("Arabic ت — a script-scoped bowl-and-two-dots filmstrip", () => {
  const steps = ductusSteps(ARABIC_TAA);
  const strip = ductusFilmstrip(ARABIC_TAA, arabicTaaOutline);

  it("shows the shared right-to-left bowl before both separately lifted dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the left dot above",
      "lift again and place the right dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    expect(ARABIC_TAA.strokes[0].segments[0].path[0].x).toBeGreaterThan(
      ARABIC_TAA.strokes[0].segments[0].path.at(-1)!.x,
    );
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("draws the Noto Naskh outline and retains the bowl and left dot in the final frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicTaaOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(ARABIC_TAA.strokes[0], 1), penPathD(ARABIC_TAA.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_TAA.strokes[2], 1),
    );
  });
});

describe("Arabic ج — a body-first hook-and-dot filmstrip", () => {
  const steps = ductusSteps(ARABIC_JEEM);
  const strip = ductusFilmstrip(ARABIC_JEEM, arabicJeemOutline);

  it("keeps the sourced head and bowl in one stroke before the lifted dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short upper head from left to right",
      "continue down and around the bowl",
      "lift once, then place the dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the body in the final dot frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicJeemOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_JEEM.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_JEEM.strokes[1], 1),
    );
  });
});

describe("Arabic ح — a stem-first, dotless filmstrip", () => {
  const steps = ductusSteps(ARABIC_HAA);
  const strip = ductusFilmstrip(ARABIC_HAA, arabicHaaOutline);

  it("keeps the short stem separate from the restarted head-and-bowl run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short left stem downward",
      "lift once and restart near the stem's top",
      "continue down and around the bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the separate stem in the final bowl frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicHaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_HAA.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_HAA.strokes[1], 1),
    );
  });
});

describe("Arabic خ — a body-first hook-and-upper-dot filmstrip", () => {
  const steps = ductusSteps(ARABIC_KHAA);
  const strip = ductusFilmstrip(ARABIC_KHAA, arabicKhaaOutline);

  it("keeps the sourced head and bowl in one stroke before the lifted upper dot", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the short upper head from left to right",
      "continue down and around the bowl",
      "lift once, then place the dot above",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and retains the body in the final upper-dot frame", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicKhaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(ARABIC_KHAA.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_KHAA.strokes[1], 1),
    );
  });
});

describe("Arabic د — an unbroken shoulder-and-baseline filmstrip", () => {
  const steps = ductusSteps(ARABIC_DAAL);
  const strip = ductusFilmstrip(ARABIC_DAAL, arabicDaalOutline);

  it("keeps the sourced descent and leftward baseline turn in one stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the upper tip and descend diagonally down and right through the curved shoulder",
      "turn left along the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicDaalOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_DAAL.strokes[0], 1),
    );
  });
});

describe("Arabic ر — an unbroken descending-curve filmstrip", () => {
  const steps = ductusSteps(ARABIC_RAA);
  const strip = ductusFilmstrip(ARABIC_RAA, arabicRaaOutline);

  it("keeps the sourced descent and leftward lower curve in one stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "begin at the upper tip and descend through the short stroke",
      "sweep left through the lower curve without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicRaaOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_RAA.strokes[0], 1),
    );
  });
});

describe("Arabic س — an unbroken teeth-and-bowl filmstrip", () => {
  const steps = ductusSteps(ARABIC_SEEN);
  const strip = ductusFilmstrip(ARABIC_SEEN, arabicSeenOutline);

  it("keeps the sourced three teeth and final bowl in one stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "form the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline behind the completed sourced path", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      arabicSeenOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(ARABIC_SEEN.strokes[0], 1),
    );
  });
});

describe("Urdu ا — an independent, source-specific filmstrip", () => {
  const steps = ductusSteps(URDU_ALEF);
  const strip = ductusFilmstrip(URDU_ALEF, urduAlefOutline);

  it("shows one downward movement with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual(["down"]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(URDU_ALEF.strokes[0].segments[0].path[0].y).toBeGreaterThan(
      URDU_ALEF.strokes[0].segments[0].path.at(-1)!.y,
    );
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("uses the vendored Noto Naskh fallback outline", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduAlefOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_ALEF.strokes[0], 1),
    );
  });
});

describe("Urdu ج — dot first, then one continuous pointed body", () => {
  const steps = ductusSteps(URDU_JIM);
  const strip = ductusFilmstrip(URDU_JIM, urduJimOutline);

  it("shows the sourced dot-first order and only one lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "place the dot below",
      "lift, then sweep left through the pointed hooked head",
      "continue down and around the bowl",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and keeps the completed dot through the body", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduJimOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(URDU_JIM.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_JIM.strokes[1], 1),
    );
  });
});

describe("Urdu ر — one downward line that continues left", () => {
  const steps = ductusSteps(URDU_RE);
  const strip = ductusFilmstrip(URDU_RE, urduReOutline);

  it("shows both sourced movements in one unbroken stroke", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the downward line",
      "continue curving to the left",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("uses Noto Naskh and completes the same pen-down run", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduReOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_RE.strokes[0], 1),
    );
  });
});

describe("Urdu س — three close teeth flowing into one final bowl", () => {
  const steps = ductusSteps(URDU_SIN);
  const strip = ductusFilmstrip(URDU_SIN, urduSinOutline);

  it("shows both sourced movements in one right-to-left pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("uses Noto Naskh and completes the same pen-down run", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduSinOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_SIN.strokes[0], 1),
    );
  });
});

describe("Urdu ش — a complete س body followed by three dots", () => {
  const steps = ductusSteps(URDU_SHIN);
  const strip = ductusFilmstrip(URDU_SHIN, urduShinOutline);

  it("shows the body first, then lower-left, lower-right, and upper dots", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the three close teeth from right to left",
      "flow directly into the final bowl without lifting",
      "lift, then place the lower-left dot",
      "lift again, then place the lower-right dot",
      "lift a third time, then place the centered upper dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3]);
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 5 movements");
  });

  it("uses Noto Naskh and preserves all completed strokes during the upper dot", () => {
    const paths = byTag(strip.frames[4], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduShinOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(3);
    expect(done.map((path) => path.attrs.d)).toEqual([
      penPathD(URDU_SHIN.strokes[0], 1),
      penPathD(URDU_SHIN.strokes[1], 1),
      penPathD(URDU_SHIN.strokes[2], 1),
    ]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_SHIN.strokes[3], 1),
    );
  });
});

describe("Urdu ک — a main-line body followed by its long slash", () => {
  const steps = ductusSteps(URDU_KAF);
  const strip = ductusFilmstrip(URDU_KAF, urduKafOutline);

  it("shows the stem and flatter hooked bowl before the separately lifted slash", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the independent stem downward",
      "flow right to left through the flatter bowl and finish with the hook without lifting",
      "lift, then draw the long slash down from the upper right toward the stem",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("uses Noto Naskh and preserves the completed body during the slash", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduKafOutline.path,
    );
    const done = paths.filter((path) => path.attrs.class === "ductus__done");
    expect(done).toHaveLength(1);
    expect(done[0].attrs.d).toBe(penPathD(URDU_KAF.strokes[0], 1));
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_KAF.strokes[1], 1),
    );
  });
});

describe("Urdu ل — its upright continues through a below-baseline bowl", () => {
  const steps = ductusSteps(URDU_LAM);
  const strip = ductusFilmstrip(URDU_LAM, urduLamOutline);

  it("keeps the downward upright and leftward bowl in one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the tall independent upright downward",
      "continue below the baseline through the leftward bowl and back up without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduLamOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_LAM.strokes[0], 1),
    );
  });
});

describe("Urdu م — its round head flows into the below-baseline tail", () => {
  const steps = ductusSteps(URDU_MIM);
  const strip = ductusFilmstrip(URDU_MIM, urduMimOutline);

  it("keeps the head and tail in one sourced pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the round head",
      "continue down the tail below the baseline without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const head = URDU_MIM.strokes[0].segments[0].path;
    const tail = URDU_MIM.strokes[0].segments[1].path;
    expect(tail[0]).toEqual(head.at(-1));
    expect(Math.min(...tail.map((point) => point.y))).toBeLessThan(0);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduMimOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_MIM.strokes[0], 1),
    );
  });
});

describe("Urdu ن — its below-baseline bowl precedes the lifted dot", () => {
  const steps = ductusSteps(URDU_NUN);
  const strip = ductusFilmstrip(URDU_NUN, urduNunOutline);

  it("keeps the bowl together, then marks the sourced lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the independent bowl right to left below the baseline",
      "lift, then place the dot near the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    const bowl = URDU_NUN.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(Math.min(...bowl.map((point) => point.y))).toBeLessThan(0);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and retains the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduNunOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(URDU_NUN.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_NUN.strokes[1], 1),
    );
  });
});

describe("Urdu ں — its dotless bowl is one unbroken stroke", () => {
  const steps = ductusSteps(URDU_GHUNNA);
  const strip = ductusFilmstrip(URDU_GHUNNA, urduGhunnaOutline);

  it("shows the sourced dotless nūn bowl with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the independent dotless bowl right to left below the baseline",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced bowl", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduGhunnaOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_GHUNNA.strokes[0], 1),
    );
  });
});

describe("Urdu ہ — its independent teardrop is one unbroken loop", () => {
  const steps = ductusSteps(URDU_HE);
  const strip = ductusFilmstrip(URDU_HE, urduHeOutline);

  it("shows one sourced counterclockwise loop with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "loop the independent teardrop counterclockwise without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("draws the Noto Naskh outline and the complete sourced loop", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduHeOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_HE.strokes[0], 1),
    );
  });
});

describe("Urdu ی — its independent S and bowl are one unbroken stroke", () => {
  const steps = ductusSteps(URDU_YE);
  const strip = ductusFilmstrip(URDU_YE, urduYeOutline);

  it("shows the sourced dotless S and bowl with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper right through the independent S curve",
      "continue left around the below-baseline bowl and finish at its rising tip",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced stroke", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduYeOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_YE.strokes[0], 1),
    );
  });
});

describe("Urdu ے — its broad bowl folds backward in one unbroken stroke", () => {
  const steps = ductusSteps(URDU_BARI_YE);
  const strip = ductusFilmstrip(URDU_BARI_YE, urduBariYeOutline);

  it("shows the sourced upper sweep, curl, and lower fold with no lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "descend from the upper right and sweep left across the broad bowl",
      "curl back underneath at the far left without lifting",
      "continue right along the lower fold without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });

  it("draws the Noto Naskh outline and finishes the complete sourced fold", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      urduBariYeOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(URDU_BARI_YE.strokes[0], 1),
    );
  });
});

describe("Persian ب — a right-to-left bowl followed by its dot", () => {
  const steps = ductusSteps(PERSIAN_BEH);
  const strip = ductusFilmstrip(PERSIAN_BEH, persianBehOutline);

  it("keeps the bowl in one right-to-left run, then marks the sourced lift", () => {
    expect(steps).toHaveLength(2);
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the dot below",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    const bowl = PERSIAN_BEH.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and preserves the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianBehOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(PERSIAN_BEH.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_BEH.strokes[1], 1),
    );
  });
});

describe("Persian ت — the shared bowl followed by two separate dots", () => {
  const steps = ductusSteps(PERSIAN_TEH);
  const strip = ductusFilmstrip(PERSIAN_TEH, persianTehOutline);

  it("keeps the bowl in one run, then preserves both sourced dot lifts", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the left dot above",
      "lift again and place the right dot",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1, 2]);
    const bowl = PERSIAN_TEH.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
    expect(PERSIAN_TEH.strokes[1].segments[0].path[0].x).toBeLessThan(
      PERSIAN_TEH.strokes[2].segments[0].path[0].x,
    );
  });

  it("reports three movements separated by two pen lifts", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("retains the bowl and left dot while the right dot is placed", () => {
    const paths = byTag(strip.frames[2], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianTehOutline.path,
    );
    expect(
      paths.filter((path) => path.attrs.class === "ductus__done").map((path) => path.attrs.d),
    ).toEqual([penPathD(PERSIAN_TEH.strokes[0], 1), penPathD(PERSIAN_TEH.strokes[1], 1)]);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_TEH.strokes[2], 1),
    );
  });
});

describe("Persian س — three teeth flowing into one final bowl", () => {
  const steps = ductusSteps(PERSIAN_SIN);
  const strip = ductusFilmstrip(PERSIAN_SIN, persianSinOutline);

  it("keeps both sourced movements in one right-to-left pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "form the three teeth from right to left",
      "flow into the final bowl without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const first = PERSIAN_SIN.strokes[0].segments[0].path[0];
    const last = PERSIAN_SIN.strokes[0].segments.at(-1)!.path.at(-1)!;
    expect(first.x).toBeGreaterThan(last.x);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianSinOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_SIN.strokes[0], 1),
    );
  });
});

describe("Persian ل — its upright turns directly into the base curve", () => {
  const steps = ductusSteps(PERSIAN_LAM);
  const strip = ductusFilmstrip(PERSIAN_LAM, persianLamOutline);

  it("keeps both sourced movements in one descending pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "draw the upright downward",
      "turn into the base curve without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const first = PERSIAN_LAM.strokes[0].segments[0].path[0];
    const last = PERSIAN_LAM.strokes[0].segments.at(-1)!.path.at(-1)!;
    expect(first.y).toBeGreaterThan(last.y);
    expect(first.x).toBeGreaterThan(last.x);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianLamOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_LAM.strokes[0], 1),
    );
  });
});

describe("Persian م — its round head flows into the descending tail", () => {
  const steps = ductusSteps(PERSIAN_MIM);
  const strip = ductusFilmstrip(PERSIAN_MIM, persianMimOutline);

  it("keeps both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the round head",
      "continue down the tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const head = PERSIAN_MIM.strokes[0].segments[0].path;
    const tail = PERSIAN_MIM.strokes[0].segments[1].path;
    expect(head[0].x).toBeLessThan(head.at(-1)!.x);
    expect(tail[0].y).toBeGreaterThan(tail.at(-1)!.y);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianMimOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_MIM.strokes[0], 1),
    );
  });
});

describe("Persian ن — its bowl is followed by a separately placed dot", () => {
  const steps = ductusSteps(PERSIAN_NUN);
  const strip = ductusFilmstrip(PERSIAN_NUN, persianNunOutline);

  it("keeps the bowl in one right-to-left run, then marks the sourced lift", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "sweep the bowl from right to left",
      "lift, then place the dot above",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    const bowl = PERSIAN_NUN.strokes[0].segments[0].path;
    expect(bowl[0].x).toBeGreaterThan(bowl.at(-1)!.x);
  });

  it("reports two movements separated by one pen lift", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });

  it("draws the Noto Naskh outline and preserves the bowl during the dot", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianNunOutline.path,
    );
    expect(paths.find((path) => path.attrs.class === "ductus__done")!.attrs.d).toBe(
      penPathD(PERSIAN_NUN.strokes[0], 1),
    );
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_NUN.strokes[1], 1),
    );
  });
});

describe("Persian و — its small head flows into one leftward tail", () => {
  const steps = ductusSteps(PERSIAN_WAW);
  const strip = ductusFilmstrip(PERSIAN_WAW, persianWawOutline);

  it("keeps both sourced movements in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "shape the small head loop",
      "flow into the leftward tail without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0]);
    const tail = PERSIAN_WAW.strokes[0].segments[1].path;
    expect(tail[0].x).toBeGreaterThan(tail.at(-1)!.x);
  });

  it("reports two movements in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 2 movements");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[1], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianWawOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_WAW.strokes[0], 1),
    );
  });
});

describe("Persian ه — its isolated looping body stays in one pen-down run", () => {
  const steps = ductusSteps(PERSIAN_HEH);
  const strip = ductusFilmstrip(PERSIAN_HEH, persianHehOutline);

  it("keeps the sourced looping body in one pen-down run", () => {
    expect(steps.map((step) => step.label)).toEqual([
      "loop the isolated body and finish left without lifting",
    ]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0]);
  });

  it("reports one movement in one unbroken stroke", () => {
    expect(strip.frames).toHaveLength(1);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 1 movement");
  });

  it("finishes the Noto Naskh path without a completed-stroke overlay", () => {
    const paths = byTag(strip.frames[0], "path");
    expect(paths.find((path) => path.attrs.class === "ductus__glyph")!.attrs.d).toBe(
      persianHehOutline.path,
    );
    expect(paths.filter((path) => path.attrs.class === "ductus__done")).toHaveLength(0);
    expect(paths.find((path) => path.attrs.class === "ductus__pen")!.attrs.d).toBe(
      penPathD(PERSIAN_HEH.strokes[0], 1),
    );
  });
});

// ---------------------------------------------------------------------------
// Generic multi-stroke edge cases still use a synthetic ductus so the test can
// vary stroke counts independently of curriculum data. Nothing in this fixture
// is ever shown to a learner, and no letter enters DUCTUS without a citation.
// ---------------------------------------------------------------------------
describe("a letter written in more than one stroke", () => {
  const twoStroke: LetterDuctus = {
    script: "test",
    glyph: "✚",
    strokes: [
      { segments: [{ label: "the upright", path: [{ x: 100, y: 0 }, { x: 100, y: 400 }] }] },
      {
        segments: [
          { label: "the crossbar, left half", path: [{ x: 0, y: 200 }, { x: 100, y: 200 }] },
          { label: "the crossbar, right half", path: [{ x: 100, y: 200 }, { x: 200, y: 200 }] },
        ],
      },
    ],
    source: { citation: "test fixture, not curriculum data", url: "https://example.invalid/fixture" },
  };
  const fakeOutline: GlyphOutline = { path: "M0 0L1 1Z", bounds: { x0: 0, y0: 0, x1: 200, y1: 400 } };

  it("renders every frame without throwing", () => {
    expect(() => ductusFilmstrip(twoStroke, fakeOutline)).not.toThrow();
    const strip = ductusFilmstrip(twoStroke, fakeOutline);
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 3 movements");
  });

  it("flags the frame where the hand leaves the paper", () => {
    const steps = ductusSteps(twoStroke);
    expect(steps.map((s) => s.startsAfterLift)).toEqual([false, true, false]);
  });

  it("keeps already-finished strokes on screen, in a settled colour", () => {
    const strip = ductusFilmstrip(twoStroke, fakeOutline);
    const first = byTag(strip.frames[0], "path").filter((p) => p.attrs.class === "ductus__done");
    const later = byTag(strip.frames[2], "path").filter((p) => p.attrs.class === "ductus__done");
    expect(first).toHaveLength(0); // nothing finished yet
    expect(later).toHaveLength(1); // the upright, done, behind the crossbar
    expect(later[0].attrs.d).toBe(penPathD(twoStroke.strokes[0], 1));
  });

  it("counts lifts and movements in plain English", () => {
    const strip = (n: number) =>
      ductusFilmstrip(
        {
          script: "test",
          glyph: "?",
          strokes: Array.from({ length: n }, (_, i) => ({
            segments: [{ label: `part ${i}`, path: [{ x: 0, y: 0 }, { x: 10, y: 0 }] }],
          })),
          source: twoStroke.source,
        },
        fakeOutline,
      ).summary;
    expect(strip(1)).toBe("one unbroken stroke · 1 movement");
    expect(strip(3)).toBe("3 strokes · 2 pen lifts · 3 movements");
  });

  it("honours caller-supplied sizes and colours", () => {
    const strip = ductusFilmstrip(twoStroke, fakeOutline, { padding: 0, penColor: "#ff0000" });
    const pen = byTag(strip.frames[0], "path").find((p) => p.attrs.class === "ductus__pen")!;
    expect(pen.attrs.stroke).toBe("#ff0000");
    // With no padding the box hugs the ink exactly sideways, and is taller than
    // the letter by exactly the caption band.
    const box = viewBoxFor(twoStroke, fakeOutline, { padding: 0 });
    expect(box.minX).toBe(0);
    expect(box.minY).toBe(-400);
    expect(box.width).toBe(200);
    expect(box.height).toBeGreaterThan(400);
  });

  it("gives every frame of a letter the SAME box, so the strip reads as one picture", () => {
    const boxes = ductusFilmstrip(twoStroke, fakeOutline).frames.map((f) => f.attrs.viewBox);
    expect(new Set(boxes).size).toBe(1);
  });

  it("draws nothing rather than an empty path when a part has no points", () => {
    const hollow: LetterDuctus = {
      script: "test",
      glyph: "␀",
      strokes: [{ segments: [{ label: "nothing at all", path: [] }] }],
      source: twoStroke.source,
    };
    const frame = ductusFrame(hollow, fakeOutline, ductusSteps(hollow)[0]);
    // Only the glyph outline survives; no zero-length pen path is emitted.
    expect(byTag(frame, "path")).toHaveLength(1);
    expect(byTag(frame, "path")[0].attrs.class).toBe("ductus__glyph");
    // The box still falls back to the glyph's own bounds when the pen has none.
    expect(viewBoxFor(hollow, fakeOutline).width).toBeGreaterThan(200);
  });

  it("survives degenerate input rather than emitting a zero-size picture", () => {
    const empty: LetterDuctus = { script: "test", glyph: "␣", strokes: [], source: twoStroke.source };
    const nowhere: GlyphOutline = { path: "", bounds: { x0: 0, y0: 0, x1: 0, y1: 0 } };
    const strip = ductusFilmstrip(empty, nowhere);
    expect(strip.frames).toHaveLength(0);
    expect(strip.penLifts).toBe(0);
    const box = viewBoxFor(empty, nowhere);
    expect(box.width).toBeGreaterThan(0);
    expect(box.height).toBeGreaterThan(0);
  });
});

describe("serialising to markup", () => {
  it("produces well-formed SVG with the path data intact", () => {
    const svg = svgMarkup(ductusFrame(MA, outline, ductusSteps(MA)[0]));
    expect(svg.startsWith("<svg ")).toBe(true);
    expect(svg.endsWith("</svg>")).toBe(true);
    expect(svg).toContain('xmlns="http://www.w3.org/2000/svg"');
    expect(svg).toContain(`d="${outline.path}"`);
    expect(svg).toContain('transform="scale(1,-1)"');
    // A DOM parser is the real test of well-formedness.
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelectorAll("path").length).toBeGreaterThan(1);
  });

  it("escapes every XML metacharacter", () => {
    expect(escapeXml(`<&>"'`)).toBe("&lt;&amp;&gt;&quot;&apos;");
    expect(escapeXml("plain")).toBe("plain");
  });

  it("cannot be escaped out of an attribute by a hostile label", () => {
    // A label is authored today, but escaping is a property of the writer, not
    // of the data. Feed it markup and check none survives as markup.
    const nasty: LetterDuctus = {
      script: "test",
      glyph: "x",
      strokes: [
        {
          segments: [
            {
              label: `"><script>alert(1)</script>`,
              path: [{ x: 0, y: 0 }, { x: 10, y: 10 }],
            },
          ],
        },
      ],
      source: { citation: "test fixture", url: "https://example.invalid/fixture" },
    };
    const svg = svgMarkup(
      ductusFrame(nasty, { path: "M0 0Z", bounds: { x0: 0, y0: 0, x1: 10, y1: 10 } }, ductusSteps(nasty)[0]),
    );
    expect(svg).not.toContain("<script>");
    expect(svg).toContain("&lt;script&gt;");
    expect(svg).toContain("&quot;");
    const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelector("script")).toBeNull();
  });

  it("self-closes childless elements", () => {
    expect(svgMarkup({ tag: "circle", attrs: { r: 3 } })).toBe('<circle r="3"/>');
  });

  // An attribute NAME cannot be escaped — there is nowhere to put the entity —
  // so a name is either legal or dropped. Every name this module emits is a
  // literal, but `SvgNode` is public and the serialiser is meant to be reused.
  it("drops attribute names that are not legal XML names", () => {
    expect(isSafeName("stroke-width")).toBe(true);
    expect(isSafeName("xlink:href")).toBe(true);
    expect(isSafeName(`x" onload="alert(1)`)).toBe(false);
    expect(isSafeName("2bad")).toBe(false);
    const svg = svgMarkup({ tag: "rect", attrs: { [`x" onload="alert(1)`]: "1", width: 4 } });
    expect(svg).toBe('<rect width="4"/>');
  });

  it("refuses event-handler attributes outright, prefix and all", () => {
    // `onload` is a perfectly legal XML name AND a script. Reject the prefix
    // rather than chase a list of handler names that keeps growing.
    expect(isSafeName("onload")).toBe(false);
    expect(isSafeName("OnClick")).toBe(false);
    expect(svgMarkup({ tag: "svg", attrs: { onload: "alert(1)", onclick: "x" } })).toBe("<svg/>");
  });

  it("neutralises a hostile tag name rather than emitting it", () => {
    expect(svgMarkup({ tag: "svg><script", attrs: {} })).toBe("<g/>");
  });
});
