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
// ம first, and alone, until every letter after it is sourced the same way. It
// is ONE stroke — the hand does not lift — made of the FIVE joined parts the
// cited primer numbers: down the left upright, along the bottom, up the right
// side, over the top, and down the middle. Each part's path starts on the
// previous part's last point, so `joinGaps` is zero and the whole thing is a
// single motion. Coordinates are font units, checked in strokes.test against
// the rendered glyph (shape) and against `source` (order), not eyeballed here.
// ---------------------------------------------------------------------------

export const DUCTUS: Record<string, LetterDuctus> = {
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
