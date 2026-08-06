// ---------------------------------------------------------------------------
// ductusview.ts — turning a pen path into something a learner can watch
// ---------------------------------------------------------------------------
//
// What this file is for
// ---------------------
// `strokes.ts` knows HOW a letter is written — the pen path, its labelled
// parts, where the hand lifts. `truetype.ts` knows WHAT the letter looks like —
// the real outline, read out of the font we ship. Neither of them draws
// anything. This file is the join: it takes one letter's ductus plus that
// letter's font outline and produces a **filmstrip** — a short series of
// pictures, each one showing the letter a little further written:
//
//     ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
//     │ ▏      │  │ ▏      │  │ ▏    ▕ │  │ ▏ ⌒  ▕ │  │ ▏ ⌒▕ ▕ │
//     │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │
//     └────────┘  └────────┘  └────────┘  └────────┘  └────────┘
//      1. down     2. along    3. up the   4. over     5. down
//      the left    the bottom  right side  the top     the middle
//
// Behind every frame, in a pale grey, sits the FINISHED letter — the actual
// font outline, never a drawing of one. In front of it, in ink, sits as much of
// the pen path as the hand has travelled so far, with a dot marking where the
// pen is at that instant. So the learner sees the target and the progress at
// once, and the two can never disagree, because the target is the font.
//
// Why there is no DOM in here
// ---------------------------
// The rest of this app keeps its thinking in pure modules and its `document`
// calls in `main.ts`. This file follows that: it returns a small tree of plain
// objects (`SvgNode`) describing the picture, and a serialiser that turns that
// tree into SVG text. `main.ts` walks the same tree with `createElementNS` and
// never touches `innerHTML`; the book pipeline can take the serialised string
// instead. One description, two consumers, no `document` here — which is also
// what makes every claim below testable without a browser.
//
// The one coordinate trick, stated once
// -------------------------------------
// Font units are **y-up** (baseline at 0, ascenders positive). SVG is
// **y-down**. Two different things are drawn in this picture — the glyph
// outline from the font, and the pen path authored by hand — and BOTH are in
// font units. If each were flipped separately, a mistake in one flip would
// produce a letter and a stroke that quietly disagree about which way is up:
// the stroke would look plausible and be upside down.
//
// So there is exactly ONE flip: a single `<g transform="scale(1,-1)">` that
// contains the glyph, every pen path, and the pen dot. Nothing inside it is
// pre-flipped; nothing outside it is in font coordinates except by way of the
// viewBox. `scale(1,-1)` maps a font point (x, y) to (x, -y), which is why the
// viewBox's vertical range is the NEGATED font range — top of the letter, the
// largest font y, becomes the smallest (most negative) SVG y.
//
// Text is the exception, and deliberately so: a `<text>` inside that group
// would be drawn mirror-image. The captions therefore live OUTSIDE the flipped
// group, in the viewBox's own space, in a band reserved for them below the
// letter. That is the only reason anything here knows about the flip twice.
// ---------------------------------------------------------------------------

import {
  DUCTUS,
  penLifts,
  penPathD,
  penTip,
  type LetterDuctus,
  type Point,
  type Stroke,
} from "./strokes.ts";

// ---------------------------------------------------------------------------
// The picture, as data
// ---------------------------------------------------------------------------

/**
 * One element of an SVG picture: a tag, its attributes, optional text, and
 * optional children. Deliberately dumb — it is a description, not a renderer,
 * so the same description can become a DOM node or a string.
 */
export interface SvgNode {
  tag: string;
  attrs: Record<string, string | number>;
  /** Text content. Only leaf nodes carry it (`<title>`, `<text>`). */
  text?: string;
  children?: SvgNode[];
}

/** The glyph's real shape, as read from the font. Never hand-drawn. */
export interface GlyphOutline {
  /** SVG path data in FONT units (y-up) — `Glyph.path` from `truetype.ts`. */
  path: string;
  /** Bounding box in font units — `boundsOf(glyph.contours)`. */
  bounds: { x0: number; y0: number; x1: number; y1: number };
}

