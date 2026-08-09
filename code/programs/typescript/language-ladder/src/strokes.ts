// ---------------------------------------------------------------------------
// strokes.ts — how a letter is written, as a path the pen actually travels
// ---------------------------------------------------------------------------
//
// The writing lessons teach a learner to FORM a letter, which means teaching
// the motion of the pen: where it starts, which way it goes, and — the part a
// static picture of the finished letter cannot show — where it stays down and
// where (if ever) it lifts.
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
 * source, and `variation` records that Tamil is taught with school-to-school
 * differences — this is one attested order, not the only one.
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

// ---------------------------------------------------------------------------
// The authored letters.
//
// The cited primer numbers the hand movements for each letter. Coordinates are
// font units, checked in strokes.test against the rendered glyph (shape) and
// against `source` (order), not trusted because they look plausible here.
//
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
// ம is Frame 1's third row: all five movements join into one unbroken stroke.
// ---------------------------------------------------------------------------

export const DUCTUS: Record<string, LetterDuctus> = {
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
