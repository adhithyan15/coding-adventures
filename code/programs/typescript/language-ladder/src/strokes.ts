// ---------------------------------------------------------------------------
// strokes.ts — how a letter is written, as a path the pen actually travels
// ---------------------------------------------------------------------------
//
// The writing lessons teach a learner to FORM a letter, which means teaching
// the motion of the pen: where it starts, which way it goes, and — the part a
// static picture of the finished letter cannot show — where it stays down and
// where (if ever) it lifts.

import persoArabic from "../../../../learning/human-languages/data/scripts/perso-arabic.json";
//
// The model, in one breath
// ------------------------
// A letter is written as one or more **strokes**. A stroke is a single
// pen-DOWN run: once you start it you do not lift until it ends. The pen lifts
// only BETWEEN strokes, so a letter written "without taking your hand off" is
// one stroke, and the number of lifts is always `strokes.length - 1`.
//
// A letter still has recognisable **parts** — ம has a left upright, a bottom
// bar, an arch. But those parts are not separate strokes; they are labelled
// SEGMENTS of one continuous pen path, chosen so that each segment begins
// exactly where the previous one ended. Keeping the parts joined like that is
// the whole point: it is what lets the hand write the letter in one motion
// instead of lifting between every piece.
//
// So the data says two different things, and keeps them separate:
//   • the PARTS a learner can see in the finished letter (labels), and
//   • the unbroken PATH the hand travels to make them (the points).
//
// Why the path is authored, but not trusted
// ------------------------------------------
// Unlike the glyph outline — which we read from the font and never draw — the
// pen path IS authored by hand. There is no font table that says "start here,
// go this way." So the path is checked against the font instead of believed:
//
//   1. every point of a stroke's pen path must land on the real glyph's ink
//      (`fractionOnInk` in the test) — a stroke drawn in the wrong place fails;
//   2. within a stroke, consecutive segments must MEET — the end of one part is
//      the start of the next, within a tight tolerance — so the "parts don't
//      force a lift" claim is verified, not asserted;
//   3. the strokes together must pass near ALL of the letter's ink, so the
//      path traces the whole letter and not just the easy half.
//
// That turns "did Claude draw this letter correctly" — which no one in the
// audience could check by looking — into "does this path lie on the font
// shape", which a machine checks exactly. Hand-drawing is allowed; hand-drawing
// something wrong is not.
// ---------------------------------------------------------------------------

/** A point on the pen path, in FONT units (y-up, baseline at 0). */
export interface Point {
  x: number;
  y: number;
}

/**
 * One labelled part of a letter, as the slice of pen path that draws it.
 *
 * `path` is a polyline the pen follows. The FIRST point of a segment is the
 * SAME point as the last point of the previous segment in the same stroke —
 * that shared point is the join where the pen carries on without lifting.
 */
export interface Segment {
  label: string;
  path: Point[];
}

/** One pen-down run: its segments, travelled in order without lifting. */
export interface Stroke {
  segments: Segment[];
}

/**
 * Where a letter's stroke ORDER came from.
 *
 * The pen path's SHAPE is checked against the font (see the test). Its ORDER
 * cannot be — no font records it — so the order must trace to a real source:
 * a handwriting primer, a stroke database, or a named native writer. There is
 * no authoritative machine-readable stroke database for Tamil (or any Indic
 * script, Arabic, or Hebrew), so the order is read from a cited teaching
 * source, and `variation` records where the attested teaching form can vary.
 */
export interface StrokeSource {
  citation: string;
  url: string;
  /** How standardised the order is, and where it varies. */
  variation?: string;
}

/** A letter's handwriting: the character and the strokes that form it. */
export interface LetterDuctus {
  glyph: string;
  /** In writing order. `strokes.length - 1` is the number of pen lifts. */
  strokes: Stroke[];
  /** Provenance of the stroke ORDER. Required — no letter enters uncited. */
  source: StrokeSource;
}

/**
 * The full pen path of a stroke: its segment polylines joined head-to-tail.
 *
 * The shared join point between two segments appears once, not twice, so the
 * result is the continuous curve the pen traces for that whole stroke.
 */
export function penPath(stroke: Stroke): Point[] {
  const out: Point[] = [];
  for (const seg of stroke.segments) {
    for (const p of seg.path) {
      const last = out[out.length - 1];
      if (last && last.x === p.x && last.y === p.y) continue; // drop the duplicate join
      out.push(p);
    }
  }
  return out;
}

/**
 * The gap at each segment join within a stroke: the distance from where one
 * part ends to where the next begins. A well-authored stroke has gaps of ~0 —
 * that is what "the parts connect, so the pen never lifts" means numerically.
 */
export function joinGaps(stroke: Stroke): number[] {
  const gaps: number[] = [];
  for (let i = 1; i < stroke.segments.length; i++) {
    const prev = stroke.segments[i - 1].path;
    const curr = stroke.segments[i].path;
    const a = prev[prev.length - 1];
    const b = curr[0];
    gaps.push(Math.hypot(a.x - b.x, a.y - b.y));
  }
  return gaps;
}