/** One frame of the filmstrip: the letter, written this far. */
export interface DuctusStep {
  /** 1-based position in the whole letter, across all strokes. */
  number: number;
  /** Which pen-down run this frame belongs to (0-based). */
  strokeIndex: number;
  /** Which labelled part of that run (0-based). */
  segmentIndex: number;
  /** The part's authored label, e.g. "down the left upright". */
  label: string;
  /** How far along `strokeIndex`'s path the pen has travelled, 0..1. */
  fraction: number;
  /**
   * True when the hand must LIFT before this frame — i.e. this frame starts a
   * new stroke and is not the first frame of the letter.
   */
  startsAfterLift: boolean;
}

/** The whole build-up: the frames, and what the learner should be told. */
export interface Filmstrip {
  glyph: string;
  steps: DuctusStep[];
  /** One `<svg>` description per step, in writing order. */
  frames: SvgNode[];
  /** Times the pen leaves the paper — `strokes.length - 1`. */
  penLifts: number;
  /** Plain-English summary, e.g. "one unbroken stroke · 5 movements". */
  summary: string;
}

/** Knobs, all in FONT units so they scale with the glyph rather than the page. */
export interface DuctusOptions {
  /** Breathing room around the ink. */
  padding?: number;
  /** Gap between the bottom of the letter and the first line of caption. */
  captionGap?: number;
  /** Width of the drawn pen path. */
  penWidth?: number;
  /** Radius of the dot showing where the pen is. */
  tipRadius?: number;
  /** Fill for the finished letter sitting behind the stroke. */
  glyphFill?: string;
  /** Colour of strokes already finished in earlier frames. */
  doneColor?: string;
  /** Colour of the stroke currently being drawn. */
  penColor?: string;
  /** Caption colour. */
  captionColor?: string;
  /** Caption size, in font units. */
  captionSize?: number;
  /** Rendered width of one frame, in CSS pixels. Height follows the aspect. */
  frameWidth?: number;
}

const DEFAULTS: Required<DuctusOptions> = {
  padding: 70,
  captionGap: 70,
  penWidth: 26,
  tipRadius: 34,
  glyphFill: "#e0ddd6",
  doneColor: "#9aa1ad",
  penColor: "#3b5bdb",
  captionColor: "#6b7280",
  captionSize: 92,
  frameWidth: 118,
};

/** Caption line spacing, as a multiple of the caption size. */
const LINE_HEIGHT = 1.22;

/**
 * A sans-serif lowercase character is roughly half an em wide on average. We
 * cannot measure text without a browser (and this module refuses to need one),
 * so wrapping uses that estimate. It only has to be close: the consequence of
 * being slightly off is a caption line a little short or a little long, not a
 * wrong letter.
 */
const AVERAGE_CHAR_WIDTH = 0.52;

// ---------------------------------------------------------------------------
// Where each labelled part ENDS, as a fraction of its stroke
// ---------------------------------------------------------------------------
//
// `penPathD(stroke, f)` draws the first `f` of a stroke by ARC LENGTH, so to
// stop a frame exactly at the end of "along the bottom" we need that part's end
// expressed as a fraction of the whole stroke's length.
//
// The subtlety: `penPath` drops the duplicated join point where one part ends
// and the next begins, so the parts' point counts do not simply add up. We
// therefore replay the exact same dedup rule here and record the index each
// part lands on, rather than measuring the parts separately and hoping the sums
// agree. Same rule, same path, same arithmetic — so a frame boundary is the
// join, not merely near it.

function joinedWithEnds(stroke: Stroke): { pts: Point[]; ends: number[] } {
  const pts: Point[] = [];
  const ends: number[] = [];
  for (const seg of stroke.segments) {
    for (const p of seg.path) {
      const last = pts[pts.length - 1];
      if (last && last.x === p.x && last.y === p.y) continue; // the shared join
      pts.push(p);
    }
    ends.push(Math.max(0, pts.length - 1));
  }
  return { pts, ends };
}