/** Number of times the pen leaves the paper to write this letter. */
export function penLifts(letter: LetterDuctus): number {
  return Math.max(0, letter.strokes.length - 1);
}

/**
 * SVG path data for a pen path, in FONT units (y-up). The renderer draws it
 * under one `scale(1,-1)` so it lines up with a glyph path drawn the same way —
 * a single, shared flip, so the stroke and the letter can never disagree about
 * which way is up. Pass `fraction` < 1 to draw only the first part of the
 * stroke, for a build-up where the pen advances along the path.
 */
export function penPathD(stroke: Stroke, fraction = 1): string {
  const pts = penPath(stroke);
  if (pts.length === 0) return "";
  const drawn = truncateToFraction(pts, clamp01(fraction));
  const f = (n: number) => Math.round(n * 10) / 10;
  return drawn.map((p, i) => `${i === 0 ? "M" : "L"}${f(p.x)} ${f(p.y)}`).join(" ");
}

/** The pen's position and direction at a given fraction along the stroke. */
export function penTip(stroke: Stroke, fraction: number): { at: Point; dir: Point } {
  const pts = penPath(stroke);
  const drawn = truncateToFraction(pts, clamp01(fraction));
  const at = drawn[drawn.length - 1] ?? { x: 0, y: 0 };
  const prev = drawn[drawn.length - 2] ?? at;
  return { at, dir: { x: at.x - prev.x, y: at.y - prev.y } };
}

// Truncate a polyline to the first `fraction` of its arc length, interpolating
// on the segment the cut falls in so the pen tip lands mid-stroke, not on a
// vertex.
function truncateToFraction(pts: Point[], fraction: number): Point[] {
  if (pts.length <= 1 || fraction >= 1) return pts;
  const lengths: number[] = [0];
  for (let i = 1; i < pts.length; i++) {
    lengths.push(lengths[i - 1] + Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y));
  }
  const target = lengths[lengths.length - 1] * fraction;
  const out: Point[] = [pts[0]];
  for (let i = 1; i < pts.length; i++) {
    if (lengths[i] <= target) {
      out.push(pts[i]);
    } else {
      const seg = lengths[i] - lengths[i - 1];
      const t = seg === 0 ? 0 : (target - lengths[i - 1]) / seg;
      out.push({ x: pts[i - 1].x + t * (pts[i].x - pts[i - 1].x), y: pts[i - 1].y + t * (pts[i].y - pts[i - 1].y) });
      break;
    }
  }
  return out;
}

const clamp01 = (n: number) => (n < 0 ? 0 : n > 1 ? 1 : n);