/**
 * For each labelled part of a stroke, the fraction of the stroke's total length
 * at which that part ends. Ascending; the last entry is 1 (the pen has arrived
 * at the end of the stroke). A stroke of zero length reports 1 throughout —
 * there is nothing to advance along, so every part is already "done".
 */
export function segmentEndFractions(stroke: Stroke): number[] {
  const { pts, ends } = joinedWithEnds(stroke);
  const cumulative: number[] = [0];
  for (let i = 1; i < pts.length; i++) {
    cumulative.push(cumulative[i - 1] + Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y));
  }
  const total = cumulative[cumulative.length - 1] ?? 0;
  if (total === 0) return ends.map(() => 1);
  return ends.map((i) => Math.min(1, (cumulative[i] ?? 0) / total));
}

/**
 * The letter's frames, in writing order: one per labelled part, each carrying
 * how far along its stroke the pen has reached by the end of that part.
 */
export function ductusSteps(letter: LetterDuctus): DuctusStep[] {
  const steps: DuctusStep[] = [];
  letter.strokes.forEach((stroke, strokeIndex) => {
    const fractions = segmentEndFractions(stroke);
    stroke.segments.forEach((segment, segmentIndex) => {
      steps.push({
        number: steps.length + 1,
        strokeIndex,
        segmentIndex,
        label: segment.label,
        fraction: fractions[segmentIndex] ?? 1,
        startsAfterLift: segmentIndex === 0 && strokeIndex > 0,
      });
    });
  });
  return steps;
}

// ---------------------------------------------------------------------------
// One frame
// ---------------------------------------------------------------------------

/**
 * The picture for a single step: the finished glyph in pale grey, every stroke
 * completed before this one in a settled grey, the current stroke in ink up to
 * `step.fraction`, a dot where the pen is, and the caption underneath.
 *
 * Everything geometric lives in the one `scale(1,-1)` group; the caption does
 * not, because flipped text reads backwards.
 */
export function ductusFrame(
  letter: LetterDuctus,
  glyph: GlyphOutline,
  step: DuctusStep,
  options: DuctusOptions = {},
): SvgNode {
  const o = { ...DEFAULTS, ...options };
  const box = viewBoxFor(letter, glyph, o);

  // --- inside the flip: the letter and the pen -----------------------------
  const drawn: SvgNode[] = [
    {
      tag: "path",
      attrs: {
        class: "ductus__glyph",
        d: glyph.path,
        fill: o.glyphFill,
        "fill-rule": "nonzero",
      },
    },
  ];

  for (let s = 0; s < step.strokeIndex; s++) {
    const d = penPathD(letter.strokes[s], 1);
    if (d) drawn.push(penLine(d, "ductus__done", o.doneColor, o.penWidth));
  }

  const current = letter.strokes[step.strokeIndex];
  if (current) {
    const d = penPathD(current, step.fraction);
    if (d) drawn.push(penLine(d, "ductus__pen", o.penColor, o.penWidth));
    const tip = penTip(current, step.fraction).at;
    drawn.push({
      tag: "circle",
      attrs: { class: "ductus__tip", cx: round(tip.x), cy: round(tip.y), r: o.tipRadius, fill: o.penColor },
    });
  }

  // --- outside the flip: words ---------------------------------------------
  //
  // Wrapped into `<tspan>` lines rather than left to run off the panel's edge.
  // The first baseline sits a gap below the LETTER, not below the viewBox, so
  // captions line up across a strip whose frames all share one box.
  const caption = `${step.number}. ${step.label}`;
  const lines = wrapCaption(caption, box.width, o.captionSize);
  const firstBaseline = -inkBox(letter, glyph, o).y0 + o.captionGap + o.captionSize;
  const midX = round(box.minX + box.width / 2);

  return {
    tag: "svg",
    attrs: {
      class: "ductus__frame",
      xmlns: "http://www.w3.org/2000/svg",
      viewBox: `${round(box.minX)} ${round(box.minY)} ${round(box.width)} ${round(box.height)}`,
      width: o.frameWidth,
      height: round((o.frameWidth * box.height) / box.width),
      role: "img",
      "aria-label": `${letter.glyph}, step ${step.number}: ${step.label}`,
    },
    children: [
      { tag: "title", attrs: {}, text: `${letter.glyph} — step ${step.number}: ${step.label}` },
      { tag: "g", attrs: { transform: "scale(1,-1)" }, children: drawn },
      {
        tag: "text",
        attrs: {
          class: "ductus__caption",
          x: midX,
          y: round(firstBaseline),
          "text-anchor": "middle",
          "font-size": o.captionSize,
          fill: o.captionColor,
        },
        children: lines.map((line, i) => ({
          tag: "tspan",
          attrs: { x: midX, y: round(firstBaseline + i * o.captionSize * LINE_HEIGHT) },
          text: line,
        })),
      },
    ],
  };
}

/**
 * Greedy word wrap to whatever fits the frame's width. A single word longer
 * than a line gets its own line and is allowed to overhang — better a wide word
 * than a word chopped in half, since these are teaching instructions.
 */
export function wrapCaption(text: string, width: number, captionSize: number): string[] {
  const perLine = Math.max(6, Math.floor(width / (captionSize * AVERAGE_CHAR_WIDTH)));
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(/\s+/).filter(Boolean)) {
    const candidate = line ? `${line} ${word}` : word;
    if (candidate.length > perLine && line) {
      lines.push(line);
      line = word;
    } else {
      line = candidate;
    }
  }
  if (line) lines.push(line);
  return lines.length > 0 ? lines : [""];
}

function penLine(d: string, className: string, color: string, width: number): SvgNode {
  return {
    tag: "path",
    attrs: {
      class: className,
      d,
      fill: "none",
      stroke: color,
      "stroke-width": width,
      "stroke-linecap": "round",
      "stroke-linejoin": "round",
    },
  };
}

// ---------------------------------------------------------------------------
// The whole strip
// ---------------------------------------------------------------------------

/**
 * The full build-up for a letter. `frames[i]` is the picture for `steps[i]`.
 *
 * The glyph outline is a REQUIRED argument and has no default: there is no way
 * to draw this picture without the font, and inventing a shape here is the one
 * thing this project forbids outright (see `truetype.ts`).
 */
export function ductusFilmstrip(
  letter: LetterDuctus,
  glyph: GlyphOutline,
  options: DuctusOptions = {},
): Filmstrip {
  const steps = ductusSteps(letter);
  const lifts = penLifts(letter);
  return {
    glyph: letter.glyph,
    steps,
    frames: steps.map((step) => ductusFrame(letter, glyph, step, options)),
    penLifts: lifts,
    summary: summarise(lifts, steps.length),
  };
}

function summarise(lifts: number, movements: number): string {
  const strokes = lifts + 1;
  const strokePart =
    lifts === 0 ? "one unbroken stroke" : `${strokes} strokes · ${lifts} pen ${lifts === 1 ? "lift" : "lifts"}`;
  return `${strokePart} · ${movements} ${movements === 1 ? "movement" : "movements"}`;
}

/**
 * The letter's ductus, if one has been authored and cited. Most letters have
 * none — `DUCTUS` holds only what a real source could be found for — so callers
 * must handle `undefined` and fall back to the prose stroke order rather than
 * showing an empty figure or, worse, an invented one.
 */
export function ductusFor(glyph: string): LetterDuctus | undefined {
  return Object.prototype.hasOwnProperty.call(DUCTUS, glyph) ? DUCTUS[glyph] : undefined;
}

// ---------------------------------------------------------------------------
// The viewBox: the flip, expressed once as numbers
// ---------------------------------------------------------------------------
//
// The box must hold the glyph AND the pen path, because an authored path may
// (legitimately) sit a hair outside the outline's box at a rounded end. We take
// the union, pad it, then negate the vertical range because of the flip:
//
//     font y ∈ [y0, y1]   --scale(1,-1)-->   svg y ∈ [-y1, -y0]
//
// and finally extend the bottom by the caption band, which is in SVG space and
// therefore added after the negation, not before.