const persianAlphabetSource = (glyph: string): StrokeSource => {
  const letter = persoArabic.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Persian ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

// ---------------------------------------------------------------------------
// The authored letters.
//
// The cited sources show or number the hand movements for each letter.
// Coordinates are font units, checked in strokes.test against the rendered
// glyph (shape) and against `source` (order), not trusted because they look
// plausible here.
//
// Persian ا opens UT Austin's freehand alphabet demonstration: one vertical
// movement travels from the top to the baseline. The lesson presents the
// alphabet right-to-left, while this isolated non-connector remains one stroke.
// The adjacent ب starts at the bowl's right lip, sweeps right-to-left through
// its shallow dip, then lifts once before placing the separate dot below.
// After the intervening Persian-added پ row, ت repeats the same bowl, lifts to
// place the left dot above, then lifts again to place the right dot.
// The later س row stays pen-down: it shapes all three teeth right-to-left and
// flows directly into the final bowl as one continuous Naskh stroke.
// Near the end of the same demonstration, ل descends its tall upright and
// turns directly into the leftward base curve without lifting the chalk.
// The adjacent م forms its round head first, then continues into the long
// descending tail in the same pen-down run.
// அ is Frame 4's first row: movements 1-4 remain on the connected body, then
// the hand lifts once before movement 5 draws the separate right upright. ஆ is
// the next row: it repeats movements 1-5, then continues from that upright into
// movement 6's long-vowel loop without another lift. இ is Frame 4's third row:
// movements 1-5 form the inner and lower body, then the hand lifts once before
// movements 6-7 climb the separate outer-left start and complete its arch.
// க is Frame 3's final row: movements 1-3 make the upper frame, movements
// 4-5 make the lower-left bowl, and movement 6 makes the lower-right bowl.
// வ is Frame 9's first row: all five movements join the spiral body to its
// bottom bar and right upright without a pen lift.
// ல is Frame 9's second row: all four movements join its spiral body to the
// deep right-hand turn and rising finish without a pen lift.
// ற is Frame 10: movements 1-2 make the left arch and first middle descent,
// movement 3 restarts on the adjacent middle descent, and movements 4-5 join
// the right arch to the below-baseline sweep and descender.
// ந is Frame 12: movements 1-3 stay joined through the opening body and first
// descent; movements 4-5 restart on the middle rise and continue through the
// top bar; movement 6 restarts on the right-hand descent and tail. The cited
// looped handwriting is fitted to Noto's straighter typographic form.
// ன is Frame 13's first row: movements 1-5 join its left spiral, single inner
// arch, and top bar; movement 6 restarts on the separate right upright. ண is
// the adjacent row: movements 1-6 stay joined through the added inner arch and
// top bar, then movement 7 restarts on its separate right upright.
// ம is Frame 1's third row: all five movements join into one unbroken stroke.
// ---------------------------------------------------------------------------

export const DUCTUS: Record<string, LetterDuctus> = {
  "ا": {
    glyph: "ا",
    strokes: [
      {
        segments: [
          {
            label: "draw the tall stem downward",
            path: [
              { x: 120, y: 640 },
              { x: 120, y: 580 },
              { x: 119, y: 500 },
              { x: 124, y: 400 },
              { x: 128, y: 250 },
              { x: 129, y: 100 },
              { x: 127, y: 10 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("ا"),
  },
  "ب": {
    glyph: "ب",
    strokes: [
      {
        segments: [
          {
            label: "sweep the shallow bowl from right to left",
            path: [
              { x: 678, y: 382 },
              { x: 663, y: 345 },
              { x: 650, y: 305 },
              { x: 654, y: 260 },
              { x: 672, y: 215 },
              { x: 688, y: 170 },
              { x: 686, y: 126 },
              { x: 620, y: 94 },
              { x: 530, y: 65 },
              { x: 430, y: 42 },
              { x: 335, y: 38 },
              { x: 245, y: 51 },
              { x: 170, y: 83 },
              { x: 120, y: 135 },
              { x: 96, y: 205 },
              { x: 100, y: 255 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then place the dot below",
            path: [
              { x: 412, y: -137 },
              { x: 379, y: -101 },
              { x: 344, y: -137 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("ب"),
  },
  "ت": {
    glyph: "ت",
    strokes: [
      {
        segments: [
          {
            label: "sweep the shallow bowl from right to left",
            path: [
              { x: 678, y: 382 },
              { x: 663, y: 345 },
              { x: 650, y: 305 },
              { x: 654, y: 260 },
              { x: 672, y: 215 },
              { x: 688, y: 170 },
              { x: 686, y: 126 },
              { x: 620, y: 94 },
              { x: 530, y: 65 },
              { x: 430, y: 42 },
              { x: 335, y: 38 },
              { x: 245, y: 51 },
              { x: 170, y: 83 },
              { x: 120, y: 135 },
              { x: 96, y: 205 },
              { x: 100, y: 255 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then place the left dot above",
            path: [
              { x: 247, y: 374 },
              { x: 284, y: 412 },
              { x: 319, y: 379 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again and place the right dot",
            path: [
              { x: 395, y: 389 },
              { x: 434, y: 430 },
              { x: 470, y: 395 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("ت"),
  },
  "س": {
    glyph: "س",
    strokes: [
      {
        segments: [
          {
            label: "form the three teeth from right to left",
            path: [
              { x: 923, y: 310 },
              { x: 935, y: 120 },
              { x: 925, y: 70 },
              { x: 870, y: 45 },
              { x: 770, y: 75 },
              { x: 748, y: 110 },
              { x: 748, y: 230 },
              { x: 690, y: 65 },
              { x: 640, y: 45 },
              { x: 540, y: 55 },
              { x: 478, y: 190 },
              { x: 515, y: 20 },
            ],
          },
          {
            label: "flow into the final bowl without lifting",
            path: [
              { x: 515, y: 20 },
              { x: 515, y: -25 },
              { x: 470, y: -125 },
              { x: 370, y: -205 },
              { x: 250, y: -230 },
              { x: 145, y: -180 },
              { x: 92, y: -95 },
              { x: 110, y: 35 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("س"),
  },
  "ل": {
    glyph: "ل",
    strokes: [
      {
        segments: [
          {
            label: "draw the upright downward",
            path: [
              { x: 458, y: 640 },
              { x: 445, y: 500 },
              { x: 440, y: 420 },
              { x: 450, y: 240 },
              { x: 475, y: 80 },
              { x: 510, y: -20 },
            ],
          },
          {
            label: "turn into the base curve without lifting",
            path: [
              { x: 510, y: -20 },
              { x: 465, y: -120 },
              { x: 350, y: -205 },
              { x: 205, y: -215 },
              { x: 105, y: -135 },
              { x: 90, y: -75 },
              { x: 100, y: 25 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("ل"),
  },
  "م": {
    glyph: "م",
    strokes: [
      {
        segments: [
          {
            label: "shape the round head",
            path: [
              { x: 120, y: 210 },
              { x: 150, y: 250 },
              { x: 200, y: 300 },
              { x: 245, y: 315 },
              { x: 285, y: 300 },
              { x: 330, y: 260 },
              { x: 365, y: 215 },
              { x: 400, y: 175 },
              { x: 430, y: 150 },
            ],
          },
          {
            label: "continue down the tail without lifting",
            path: [
              { x: 430, y: 150 },
              { x: 390, y: 110 },
              { x: 330, y: 95 },
              { x: 260, y: 80 },
              { x: 180, y: 65 },
              { x: 100, y: 35 },
              { x: 90, y: -20 },
              { x: 100, y: -90 },
              { x: 110, y: -160 },
              { x: 120, y: -240 },
              { x: 105, y: -285 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("م"),
  },
  அ: {
    glyph: "அ",
    strokes: [
      {
        segments: [
          {
            label: "curl around the upper loop",
            path: [
              { x: 500, y: 510 },
              { x: 440, y: 530 },
              { x: 380, y: 500 },
              { x: 340, y: 445 },
              { x: 335, y: 385 },
              { x: 360, y: 330 },
              { x: 420, y: 285 },
              { x: 490, y: 280 },
              { x: 550, y: 310 },
              { x: 590, y: 365 },
              { x: 595, y: 430 },
            ],
          },
          {
            label: "sweep down the outer curve",
            path: [
              { x: 595, y: 430 },
              { x: 650, y: 500 },
              { x: 710, y: 460 },
              { x: 755, y: 390 },
              { x: 775, y: 300 },
              { x: 775, y: 215 },
              { x: 755, y: 130 },
              { x: 710, y: 55 },
            ],
          },
          {
            label: "turn around the lower loop",
            path: [
              { x: 710, y: 55 },
              { x: 650, y: -5 },
              { x: 570, y: -50 },
              { x: 470, y: -80 },
              { x: 350, y: -90 },
              { x: 240, y: -80 },
              { x: 150, y: -50 },
              { x: 90, y: 0 },
              { x: 70, y: 55 },
              { x: 90, y: 105 },
              { x: 145, y: 140 },
              { x: 215, y: 140 },
            ],
          },
          {
            label: "carry the horizontal to the right",
            path: [
              { x: 215, y: 140 },
              { x: 350, y: 140 },
              { x: 520, y: 140 },
              { x: 700, y: 140 },
              { x: 850, y: 140 },
              { x: 950, y: 140 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "draw the right upright down",
            path: [
              { x: 990, y: 530 },
              { x: 990, y: 390 },
              { x: 990, y: 230 },
              { x: 990, y: 70 },
              { x: 990, y: -130 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 4, அ (Univ. of Texas at Austin), p. 192",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  ஆ: {
    glyph: "ஆ",
    strokes: [
      {
        segments: [
          {
            label: "curl around the upper loop",
            path: [
              { x: 500, y: 510 },
              { x: 440, y: 530 },
              { x: 380, y: 500 },
              { x: 340, y: 445 },
              { x: 335, y: 385 },
              { x: 360, y: 330 },
              { x: 420, y: 285 },
              { x: 490, y: 280 },
              { x: 550, y: 310 },
              { x: 590, y: 365 },
              { x: 595, y: 430 },
            ],
          },
          {
            label: "sweep down the outer curve",
            path: [
              { x: 595, y: 430 },
              { x: 650, y: 500 },
              { x: 710, y: 460 },
              { x: 755, y: 390 },
              { x: 775, y: 300 },
              { x: 775, y: 215 },
              { x: 755, y: 130 },
              { x: 710, y: 55 },
            ],
          },
          {
            label: "turn around the lower loop",
            path: [
              { x: 710, y: 55 },
              { x: 650, y: -5 },
              { x: 570, y: -50 },
              { x: 470, y: -80 },
              { x: 350, y: -90 },
              { x: 240, y: -80 },
              { x: 150, y: -50 },
              { x: 90, y: 0 },
              { x: 70, y: 55 },
              { x: 90, y: 105 },
              { x: 145, y: 140 },
              { x: 215, y: 140 },
            ],
          },
          {
            label: "carry the horizontal to the right",
            path: [
              { x: 215, y: 140 },
              { x: 350, y: 140 },
              { x: 520, y: 140 },
              { x: 700, y: 140 },
              { x: 850, y: 140 },
              { x: 950, y: 140 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "draw the right upright down",
            path: [
              { x: 990, y: 530 },
              { x: 990, y: 390 },
              { x: 990, y: 230 },
              { x: 990, y: 70 },
              { x: 990, y: 50 },
            ],
          },
          {
            label: "loop the long-vowel tail to the left",
            path: [
              { x: 990, y: 50 },
              { x: 1050, y: 55 },
              { x: 1120, y: 65 },
              { x: 1200, y: 45 },
              { x: 1250, y: 10 },
              { x: 1265, y: -70 },
              { x: 1230, y: -140 },
              { x: 1180, y: -205 },
              { x: 1100, y: -260 },
              { x: 960, y: -285 },
              { x: 850, y: -280 },
              { x: 760, y: -245 },
              { x: 700, y: -200 },
              { x: 685, y: -155 },
              { x: 700, y: -105 },
              { x: 750, y: -75 },
              { x: 820, y: -75 },
              { x: 880, y: -95 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 4, ஆ (Univ. of Texas at Austin), p. 192",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  இ: {
    glyph: "இ",
    strokes: [
      {
        segments: [
          {
            label: "curl around the inner loop",
            path: [
              { x: 500, y: 510 },
              { x: 440, y: 535 },
              { x: 370, y: 510 },
              { x: 330, y: 460 },
              { x: 325, y: 400 },
              { x: 345, y: 340 },
              { x: 400, y: 290 },
              { x: 460, y: 280 },
              { x: 520, y: 300 },
              { x: 570, y: 340 },
              { x: 590, y: 390 },
            ],
          },
          {
            label: "sweep down the inner right curve",
            path: [
              { x: 590, y: 390 },
              { x: 590, y: 470 },
              { x: 630, y: 480 },
              { x: 650, y: 450 },
              { x: 720, y: 430 },
              { x: 760, y: 350 },
              { x: 760, y: 250 },
              { x: 750, y: 150 },
              { x: 720, y: 70 },
              { x: 690, y: 0 },
            ],
          },
          {
            label: "carry left and turn through the lower loop",
            path: [
              { x: 690, y: 0 },
              { x: 700, y: 90 },
              { x: 600, y: 105 },
              { x: 470, y: 110 },
              { x: 320, y: 100 },
              { x: 240, y: 80 },
              { x: 190, y: 110 },
              { x: 170, y: 80 },
              { x: 130, y: 10 },
              { x: 95, y: -60 },
              { x: 80, y: -140 },
              { x: 100, y: -210 },
              { x: 180, y: -260 },
              { x: 300, y: -285 },
              { x: 450, y: -270 },
              { x: 540, y: -200 },
            ],
          },
          {
            label: "climb the lower diagonal",
            path: [
              { x: 540, y: -200 },
              { x: 470, y: -185 },
              { x: 400, y: -150 },
              { x: 330, y: -110 },
              { x: 270, y: -60 },
              { x: 220, y: 0 },
              { x: 190, y: 80 },
              { x: 190, y: 110 },
            ],
          },
          {
            label: "carry right and turn around the lower loop",
            path: [
              { x: 190, y: 110 },
              { x: 240, y: 80 },
              { x: 320, y: 100 },
              { x: 470, y: 110 },
              { x: 600, y: 105 },
              { x: 700, y: 90 },
              { x: 780, y: 50 },
              { x: 835, y: -10 },
              { x: 850, y: -90 },
              { x: 830, y: -170 },
              { x: 790, y: -225 },
              { x: 740, y: -270 },
              { x: 670, y: -280 },
              { x: 610, y: -250 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "climb the outer left side",
            path: [
              { x: 170, y: 80 },
              { x: 170, y: 180 },
              { x: 120, y: 280 },
              { x: 120, y: 380 },
            ],
          },
          {
            label: "arch over the top and down the right",
            path: [
              { x: 120, y: 380 },
              { x: 130, y: 500 },
              { x: 200, y: 650 },
              { x: 330, y: 760 },
              { x: 500, y: 800 },
              { x: 650, y: 800 },
              { x: 800, y: 740 },
              { x: 900, y: 650 },
              { x: 970, y: 520 },
              { x: 990, y: 350 },
              { x: 990, y: 150 },
              { x: 990, y: 40 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 4, இ (Univ. of Texas at Austin), p. 192",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  க: {
    glyph: "க",
    strokes: [
      {
        segments: [
          {
            label: "climb the left upright",
            path: [
              { x: 160, y: 300 },
              { x: 160, y: 400 },
              { x: 160, y: 510 },
            ],
          },
          {
            label: "carry the top bar to the right",
            path: [
              { x: 160, y: 510 },
              { x: 300, y: 510 },
              { x: 450, y: 510 },
              { x: 580, y: 510 },
              { x: 500, y: 510 },
              { x: 420, y: 510 },
            ],
          },
          {
            label: "drop the inner upright and carry left",
            path: [
              { x: 420, y: 510 },
              { x: 420, y: 410 },
              { x: 420, y: 300 },
              { x: 300, y: 300 },
              { x: 160, y: 300 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "curve down and around the lower left",
            path: [
              { x: 420, y: 300 },
              { x: 420, y: 220 },
              { x: 440, y: 160 },
              { x: 420, y: 100 },
              { x: 380, y: 60 },
              { x: 330, y: 40 },
              { x: 280, y: 35 },
              { x: 200, y: 35 },
            ],
          },
          {
            label: "return up the outer left side",
            path: [
              { x: 200, y: 35 },
              { x: 125, y: 50 },
              { x: 75, y: 100 },
              { x: 65, y: 160 },
              { x: 80, y: 225 },
              { x: 115, y: 275 },
              { x: 160, y: 300 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "turn around the lower right bowl",
            path: [
              { x: 420, y: 300 },
              { x: 520, y: 300 },
              { x: 620, y: 285 },
              { x: 680, y: 245 },
              { x: 710, y: 185 },
              { x: 705, y: 120 },
              { x: 675, y: 70 },
              { x: 620, y: 35 },
              { x: 555, y: 25 },
              { x: 510, y: 40 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 3, க (Univ. of Texas at Austin), p. 191",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  வ: {
    glyph: "வ",
    strokes: [
      {
        segments: [
          {
            label: "curl outward and climb the left",
            path: [
              { x: 300, y: 35 },
              { x: 310, y: 55 },
              { x: 350, y: 90 },
              { x: 380, y: 145 },
              { x: 380, y: 205 },
              { x: 340, y: 260 },
              { x: 285, y: 292 },
              { x: 225, y: 290 },
              { x: 170, y: 275 },
              { x: 120, y: 275 },
              { x: 95, y: 270 },
              { x: 95, y: 180 },
              { x: 115, y: 105 },
              { x: 175, y: 45 },
              { x: 250, y: 25 },
              { x: 175, y: 45 },
              { x: 115, y: 105 },
              { x: 95, y: 180 },
              { x: 95, y: 270 },
              { x: 120, y: 360 },
              { x: 180, y: 450 },
            ],
          },
          {
            label: "arch over the top and down the right",
            path: [
              { x: 180, y: 450 },
              { x: 270, y: 525 },
              { x: 370, y: 530 },
              { x: 470, y: 500 },
              { x: 545, y: 430 },
              { x: 600, y: 340 },
              { x: 605, y: 255 },
              { x: 585, y: 175 },
              { x: 545, y: 105 },
            ],
          },
          {
            label: "turn down to the baseline",
            path: [
              { x: 545, y: 105 },
              { x: 550, y: 80 },
              { x: 555, y: 55 },
              { x: 555, y: 35 },
              { x: 515, y: 35 },
            ],
          },
          {
            label: "carry the bottom bar right",
            path: [
              { x: 515, y: 35 },
              { x: 650, y: 35 },
              { x: 780, y: 35 },
              { x: 913, y: 35 },
            ],
          },
          {
            label: "rise up the right upright",
            path: [
              { x: 913, y: 35 },
              { x: 913, y: 180 },
              { x: 913, y: 350 },
              { x: 913, y: 515 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 9, வ (Univ. of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  ல: {
    glyph: "ல",
    strokes: [
      {
        segments: [
          {
            label: "curl outward and climb the left",
            path: [
              { x: 300, y: 35 },
              { x: 310, y: 55 },
              { x: 350, y: 90 },
              { x: 380, y: 145 },
              { x: 380, y: 205 },
              { x: 340, y: 260 },
              { x: 285, y: 292 },
              { x: 225, y: 290 },
              { x: 170, y: 275 },
              { x: 120, y: 275 },
              { x: 95, y: 270 },
              { x: 95, y: 180 },
              { x: 115, y: 105 },
              { x: 175, y: 45 },
              { x: 250, y: 25 },
              { x: 175, y: 45 },
              { x: 115, y: 105 },
              { x: 95, y: 180 },
              { x: 95, y: 270 },
              { x: 120, y: 360 },
              { x: 180, y: 450 },
            ],
          },
          {
            label: "arch over and descend through the middle",
            path: [
              { x: 180, y: 450 },
              { x: 260, y: 520 },
              { x: 370, y: 560 },
              { x: 470, y: 530 },
              { x: 530, y: 480 },
              { x: 560, y: 430 },
              { x: 570, y: 350 },
              { x: 570, y: 250 },
              { x: 580, y: 160 },
              { x: 590, y: 100 },
            ],
          },
          {
            label: "turn around the deep right-hand curve",
            path: [
              { x: 590, y: 100 },
              { x: 620, y: 60 },
              { x: 690, y: 30 },
              { x: 770, y: 25 },
              { x: 840, y: 60 },
              { x: 875, y: 110 },
            ],
          },
          {
            label: "rise to the open right tip",
            path: [
              { x: 875, y: 110 },
              { x: 900, y: 200 },
              { x: 900, y: 350 },
              { x: 875, y: 450 },
              { x: 825, y: 540 },
              { x: 790, y: 580 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 9, ல (Univ. of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  ற: {
    glyph: "ற",
    strokes: [
      {
        segments: [
          {
            label: "climb the left and arch to the middle",
            path: [
              { x: 105, y: 40 },
              { x: 105, y: 200 },
              { x: 105, y: 400 },
              { x: 135, y: 485 },
              { x: 215, y: 535 },
              { x: 300, y: 530 },
              { x: 370, y: 485 },
              { x: 405, y: 430 },
            ],
          },
          {
            label: "descend the first middle upright",
            path: [
              { x: 405, y: 430 },
              { x: 405, y: 300 },
              { x: 405, y: 150 },
              { x: 405, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the second middle upright",
            path: [
              { x: 405, y: 430 },
              { x: 405, y: 300 },
              { x: 405, y: 150 },
              { x: 405, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "arch over and descend the right side",
            path: [
              { x: 420, y: 450 },
              { x: 455, y: 510 },
              { x: 540, y: 540 },
              { x: 640, y: 520 },
              { x: 710, y: 460 },
              { x: 755, y: 380 },
              { x: 760, y: 260 },
              { x: 760, y: 130 },
              { x: 750, y: 50 },
            ],
          },
          {
            label: "sweep left below the baseline and down",
            path: [
              { x: 750, y: 50 },
              { x: 730, y: 0 },
              { x: 690, y: -40 },
              { x: 620, y: -75 },
              { x: 520, y: -100 },
              { x: 400, y: -115 },
              { x: 280, y: -120 },
              { x: 160, y: -125 },
              { x: 105, y: -155 },
              { x: 105, y: -230 },
              { x: 105, y: -315 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 10, ற (Univ. of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  ந: {
    glyph: "ந",
    strokes: [
      {
        segments: [
          {
            label: "sweep from the lower-left tail around the low bowl",
            path: [
              { x: 92, y: -300 },
              { x: 92, y: -235 },
              { x: 110, y: -185 },
              { x: 160, y: -140 },
              { x: 250, y: -120 },
              { x: 360, y: -120 },
              { x: 470, y: -105 },
              { x: 570, y: -70 },
              { x: 640, y: -15 },
              { x: 690, y: 75 },
              { x: 698, y: 160 },
              { x: 680, y: 235 },
              { x: 625, y: 300 },
              { x: 550, y: 325 },
              { x: 430, y: 300 },
            ],
          },
          {
            label: "climb to the top and carry left to the first descent",
            path: [
              { x: 430, y: 300 },
              { x: 390, y: 350 },
              { x: 390, y: 420 },
              { x: 390, y: 518 },
              { x: 300, y: 518 },
              { x: 210, y: 518 },
              { x: 130, y: 518 },
            ],
          },
          {
            label: "descend the first upright",
            path: [
              { x: 130, y: 518 },
              { x: 130, y: 380 },
              { x: 130, y: 220 },
              { x: 130, y: 25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "rise on the adjacent middle upright",
            path: [
              { x: 390, y: 25 },
              { x: 390, y: 180 },
              { x: 390, y: 350 },
              { x: 390, y: 518 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 390, y: 518 },
              { x: 470, y: 518 },
              { x: 540, y: 518 },
              { x: 605, y: 518 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the right curve and sweep into the tail",
            path: [
              { x: 540, y: 325 },
              { x: 620, y: 300 },
              { x: 675, y: 235 },
              { x: 700, y: 155 },
              { x: 690, y: 75 },
              { x: 645, y: -10 },
              { x: 575, y: -70 },
              { x: 470, y: -105 },
              { x: 360, y: -120 },
              { x: 250, y: -120 },
              { x: 160, y: -140 },
              { x: 110, y: -185 },
              { x: 92, y: -235 },
              { x: 92, y: -300 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 12, ந (Univ. of Texas at Austin), p. 195",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. Frame 12 uses a looped handwritten form; the vendored Noto face straightens that family resemblance into two stems and a curled right descent, so the evidenced order is adapted to the rendered outline without inventing lifts.",
    },
  },
  ன: {
    glyph: "ன",
    strokes: [
      {
        segments: [
          {
            label: "curl outward and climb the outer left",
            path: [
              { x: 300, y: 35 },
              { x: 310, y: 55 },
              { x: 350, y: 90 },
              { x: 380, y: 145 },
              { x: 380, y: 205 },
              { x: 340, y: 260 },
              { x: 285, y: 292 },
              { x: 225, y: 290 },
              { x: 170, y: 275 },
              { x: 120, y: 275 },
              { x: 95, y: 270 },
              { x: 95, y: 180 },
              { x: 115, y: 105 },
              { x: 175, y: 45 },
              { x: 250, y: 25 },
              { x: 175, y: 45 },
              { x: 115, y: 105 },
              { x: 95, y: 180 },
              { x: 95, y: 270 },
              { x: 120, y: 360 },
              { x: 180, y: 450 },
            ],
          },
          {
            label: "arch over the left loop to the middle",
            path: [
              { x: 180, y: 450 },
              { x: 270, y: 520 },
              { x: 370, y: 530 },
              { x: 470, y: 520 },
              { x: 560, y: 500 },
              { x: 650, y: 495 },
            ],
          },
          {
            label: "descend around the single inner arch",
            path: [
              { x: 650, y: 495 },
              { x: 700, y: 455 },
              { x: 750, y: 400 },
              { x: 785, y: 320 },
              { x: 785, y: 230 },
              { x: 770, y: 140 },
              { x: 735, y: 75 },
              { x: 690, y: 35 },
              { x: 660, y: 25 },
            ],
          },
          {
            label: "turn through the bottom and climb inside",
            path: [
              { x: 660, y: 25 },
              { x: 610, y: 45 },
              { x: 555, y: 100 },
              { x: 540, y: 180 },
              { x: 545, y: 260 },
              { x: 565, y: 350 },
              { x: 600, y: 430 },
              { x: 650, y: 495 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 650, y: 495 },
              { x: 720, y: 515 },
              { x: 850, y: 518 },
              { x: 1000, y: 518 },
              { x: 1100, y: 518 },
              { x: 1210, y: 518 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the separate right upright",
            path: [
              { x: 1004, y: 480 },
              { x: 1004, y: 350 },
              { x: 1004, y: 180 },
              { x: 1004, y: 25 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 13, ன (Univ. of Texas at Austin), p. 195",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  ண: {
    glyph: "ண",
    strokes: [
      {
        segments: [
          {
            label: "curl outward and climb the outer left",
            path: [
              { x: 300, y: 35 },
              { x: 310, y: 55 },
              { x: 350, y: 90 },
              { x: 380, y: 145 },
              { x: 380, y: 205 },
              { x: 340, y: 260 },
              { x: 285, y: 292 },
              { x: 225, y: 290 },
              { x: 170, y: 275 },
              { x: 120, y: 275 },
              { x: 95, y: 270 },
              { x: 95, y: 180 },
              { x: 115, y: 105 },
              { x: 175, y: 45 },
              { x: 250, y: 25 },
              { x: 175, y: 45 },
              { x: 115, y: 105 },
              { x: 95, y: 180 },
              { x: 95, y: 270 },
              { x: 120, y: 360 },
              { x: 180, y: 450 },
            ],
          },
          {
            label: "arch over the left loop to the first junction",
            path: [
              { x: 180, y: 450 },
              { x: 270, y: 520 },
              { x: 370, y: 530 },
              { x: 470, y: 520 },
              { x: 560, y: 500 },
              { x: 650, y: 495 },
            ],
          },
          {
            label: "descend around the first inner arch",
            path: [
              { x: 650, y: 495 },
              { x: 700, y: 455 },
              { x: 750, y: 400 },
              { x: 785, y: 320 },
              { x: 785, y: 230 },
              { x: 770, y: 140 },
              { x: 735, y: 75 },
              { x: 690, y: 35 },
              { x: 660, y: 25 },
            ],
          },
          {
            label: "turn through the bottom and climb inside",
            path: [
              { x: 660, y: 25 },
              { x: 610, y: 45 },
              { x: 555, y: 100 },
              { x: 540, y: 180 },
              { x: 545, y: 260 },
              { x: 565, y: 350 },
              { x: 600, y: 430 },
              { x: 650, y: 495 },
            ],
          },
          {
            label: "sweep through the extra inner arch",
            path: [
              { x: 650, y: 495 },
              { x: 735, y: 525 },
              { x: 825, y: 530 },
              { x: 920, y: 515 },
              { x: 1000, y: 500 },
              { x: 1065, y: 495 },
              { x: 1115, y: 455 },
              { x: 1165, y: 400 },
              { x: 1200, y: 320 },
              { x: 1200, y: 230 },
              { x: 1185, y: 140 },
              { x: 1150, y: 75 },
              { x: 1105, y: 35 },
              { x: 1075, y: 25 },
              { x: 1025, y: 45 },
              { x: 970, y: 100 },
              { x: 955, y: 180 },
              { x: 960, y: 260 },
              { x: 980, y: 350 },
              { x: 1015, y: 430 },
              { x: 1065, y: 495 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 1065, y: 495 },
              { x: 1135, y: 515 },
              { x: 1260, y: 518 },
              { x: 1380, y: 518 },
              { x: 1500, y: 518 },
              { x: 1625, y: 518 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the separate right upright",
            path: [
              { x: 1418, y: 480 },
              { x: 1418, y: 350 },
              { x: 1418, y: 180 },
              { x: 1418, y: 25 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 13, ண (Univ. of Texas at Austin), p. 195",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
  ம: {
    glyph: "ம",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 110, y: 548 },
              { x: 104, y: 120 },
              { x: 104, y: 40 },
            ],
          },
          {
            label: "along the bottom",
            path: [
              { x: 104, y: 40 },
              { x: 250, y: 26 },
              { x: 430, y: 24 },
              { x: 600, y: 30 },
              { x: 715, y: 44 },
            ],
          },
          {
            label: "up the right side",
            path: [
              { x: 715, y: 44 },
              { x: 726, y: 180 },
              { x: 724, y: 430 },
            ],
          },
          {
            label: "over the top",
            path: [
              { x: 724, y: 430 },
              { x: 690, y: 510 },
              { x: 600, y: 548 },
              { x: 500, y: 548 },
              { x: 452, y: 512 },
              { x: 434, y: 430 },
            ],
          },
          {
            label: "down the middle",
            path: [
              { x: 434, y: 430 },
              { x: 430, y: 180 },
              { x: 430, y: 44 },
            ],
          },
        ],
      },
    ],
    // Five movements, matching the numbered arrows in the cited primer's
    // Frame 1 (down the left · along the bottom · up the right · over the top ·
    // down the middle) — one unbroken pen path, exactly as authored.
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 1 (Univ. of Texas at Austin)",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
};