function penBounds(letter: LetterDuctus): { x0: number; y0: number; x1: number; y1: number } | null {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const stroke of letter.strokes) {
    for (const segment of stroke.segments) {
      for (const p of segment.path) {
        if (p.x < x0) x0 = p.x;
        if (p.x > x1) x1 = p.x;
        if (p.y < y0) y0 = p.y;
        if (p.y > y1) y1 = p.y;
      }
    }
  }
  return x0 === Infinity ? null : { x0, y0, x1, y1 };
}

/** The padded union of glyph and pen, in FONT units (still y-up). */
function inkBox(
  letter: LetterDuctus,
  glyph: GlyphOutline,
  o: Required<DuctusOptions>,
): { x0: number; y0: number; x1: number; y1: number } {
  const pen = penBounds(letter);
  const b = glyph.bounds;
  return {
    x0: (pen ? Math.min(b.x0, pen.x0) : b.x0) - o.padding,
    x1: (pen ? Math.max(b.x1, pen.x1) : b.x1) + o.padding,
    y0: (pen ? Math.min(b.y0, pen.y0) : b.y0) - o.padding,
    y1: (pen ? Math.max(b.y1, pen.y1) : b.y1) + o.padding,
  };
}

/**
 * The frame's viewBox, in SVG (post-flip) coordinates.
 *
 * Every frame of a letter shares ONE box, so the strip reads as a sequence of
 * the same picture rather than a row of differently-cropped ones. That is why
 * the caption band is sized by the LONGEST caption in the whole letter, not by
 * the caption of any single frame.
 */
export function viewBoxFor(
  letter: LetterDuctus,
  glyph: GlyphOutline,
  options: DuctusOptions = {},
): { minX: number; minY: number; width: number; height: number } {
  const o = { ...DEFAULTS, ...options };
  const box = inkBox(letter, glyph, o);
  // A degenerate glyph (no contours, so a zero box) would give a zero-size
  // viewBox that renders nothing at all; keep it at least one unit wide/tall.
  const width = Math.max(1, box.x1 - box.x0);
  const inkHeight = Math.max(1, box.y1 - box.y0);
  const captionLines = Math.max(
    1,
    ...ductusSteps(letter).map((s) => wrapCaption(`${s.number}. ${s.label}`, width, o.captionSize).length),
  );
  const band = o.captionGap + o.captionSize * (1 + (captionLines - 1) * LINE_HEIGHT) + o.captionSize * 0.35;
  return { minX: box.x0, minY: -box.y1, width, height: inkHeight + band };
}

// ---------------------------------------------------------------------------
// Serialising: the only place a string becomes markup
// ---------------------------------------------------------------------------
//
// Every value that reaches an attribute or a text node goes through `escapeXml`
// FIRST, without exception and without asking where it came from. Today the
// only non-numeric inputs are authored labels and a font-derived path, both
// trusted — but "trusted" is a property of today's data, not of this function,
// and the point of escaping at the boundary is that it stays correct when
// tomorrow's data is a lesson file or a URL. `main.ts` builds the same tree
// through `createElementNS`/`setAttribute`, which escapes structurally; this is
// the string-building twin of that guarantee.

/** Escape the five XML metacharacters, so no value can end an attribute early. */
export function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

/** Render a node tree to SVG text. Pure — no `document`, no globals. */
export function svgMarkup(node: SvgNode): string {
  const attrs = Object.entries(node.attrs)
    .map(([k, v]) => ` ${k}="${escapeXml(String(v))}"`)
    .join("");
  const inner =
    (node.text !== undefined ? escapeXml(node.text) : "") +
    (node.children ?? []).map(svgMarkup).join("");
  return inner === "" ? `<${node.tag}${attrs}/>` : `<${node.tag}${attrs}>${inner}</${node.tag}>`;
}

const round = (n: number) => Math.round(n * 10) / 10;
