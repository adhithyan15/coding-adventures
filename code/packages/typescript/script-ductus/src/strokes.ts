// ---------------------------------------------------------------------------
// strokes.ts — how a letter is written, as a path the pen actually travels
// ---------------------------------------------------------------------------
//
// The writing lessons teach a learner to FORM a letter, which means teaching
// the motion of the pen: where it starts, which way it goes, and — the part a
// static picture of the finished letter cannot show — where it stays down and
// where (if ever) it lifts.

import arabic from "../../../../learning/human-languages/data/scripts/arabic.json";
import chinese from "../../../../learning/human-languages/data/scripts/chinese.json";
import cyrillic from "../../../../learning/human-languages/data/scripts/cyrillic.json";
import devanagari from "../../../../learning/human-languages/data/scripts/devanagari.json";
import gujarati from "../../../../learning/human-languages/data/scripts/gujarati.json";
import hebrew from "../../../../learning/human-languages/data/scripts/hebrew.json";
import persoArabic from "../../../../learning/human-languages/data/scripts/perso-arabic.json";
import urduNastaliq from "../../../../learning/human-languages/data/scripts/urdu-nastaliq.json";
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
  /** Canonical script id. Glyphs shared by multiple scripts need distinct identities. */
  script: string;
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

const arabicAlphabetSource = (glyph: string): StrokeSource => {
  const letter = arabic.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Arabic ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const chineseCharacterSource = (glyph: string): StrokeSource => {
  const letter = chinese.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Chinese ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const cyrillicAlphabetSource = (glyph: string): StrokeSource => {
  const letter = cyrillic.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Cyrillic ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const devanagariAlphabetSource = (glyph: string): StrokeSource => {
  const letter = devanagari.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Devanagari ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const gujaratiAlphabetSource = (glyph: string): StrokeSource => {
  const letter = gujarati.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Gujarati ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const hebrewAlphabetSource = (glyph: string): StrokeSource => {
  const letter = hebrew.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Hebrew ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const persianAlphabetSource = (glyph: string): StrokeSource => {
  const letter = persoArabic.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Persian ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const urduAlphabetSource = (glyph: string): StrokeSource => {
  const letter = urduNastaliq.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Urdu ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

/** Collision-safe key for a glyph whose Unicode character is shared by scripts. */
export const ductusKey = (script: string, glyph: string): string => `${script}:${glyph}`;

// ---------------------------------------------------------------------------
// The authored letters.
//
// The cited sources show or number the hand movements for each letter.
// Coordinates are font units, checked in strokes.test against the rendered
// glyph (shape) and against `source` (order), not trusted because they look
// plausible here.
//
// RussianIrina's all-letter school-hand video forms lowercase Cyrillic а as
// one continuous round loop and right-hand finishing stem. The source's
// handwritten form is single-storey; this learner path preserves that one-run
// motion while widening the entry to cover Noto Sans Cyrillic's extra printed
// upper shoulder before circling the lower bowl and descending the right stem.
// The same lesson writes lowercase Cyrillic б without lifting: a
// counterclockwise lower body closes before the pen rises into its rightward
// top flag. The handwritten diagonal transition is routed through Noto Sans
// Cyrillic's upper-left printed shoulder so the fitted path stays on its ink.
// Arabic ا opens the smallest remaining starter inventory from the University
// of Oregon's instructional video: one top-to-bottom movement with no lift.
// Its scoped key preserves Arabic provenance separately from the Persian and
// Urdu records for the same Unicode glyph while sharing the Noto Naskh shape.
// The adjacent Arabic ب demonstration sweeps its bowl right-to-left, turns up
// at the left tip, then lifts once to place the dot below. Its Arabic-scoped
// source remains separate from Persian ب while sharing the checked Noto shape.
// Arabic ت reuses that separately demonstrated bowl because its own clip opens
// with the body already complete, then places the left and right upper dots as
// separate strokes. Its scoped source remains distinct from Persian ت.
// The linked Arabic ث asset is actually another ت lesson: it draws only two
// upper dots, so it cannot support a three-dot path. The next viable source,
// Arabic ج, draws the short head left-to-right, continues around the bowl, then
// lifts once for its dot. That body-first order stays distinct from Urdu ج.
// The page's unlinked Arabic ح attachment instead opens while a short left stem
// is descending, then restarts near that stem's top and sweeps around the
// dotless bowl. Its one verified lift is not inferred from adjacent Jeem.
// The same page's Arabic خ clip returns to a body-first order: the short head
// flows directly around the bowl, then one lift precedes the upper dot. That
// order comes from Khaa's own clip rather than being copied from ج or ح.
// The next page's Arabic د clip starts at the independent form's upper tip,
// descends down-right through its curved shoulder, and turns left along the
// baseline in the same pen-down run. Its one-way-connector context stays
// explicit, but the path does not infer motion from a contextual form.
// The same page's Arabic ر clip begins at the independent form's upper tip,
// descends through its short stroke, and sweeps left through the lower curve
// without lifting. Its scoped source stays distinct from Urdu ر even though
// both paths fit the same vendored Noto Naskh glyph.
// The next page's Arabic س clip shapes all three close teeth right-to-left and
// flows directly into the final bowl in one uninterrupted run. Its scoped
// evidence stays distinct from the already-authored Persian and Urdu س paths.
// The page's Arabic ش clip repeats that complete body, then separately places
// the lower-left, lower-right, and centered upper dots. Its three verified
// lifts stay scoped separately from Urdu ش even though the geometry agrees.
// The page's Arabic ص clip closes its oval and rises into the short shoulder in
// one run, then restarts once at the baseline junction for the trailing bowl.
// The pause and restart keep Saad's path distinct from the adjacent س and ش.
// The page's embedded Arabic ض lesson repeats those two Saad body runs, then
// lifts again to place the upper dot last. Its independently observed order is
// recorded even though the directly linked short MOV was unavailable at audit.
// The next source page's Arabic ع clip begins at the upper-right tip, shapes the
// open head, and flows directly down and around the lower bowl without lifting.
// Its independent form stays distinct from the adjacent dotted Ghayn lesson.
// The next page's Arabic ك clip descends the main upright and turns left along
// the baseline without lifting, then restarts once for the inner arm. Its
// Arabic identity stays distinct from the separately sourced Urdu ک glyph.
// The same page's Arabic ل clip keeps the tall upright and leftward base bowl
// in one pen-down run. Its scoped source remains distinct from the Persian and
// Urdu records even though all three share the same Unicode glyph and outline.
// The later Arabic ه clip starts at the independent form's upper right, closes
// its lower counter, threads through the centre into the upper-right counter,
// then sweeps left along the baseline without lifting. The compact handwritten
// order is fitted to Noto Naskh's wider two-counter outline and remains scoped
// independently from the Persian record for the same Unicode glyph.
// The adjacent Arabic و clip begins at the small head's lower-right junction,
// sweeps left around the loop, then continues down and left through the tail
// without lifting. Its one-way-connector context and Arabic source remain
// distinct from the Persian record for the same Unicode glyph and outline.
// Persian ا opens UT Austin's freehand alphabet demonstration: one vertical
// movement travels from the top to the baseline. The lesson presents the
// alphabet right-to-left, while this isolated non-connector remains one stroke.
// Urdu ا has the same fallback-font geometry but a distinct source and identity:
// Zer o Zabar's independent animation travels top-to-bottom in one continuous
// stroke, explicitly unlike the bottom-to-top final-form animation beside it.
// Urdu ج starts with its dot, then restarts once on the independent body: the
// pointed hooked head turns into the descent and bowl without another lift.
// The textbook's separately animated flat head is an aesthetic alternative.
// Urdu ر descends first, then bends left without lifting. The source contrasts
// that independent run with the sharper lower-left drop of the final form and
// records a separate Naskh/Nastaliq difference for final re.
// Urdu س keeps the standard three close teeth and final bowl in one pen-down
// run. The textbook also records a toothless long curve as an optional,
// especially handwritten alternative; this learner path uses the canonical
// toothed Noto Naskh fallback form shown in both independent animations.
// Urdu ش repeats that complete body before three separate dot strokes: lower
// left, lower right, then the centered upper dot. The chapter keeps the same
// optional toothless body while requiring the dots to remain centered above.
// Urdu ک writes the independent main-line body first: descend the stem, sweep
// left through its flatter bowl, and finish with the hook. After one lift, the
// long upper-right slash descends toward the stem as a separate stroke.
// Urdu ل begins at the top of its tall independent upright, descends through
// the baseline, and continues below it around the leftward bowl in one run.
// Urdu م keeps its round head and below-baseline tail in one run. The source
// contrasts the calligraphic head with handwriting's counterclockwise loop;
// this path reconciles their shared head-to-tail motion with Noto Naskh.
// Urdu ن draws its independent below-baseline bowl first, then lifts once for
// the dot near the baseline. Initial and medial forms use a distinct tooth.
// Urdu ہ starts at the independent teardrop's upper right, loops
// counterclockwise around its base, and crosses the top without lifting.
// Urdu ے begins at the independent form's upper right, sweeps left across its
// broad bowl, curls back underneath at the far left, and continues right along
// the lower fold in one uninterrupted stroke.
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
// The next ن sweeps its bowl right-to-left, then lifts once to place the dot.
// Contrary to the old queue, the source then demonstrates و before ه: its
// small head loops and flows into the leftward curving tail without a lift.
// The later isolated ه closes one simple handwritten loop without lifting; its
// one pen-down path is fitted to Noto Naskh's wider two-counter isolated form.
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
  // Hanzi Writer Data's ordered medians draw 人 with the left-falling stroke
  // first, then restart at the central junction for the right-falling stroke.
  // The source's Arphic-derived proportions are fitted to the vendored Noto
  // Sans SC outline while preserving both directions and the intervening lift.
  [ductusKey("chinese", "人")]: {
    script: "chinese",
    glyph: "人",
    strokes: [
      {
        segments: [
          {
            label: "draw the left-falling piě stroke from the upper centre",
            path: [
              { x: 500, y: 810 },
              { x: 500, y: 740 },
              { x: 490, y: 650 },
              { x: 470, y: 555 },
              { x: 445, y: 465 },
              { x: 410, y: 375 },
              { x: 365, y: 285 },
              { x: 310, y: 200 },
              { x: 245, y: 120 },
              { x: 175, y: 55 },
              { x: 105, y: 5 },
              { x: 65, y: -25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the right-falling nà stroke from the junction",
            path: [
              { x: 500, y: 690 },
              { x: 515, y: 620 },
              { x: 535, y: 535 },
              { x: 565, y: 445 },
              { x: 605, y: 355 },
              { x: 655, y: 265 },
              { x: 715, y: 180 },
              { x: 785, y: 105 },
              { x: 860, y: 45 },
              { x: 925, y: 0 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("人"),
  },
  // The compressed person radical keeps the source dataset's two-run order:
  // a long left-falling stroke, then a separately started vertical. Its Noto
  // Sans SC fit follows the glyph's narrow left-side proportions rather than
  // mechanically squeezing the full 人 path.
  [ductusKey("chinese", "亻")]: {
    script: "chinese",
    glyph: "亻",
    strokes: [
      {
        segments: [
          {
            label: "draw the left-falling piě stroke from upper right to lower left",
            path: [
              { x: 440, y: 820 },
              { x: 430, y: 790 },
              { x: 415, y: 755 },
              { x: 395, y: 720 },
              { x: 375, y: 680 },
              { x: 350, y: 640 },
              { x: 325, y: 600 },
              { x: 295, y: 560 },
              { x: 265, y: 520 },
              { x: 230, y: 475 },
              { x: 195, y: 435 },
              { x: 160, y: 395 },
              { x: 125, y: 360 },
              { x: 95, y: 330 },
              { x: 75, y: 305 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the vertical shù stroke from the junction to the baseline",
            path: [
              { x: 310, y: 590 },
              { x: 310, y: 550 },
              { x: 310, y: 500 },
              { x: 310, y: 440 },
              { x: 310, y: 370 },
              { x: 310, y: 295 },
              { x: 310, y: 220 },
              { x: 310, y: 140 },
              { x: 310, y: 60 },
              { x: 310, y: -50 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("亻"),
  },
  // 口 establishes the first Chinese joined corner in the authored inventory:
  // descend the left side, join the top and right side in one héngzhé run, then
  // close the bottom last. The flat Noto fit preserves those three source runs.
  [ductusKey("chinese", "口")]: {
    script: "chinese",
    glyph: "口",
    strokes: [
      {
        segments: [
          {
            label: "draw the left vertical shù stroke from top to bottom",
            path: [
              { x: 166, y: 700 },
              { x: 166, y: 620 },
              { x: 166, y: 530 },
              { x: 166, y: 440 },
              { x: 166, y: 350 },
              { x: 166, y: 260 },
              { x: 166, y: 170 },
              { x: 166, y: 80 },
              { x: 166, y: -35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the top bar from left to right",
            path: [
              { x: 166, y: 700 },
              { x: 260, y: 700 },
              { x: 360, y: 700 },
              { x: 470, y: 700 },
              { x: 580, y: 700 },
              { x: 690, y: 700 },
              { x: 785, y: 700 },
              { x: 835, y: 700 },
            ],
          },
          {
            label: "turn the corner without lifting and descend the right side",
            path: [
              { x: 835, y: 700 },
              { x: 835, y: 610 },
              { x: 835, y: 520 },
              { x: 835, y: 430 },
              { x: 835, y: 340 },
              { x: 835, y: 250 },
              { x: 835, y: 160 },
              { x: 835, y: 70 },
              { x: 835, y: -30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then close the bottom from left to right",
            path: [
              { x: 166, y: 70 },
              { x: 260, y: 70 },
              { x: 360, y: 70 },
              { x: 470, y: 70 },
              { x: 580, y: 70 },
              { x: 690, y: 70 },
              { x: 785, y: 70 },
              { x: 835, y: 70 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("口"),
  },
  // 女 begins with one bent piědiǎn run: descend down-left, turn at the lower
  // junction, and sweep down-right without lifting. A separately started
  // left-falling piě comes next, then the middle héng crosses left-to-right.
  // The four movements follow the three pinned medians on the Noto Sans SC fit.
  [ductusKey("chinese", "女")]: {
    script: "chinese",
    glyph: "女",
    strokes: [
      {
        segments: [
          {
            label: "draw the first piědiǎn stroke down and left",
            path: [
              { x: 460, y: 840 },
              { x: 440, y: 790 },
              { x: 415, y: 720 },
              { x: 390, y: 650 },
              { x: 365, y: 580 },
              { x: 340, y: 510 },
              { x: 310, y: 440 },
              { x: 285, y: 375 },
              { x: 255, y: 320 },
              { x: 220, y: 275 },
            ],
          },
          {
            label: "turn without lifting and sweep down to the lower right",
            path: [
              { x: 220, y: 275 },
              { x: 300, y: 265 },
              { x: 400, y: 220 },
              { x: 500, y: 175 },
              { x: 600, y: 125 },
              { x: 700, y: 75 },
              { x: 800, y: 20 },
              { x: 890, y: -35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the left-falling piě stroke from upper right to lower left",
            path: [
              { x: 717, y: 550 },
              { x: 700, y: 490 },
              { x: 680, y: 430 },
              { x: 650, y: 360 },
              { x: 615, y: 295 },
              { x: 570, y: 235 },
              { x: 520, y: 180 },
              { x: 460, y: 125 },
              { x: 390, y: 75 },
              { x: 310, y: 30 },
              { x: 220, y: -10 },
              { x: 130, y: -45 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng from left to right",
            path: [
              { x: 70, y: 561 },
              { x: 180, y: 561 },
              { x: 300, y: 561 },
              { x: 420, y: 561 },
              { x: 540, y: 561 },
              { x: 660, y: 561 },
              { x: 780, y: 561 },
              { x: 890, y: 561 },
              { x: 940, y: 561 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("女"),
  },
  // 子 has two joined turns across its first two strokes: the top horizontal
  // turns down-left, then a separately started vertical hooks left at the base.
  // A second lift precedes the final middle horizontal from left to right.
  [ductusKey("chinese", "子")]: {
    script: "chinese",
    glyph: "子",
    strokes: [
      {
        segments: [
          {
            label: "draw the top horizontal héng from left to right",
            path: [
              { x: 160, y: 735 },
              { x: 250, y: 735 },
              { x: 350, y: 735 },
              { x: 450, y: 735 },
              { x: 550, y: 735 },
              { x: 650, y: 735 },
              { x: 740, y: 735 },
              { x: 790, y: 735 },
            ],
          },
          {
            label: "turn without lifting and sweep down-left",
            path: [
              { x: 790, y: 735 },
              { x: 750, y: 680 },
              { x: 700, y: 640 },
              { x: 650, y: 600 },
              { x: 600, y: 565 },
              { x: 550, y: 535 },
              { x: 490, y: 515 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the central vertical",
            path: [
              { x: 504, y: 530 },
              { x: 504, y: 460 },
              { x: 504, y: 380 },
              { x: 504, y: 300 },
              { x: 504, y: 220 },
              { x: 504, y: 140 },
              { x: 504, y: 70 },
              { x: 500, y: 20 },
              { x: 500, y: -35 },
            ],
          },
          {
            label: "hook left at the base without lifting",
            path: [
              { x: 500, y: -35 },
              { x: 450, y: -40 },
              { x: 390, y: -40 },
              { x: 330, y: -35 },
              { x: 285, y: -20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng from left to right",
            path: [
              { x: 60, y: 357 },
              { x: 170, y: 357 },
              { x: 290, y: 357 },
              { x: 410, y: 357 },
              { x: 530, y: 357 },
              { x: 650, y: 357 },
              { x: 770, y: 357 },
              { x: 890, y: 357 },
              { x: 945, y: 357 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("子"),
  },
  // 日 starts with the left side, then joins the top bar to the right side in
  // one héngzhé stroke. The inside bar precedes a separately closing bottom.
  [ductusKey("chinese", "日")]: {
    script: "chinese",
    glyph: "日",
    strokes: [
      {
        segments: [
          {
            label: "descend the left vertical shù from top to bottom",
            path: [
              { x: 214, y: 735 },
              { x: 214, y: 630 },
              { x: 214, y: 520 },
              { x: 214, y: 410 },
              { x: 214, y: 300 },
              { x: 214, y: 190 },
              { x: 214, y: 80 },
              { x: 214, y: 0 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the top horizontal héng from left to right",
            path: [
              { x: 214, y: 735 },
              { x: 310, y: 735 },
              { x: 410, y: 735 },
              { x: 510, y: 735 },
              { x: 610, y: 735 },
              { x: 710, y: 735 },
              { x: 792, y: 735 },
            ],
          },
          {
            label: "turn without lifting and descend the right side",
            path: [
              { x: 792, y: 735 },
              { x: 792, y: 630 },
              { x: 792, y: 520 },
              { x: 792, y: 410 },
              { x: 792, y: 300 },
              { x: 792, y: 190 },
              { x: 792, y: 80 },
              { x: 792, y: 0 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng from left to right",
            path: [
              { x: 214, y: 389 },
              { x: 310, y: 389 },
              { x: 410, y: 389 },
              { x: 510, y: 389 },
              { x: 610, y: 389 },
              { x: 710, y: 389 },
              { x: 792, y: 389 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then close the bottom horizontal héng from left to right",
            path: [
              { x: 214, y: 33 },
              { x: 310, y: 33 },
              { x: 410, y: 33 },
              { x: 510, y: 33 },
              { x: 610, y: 33 },
              { x: 710, y: 33 },
              { x: 792, y: 33 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("日"),
  },
  // 讠 starts with a down-right dot. After one lift, the short horizontal,
  // vertical descent, and rising finish stay joined inside one second stroke.
  [ductusKey("chinese", "讠")]: {
    script: "chinese",
    glyph: "讠",
    strokes: [
      {
        segments: [
          {
            label: "draw the top dot down and right",
            path: [
              { x: 150, y: 780 },
              { x: 180, y: 755 },
              { x: 215, y: 720 },
              { x: 250, y: 685 },
              { x: 290, y: 645 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the short horizontal from left to right",
            path: [
              { x: 60, y: 492 },
              { x: 110, y: 492 },
              { x: 160, y: 492 },
              { x: 210, y: 492 },
              { x: 255, y: 492 },
              { x: 293, y: 492 },
            ],
          },
          {
            label: "turn without lifting and descend the vertical",
            path: [
              { x: 293, y: 492 },
              { x: 293, y: 410 },
              { x: 293, y: 320 },
              { x: 293, y: 230 },
              { x: 293, y: 140 },
              { x: 293, y: 60 },
              { x: 293, y: 20 },
            ],
          },
          {
            label: "turn without lifting and rise to the upper right",
            path: [
              { x: 293, y: 20 },
              { x: 330, y: 35 },
              { x: 370, y: 60 },
              { x: 410, y: 85 },
              { x: 445, y: 110 },
              { x: 475, y: 140 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("讠"),
  },
  // 氵 stacks two separately drawn down-right dots above a third stroke that
  // begins at the bottom and rises to the upper right. The three pinned
  // medians remain separate while fitting the narrow Noto Sans SC radical.
  [ductusKey("chinese", "氵")]: {
    script: "chinese",
    glyph: "氵",
    strokes: [
      {
        segments: [
          {
            label: "draw the upper dot down and right",
            path: [
              { x: 155, y: 785 },
              { x: 195, y: 770 },
              { x: 235, y: 745 },
              { x: 275, y: 720 },
              { x: 315, y: 695 },
              { x: 350, y: 675 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle dot down and right",
            path: [
              { x: 72, y: 515 },
              { x: 110, y: 505 },
              { x: 150, y: 485 },
              { x: 190, y: 465 },
              { x: 230, y: 445 },
              { x: 270, y: 420 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then begin the bottom stroke with a slight rise left",
            path: [
              { x: 158, y: -58 },
              { x: 150, y: -32 },
              { x: 155, y: 0 },
            ],
          },
          {
            label: "continue without lifting in a long rise to the upper right",
            path: [
              { x: 155, y: 0 },
              { x: 185, y: 45 },
              { x: 220, y: 95 },
              { x: 255, y: 145 },
              { x: 290, y: 195 },
              { x: 325, y: 245 },
              { x: 360, y: 295 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("氵"),
  },
  // 宀 places its top dot first, then a separate down-left stroke on the left.
  // After the second lift, the roof crosses left-to-right and hooks down-left
  // without breaking. The Noto fit keeps that source order and joined hook.
  [ductusKey("chinese", "宀")]: {
    script: "chinese",
    glyph: "宀",
    strokes: [
      {
        segments: [
          {
            label: "draw the top dot down and right",
            path: [
              { x: 440, y: 805 },
              { x: 455, y: 790 },
              { x: 470, y: 770 },
              { x: 485, y: 750 },
              { x: 500, y: 730 },
              { x: 515, y: 715 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the left-side stroke down and left",
            path: [
              { x: 150, y: 660 },
              { x: 145, y: 625 },
              { x: 138, y: 585 },
              { x: 130, y: 545 },
              { x: 122, y: 505 },
              { x: 112, y: 475 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the horizontal roof from left to right",
            path: [
              { x: 150, y: 646 },
              { x: 250, y: 646 },
              { x: 360, y: 646 },
              { x: 470, y: 646 },
              { x: 580, y: 646 },
              { x: 690, y: 646 },
              { x: 790, y: 646 },
              { x: 875, y: 646 },
            ],
          },
          {
            label: "hook down and left without lifting",
            path: [
              { x: 875, y: 646 },
              { x: 880, y: 620 },
              { x: 875, y: 585 },
              { x: 865, y: 545 },
              { x: 850, y: 505 },
              { x: 833, y: 475 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("宀"),
  },
  // 你 writes 亻 first, then the five strokes of 尔: a falling stroke, a
  // joined horizontal hook, a joined vertical hook, and two separate dots.
  // The seven Noto-fitted runs preserve that component order and six lifts.
  [ductusKey("chinese", "你")]: {
    script: "chinese",
    glyph: "你",
    strokes: [
      {
        segments: [
          {
            label: "draw the left-falling stroke of the person radical",
            path: [
              { x: 300, y: 810 },
              { x: 285, y: 760 },
              { x: 265, y: 705 },
              { x: 240, y: 650 },
              { x: 210, y: 595 },
              { x: 175, y: 540 },
              { x: 140, y: 495 },
              { x: 105, y: 455 },
              { x: 70, y: 425 },
              { x: 45, y: 410 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the vertical stroke of the person radical",
            path: [
              { x: 196, y: 605 },
              { x: 196, y: 520 },
              { x: 196, y: 430 },
              { x: 196, y: 340 },
              { x: 196, y: 250 },
              { x: 196, y: 160 },
              { x: 196, y: 70 },
              { x: 196, y: -50 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the upper-right left-falling stroke",
            path: [
              { x: 500, y: 810 },
              { x: 490, y: 765 },
              { x: 475, y: 715 },
              { x: 455, y: 660 },
              { x: 430, y: 605 },
              { x: 405, y: 550 },
              { x: 375, y: 500 },
              { x: 345, y: 455 },
              { x: 325, y: 435 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the upper horizontal from left to right",
            path: [
              { x: 450, y: 612 },
              { x: 530, y: 612 },
              { x: 620, y: 612 },
              { x: 710, y: 612 },
              { x: 800, y: 612 },
              { x: 890, y: 612 },
            ],
          },
          {
            label: "hook down and left without lifting",
            path: [
              { x: 890, y: 612 },
              { x: 900, y: 580 },
              { x: 900, y: 540 },
              { x: 895, y: 500 },
              { x: 885, y: 465 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the central vertical",
            path: [
              { x: 649, y: 590 },
              { x: 649, y: 500 },
              { x: 649, y: 400 },
              { x: 649, y: 300 },
              { x: 649, y: 200 },
              { x: 649, y: 100 },
              { x: 649, y: 15 },
              { x: 645, y: -30 },
            ],
          },
          {
            label: "hook left at the base without lifting",
            path: [
              { x: 645, y: -30 },
              { x: 615, y: -40 },
              { x: 580, y: -42 },
              { x: 545, y: -40 },
              { x: 515, y: -30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the lower-left dot down and left",
            path: [
              { x: 485, y: 380 },
              { x: 470, y: 330 },
              { x: 450, y: 275 },
              { x: 425, y: 220 },
              { x: 400, y: 165 },
              { x: 370, y: 110 },
              { x: 345, y: 80 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the lower-right dot down and right",
            path: [
              { x: 790, y: 380 },
              { x: 815, y: 335 },
              { x: 840, y: 285 },
              { x: 865, y: 235 },
              { x: 885, y: 185 },
              { x: 900, y: 135 },
              { x: 915, y: 90 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("你"),
  },
  // 好 writes all three strokes of 女 before the three strokes of 子. The
  // first strokes of both components turn without lifting, and 子's vertical
  // keeps its base hook joined. Six Noto-fitted runs preserve five lifts.
  [ductusKey("chinese", "好")]: {
    script: "chinese",
    glyph: "好",
    strokes: [
      {
        segments: [
          {
            label: "draw 女's first bent stroke down and left",
            path: [
              { x: 218, y: 820 },
              { x: 205, y: 750 },
              { x: 190, y: 675 },
              { x: 175, y: 600 },
              { x: 155, y: 520 },
              { x: 135, y: 440 },
              { x: 120, y: 365 },
              { x: 100, y: 320 },
              { x: 82, y: 300 },
            ],
          },
          {
            label: "turn without lifting and sweep right",
            path: [
              { x: 82, y: 300 },
              { x: 145, y: 270 },
              { x: 205, y: 225 },
              { x: 265, y: 175 },
              { x: 325, y: 120 },
              { x: 375, y: 70 },
              { x: 410, y: 40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 女's left-falling stroke",
            path: [
              { x: 390, y: 620 },
              { x: 380, y: 550 },
              { x: 365, y: 475 },
              { x: 345, y: 395 },
              { x: 320, y: 310 },
              { x: 290, y: 225 },
              { x: 255, y: 150 },
              { x: 215, y: 80 },
              { x: 165, y: 20 },
              { x: 95, y: -45 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 女's horizontal stroke left to right",
            path: [
              { x: 45, y: 600 },
              { x: 100, y: 600 },
              { x: 160, y: 600 },
              { x: 220, y: 600 },
              { x: 280, y: 600 },
              { x: 335, y: 600 },
              { x: 370, y: 600 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 子's top horizontal left to right",
            path: [
              { x: 485, y: 730 },
              { x: 555, y: 730 },
              { x: 630, y: 730 },
              { x: 705, y: 730 },
              { x: 780, y: 730 },
              { x: 850, y: 730 },
            ],
          },
          {
            label: "turn without lifting and sweep down-left",
            path: [
              { x: 850, y: 730 },
              { x: 840, y: 700 },
              { x: 820, y: 665 },
              { x: 790, y: 625 },
              { x: 755, y: 585 },
              { x: 715, y: 545 },
              { x: 680, y: 520 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend 子's vertical stroke",
            path: [
              { x: 700, y: 520 },
              { x: 700, y: 440 },
              { x: 700, y: 350 },
              { x: 700, y: 260 },
              { x: 700, y: 170 },
              { x: 700, y: 80 },
              { x: 700, y: 15 },
              { x: 695, y: -25 },
            ],
          },
          {
            label: "hook left at the base without lifting",
            path: [
              { x: 695, y: -25 },
              { x: 665, y: -35 },
              { x: 625, y: -40 },
              { x: 585, y: -38 },
              { x: 545, y: -25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 子's middle horizontal left to right",
            path: [
              { x: 440, y: 380 },
              { x: 520, y: 380 },
              { x: 610, y: 380 },
              { x: 700, y: 380 },
              { x: 790, y: 380 },
              { x: 880, y: 380 },
              { x: 950, y: 380 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("好"),
  },
  // 我 has seven sourced strokes. Only the vertical and its base hook remain
  // joined; the long curved slash also hooks upward without lifting, producing
  // nine visible movements and six pen lifts.
  [ductusKey("chinese", "我")]: {
    script: "chinese",
    glyph: "我",
    strokes: [
      { segments: [{ label: "draw the short upper-left falling stroke", path: [
        { x: 450, y: 800 }, { x: 390, y: 785 }, { x: 325, y: 770 },
        { x: 255, y: 755 }, { x: 185, y: 740 }, { x: 105, y: 720 },
      ] }] },
      { segments: [{ label: "lift, then draw the upper horizontal left to right", path: [
        { x: 65, y: 510 }, { x: 180, y: 510 }, { x: 300, y: 510 },
        { x: 420, y: 510 }, { x: 540, y: 510 }, { x: 660, y: 510 },
        { x: 780, y: 510 }, { x: 900, y: 510 }, { x: 940, y: 510 },
      ] }] },
      { segments: [
        { label: "lift, then descend the vertical stroke", path: [
          { x: 307, y: 720 }, { x: 307, y: 620 }, { x: 307, y: 520 },
          { x: 307, y: 420 }, { x: 307, y: 320 }, { x: 307, y: 220 },
          { x: 307, y: 120 }, { x: 307, y: 20 }, { x: 302, y: -25 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 302, y: -25 }, { x: 275, y: -35 }, { x: 235, y: -40 },
          { x: 195, y: -38 }, { x: 155, y: -25 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw the lower rising stroke", path: [
        { x: 55, y: 215 }, { x: 120, y: 230 }, { x: 190, y: 245 },
        { x: 265, y: 262 }, { x: 340, y: 280 }, { x: 415, y: 298 },
        { x: 490, y: 315 },
      ] }] },
      { segments: [
        { label: "lift, then draw the long curved slash down and right", path: [
          { x: 600, y: 810 }, { x: 600, y: 700 }, { x: 605, y: 590 },
          { x: 615, y: 480 }, { x: 635, y: 365 }, { x: 660, y: 255 },
          { x: 700, y: 150 }, { x: 750, y: 65 }, { x: 805, y: 5 },
          { x: 850, y: -35 }, { x: 875, y: -45 },
        ] },
        { label: "hook upward on the right without lifting", path: [
          { x: 875, y: -45 }, { x: 895, y: 5 }, { x: 905, y: 55 },
          { x: 915, y: 105 }, { x: 925, y: 145 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw the separate rising slash up and left", path: [
        { x: 850, y: 390 }, { x: 815, y: 325 }, { x: 770, y: 260 },
        { x: 720, y: 200 }, { x: 660, y: 140 }, { x: 595, y: 85 },
        { x: 525, y: 35 }, { x: 455, y: -5 },
      ] }] },
      { segments: [{ label: "lift, then place the upper-right dot down and right", path: [
        { x: 755, y: 785 }, { x: 785, y: 755 }, { x: 815, y: 720 },
        { x: 845, y: 685 }, { x: 875, y: 650 }, { x: 895, y: 625 },
      ] }] },
    ],
    source: chineseCharacterSource("我"),
  },
  // 是 closes 日 in four strokes before drawing the five-stroke lower body.
  // Only 日's top-right corner remains joined: nine strokes, eight lifts, and
  // ten visible movements on the Noto Sans SC fit.
  [ductusKey("chinese", "是")]: {
    script: "chinese",
    glyph: "是",
    strokes: [
      { segments: [{ label: "draw 日's left vertical", path: [
        { x: 200, y: 770 }, { x: 200, y: 710 }, { x: 200, y: 650 },
        { x: 200, y: 590 }, { x: 200, y: 530 }, { x: 200, y: 490 },
      ] }] },
      { segments: [
        { label: "lift, then draw 日's top horizontal", path: [
          { x: 200, y: 760 }, { x: 300, y: 760 }, { x: 400, y: 760 },
          { x: 500, y: 760 }, { x: 600, y: 760 }, { x: 700, y: 760 },
          { x: 795, y: 760 },
        ] },
        { label: "turn down the right side without lifting", path: [
          { x: 795, y: 760 }, { x: 795, y: 700 }, { x: 795, y: 640 },
          { x: 795, y: 580 }, { x: 795, y: 520 }, { x: 795, y: 490 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 日's inner horizontal", path: [
          { x: 235, y: 634 }, { x: 330, y: 634 }, { x: 430, y: 634 },
          { x: 530, y: 634 }, { x: 630, y: 634 }, { x: 730, y: 634 },
          { x: 760, y: 634 },
      ] }] },
      { segments: [{ label: "lift, then close 日 with the bottom horizontal", path: [
        { x: 235, y: 500 }, { x: 330, y: 500 }, { x: 430, y: 500 },
        { x: 530, y: 500 }, { x: 630, y: 500 }, { x: 730, y: 500 },
        { x: 760, y: 500 },
      ] }] },
      { segments: [{ label: "lift, then draw the wide middle horizontal", path: [
        { x: 65, y: 365 }, { x: 180, y: 365 }, { x: 300, y: 365 },
        { x: 420, y: 365 }, { x: 540, y: 365 }, { x: 660, y: 365 },
        { x: 780, y: 365 }, { x: 900, y: 365 }, { x: 940, y: 365 },
      ] }] },
      { segments: [{ label: "lift, then descend the central vertical", path: [
        { x: 508, y: 350 }, { x: 508, y: 300 }, { x: 508, y: 245 },
        { x: 508, y: 190 }, { x: 508, y: 130 }, { x: 508, y: 70 },
        { x: 508, y: 10 },
      ] }] },
      { segments: [{ label: "lift, then draw the short lower-right horizontal", path: [
        { x: 510, y: 185 }, { x: 580, y: 185 }, { x: 650, y: 185 },
        { x: 720, y: 185 }, { x: 790, y: 185 }, { x: 850, y: 185 },
      ] }] },
      { segments: [{ label: "lift, then draw the lower-left falling stroke", path: [
        { x: 265, y: 280 }, { x: 250, y: 230 }, { x: 230, y: 180 },
        { x: 205, y: 130 }, { x: 175, y: 85 }, { x: 140, y: 45 },
        { x: 100, y: 10 }, { x: 60, y: -25 },
      ] }] },
      { segments: [{ label: "lift, then draw the long finishing stroke down and right", path: [
        { x: 245, y: 180 }, { x: 280, y: 140 }, { x: 320, y: 105 },
        { x: 370, y: 65 }, { x: 430, y: 25 }, { x: 500, y: -5 },
        { x: 580, y: -25 }, { x: 670, y: -25 }, { x: 760, y: -25 },
        { x: 850, y: -25 }, { x: 920, y: -20 },
      ] }] },
    ],
    source: chineseCharacterSource("是"),
  },
  // 不 places four independent strokes: top horizontal, long falling stroke,
  // central vertical, then the right-falling dot. Four strokes mean three
  // lifts and four visible movements on the Noto Sans SC fit.
  [ductusKey("chinese", "不")]: {
    script: "chinese",
    glyph: "不",
    strokes: [
      { segments: [{ label: "draw the top horizontal left-to-right", path: [
        { x: 85, y: 730 }, { x: 200, y: 730 }, { x: 320, y: 730 },
        { x: 440, y: 730 }, { x: 560, y: 730 }, { x: 680, y: 730 },
        { x: 800, y: 730 }, { x: 915, y: 730 },
      ] }] },
      { segments: [{ label: "lift, then draw the long stroke down-left", path: [
        { x: 545, y: 710 }, { x: 525, y: 650 }, { x: 490, y: 590 },
        { x: 440, y: 525 }, { x: 380, y: 460 }, { x: 315, y: 395 },
        { x: 245, y: 330 }, { x: 175, y: 275 }, { x: 105, y: 225 },
      ] }] },
      { segments: [{ label: "lift, then descend the central vertical", path: [
        { x: 500, y: 550 }, { x: 500, y: 475 }, { x: 500, y: 400 },
        { x: 500, y: 325 }, { x: 500, y: 250 }, { x: 500, y: 175 },
        { x: 500, y: 100 }, { x: 500, y: 20 }, { x: 500, y: -55 },
      ] }] },
      { segments: [{ label: "lift, then draw the separate right-falling dot", path: [
        { x: 610, y: 470 }, { x: 660, y: 430 }, { x: 715, y: 390 },
        { x: 770, y: 350 }, { x: 825, y: 305 }, { x: 875, y: 260 },
        { x: 920, y: 220 },
      ] }] },
    ],
    source: chineseCharacterSource("不"),
  },
  // 名 completes 夕 before 口. The second 夕 stroke joins its horizontal to the
  // long down-left fall, and 口 joins its top to the right side: six strokes,
  // five lifts, and eight visible movements on the Noto Sans SC fit.
  [ductusKey("chinese", "名")]: {
    script: "chinese",
    glyph: "名",
    strokes: [
      { segments: [{ label: "draw 夕's upper left-falling stroke", path: [
        { x: 445, y: 820 }, { x: 420, y: 775 }, { x: 385, y: 730 },
        { x: 340, y: 685 }, { x: 285, y: 640 }, { x: 225, y: 600 },
        { x: 165, y: 560 }, { x: 105, y: 525 },
      ] }] },
      { segments: [
        { label: "lift, then draw 夕's horizontal", path: [
          { x: 350, y: 705 }, { x: 440, y: 705 }, { x: 530, y: 705 },
          { x: 620, y: 705 }, { x: 710, y: 705 }, { x: 775, y: 705 },
        ] },
        { label: "continue down-left without lifting", path: [
          { x: 775, y: 705 }, { x: 745, y: 650 }, { x: 700, y: 590 },
          { x: 640, y: 530 }, { x: 570, y: 470 }, { x: 490, y: 415 },
          { x: 400, y: 360 }, { x: 305, y: 315 }, { x: 205, y: 275 },
          { x: 110, y: 240 },
        ] },
      ] },
      { segments: [{ label: "lift, then place 夕's inner down-right dot", path: [
        { x: 300, y: 540 }, { x: 330, y: 515 }, { x: 365, y: 490 },
        { x: 400, y: 460 }, { x: 435, y: 430 }, { x: 470, y: 400 },
      ] }] },
      { segments: [{ label: "lift, then descend 口's left side", path: [
        { x: 290, y: 305 }, { x: 290, y: 245 }, { x: 290, y: 185 },
        { x: 290, y: 125 }, { x: 290, y: 65 }, { x: 290, y: 5 },
        { x: 290, y: -50 },
      ] }] },
      { segments: [
        { label: "lift, then draw 口's top horizontal", path: [
          { x: 300, y: 305 }, { x: 385, y: 305 }, { x: 470, y: 305 },
          { x: 555, y: 305 }, { x: 640, y: 305 }, { x: 725, y: 305 },
          { x: 810, y: 305 },
        ] },
        { label: "turn down the right side without lifting", path: [
          { x: 810, y: 305 }, { x: 810, y: 245 }, { x: 810, y: 185 },
          { x: 810, y: 125 }, { x: 810, y: 65 }, { x: 810, y: 5 },
          { x: 810, y: -50 },
        ] },
      ] },
      { segments: [{ label: "lift, then close 口 with the bottom horizontal", path: [
        { x: 300, y: 5 }, { x: 385, y: 5 }, { x: 470, y: 5 },
        { x: 555, y: 5 }, { x: 640, y: 5 }, { x: 725, y: 5 },
        { x: 800, y: 5 },
      ] }] },
    ],
    source: chineseCharacterSource("名"),
  },
  // 字 writes 宀 before 子. The roof ends in one joined hook; 子 then keeps its
  // top turn and vertical base hook joined: six strokes, five lifts, and nine
  // visible movements on the Noto Sans SC fit.
  [ductusKey("chinese", "字")]: {
    script: "chinese",
    glyph: "字",
    strokes: [
      { segments: [{ label: "draw 宀's top dot down-right", path: [
        { x: 455, y: 825 }, { x: 475, y: 800 }, { x: 495, y: 775 },
        { x: 520, y: 750 }, { x: 545, y: 725 },
      ] }] },
      { segments: [{ label: "lift, then draw 宀's left-side stroke down-left", path: [
        { x: 125, y: 690 }, { x: 120, y: 650 }, { x: 115, y: 610 },
        { x: 105, y: 570 }, { x: 95, y: 535 },
      ] }] },
      { segments: [
        { label: "lift, then draw 宀's horizontal roof", path: [
          { x: 140, y: 700 }, { x: 250, y: 700 }, { x: 360, y: 700 },
          { x: 470, y: 700 }, { x: 580, y: 700 }, { x: 690, y: 700 },
          { x: 800, y: 700 }, { x: 880, y: 700 },
        ] },
        { label: "hook down-left without lifting", path: [
          { x: 880, y: 700 }, { x: 875, y: 660 }, { x: 865, y: 620 },
          { x: 855, y: 580 }, { x: 850, y: 545 },
        ] },
      ] },
      { segments: [
        { label: "lift, then draw 子's top horizontal", path: [
          { x: 260, y: 515 }, { x: 345, y: 515 }, { x: 430, y: 515 },
          { x: 515, y: 515 }, { x: 600, y: 515 }, { x: 685, y: 515 },
          { x: 735, y: 515 },
        ] },
        { label: "turn down-left without lifting", path: [
          { x: 735, y: 515 }, { x: 700, y: 480 }, { x: 660, y: 445 },
          { x: 615, y: 410 }, { x: 570, y: 380 }, { x: 525, y: 350 },
          { x: 490, y: 330 },
        ] },
      ] },
      { segments: [
        { label: "lift, then descend 子's vertical", path: [
          { x: 500, y: 350 }, { x: 500, y: 290 }, { x: 500, y: 230 },
          { x: 500, y: 170 }, { x: 500, y: 110 }, { x: 500, y: 50 },
          { x: 500, y: 5 },
        ] },
        { label: "hook left without lifting", path: [
          { x: 500, y: 5 }, { x: 480, y: -20 }, { x: 450, y: -35 },
          { x: 410, y: -40 }, { x: 365, y: -40 }, { x: 325, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 子's middle horizontal", path: [
        { x: 85, y: 265 }, { x: 200, y: 265 }, { x: 315, y: 265 },
        { x: 430, y: 265 }, { x: 545, y: 265 }, { x: 660, y: 265 },
        { x: 775, y: 265 }, { x: 900, y: 265 },
      ] }] },
    ],
    source: chineseCharacterSource("字"),
  },
  // 谢 writes 讠, then 身, then 寸. Its twelve cited strokes preserve the two
  // turns in 讠's second run, 身's two-turn enclosure, and 寸's base hook:
  // twelve strokes, eleven lifts, and seventeen visible movements.
  [ductusKey("chinese", "谢")]: {
    script: "chinese",
    glyph: "谢",
    strokes: [
      { segments: [{ label: "draw 讠's top dot down-right", path: [
        { x: 90, y: 780 }, { x: 120, y: 755 }, { x: 150, y: 725 },
        { x: 185, y: 690 }, { x: 225, y: 650 },
      ] }] },
      { segments: [
        { label: "lift, then draw 讠's short horizontal", path: [
          { x: 50, y: 490 }, { x: 85, y: 490 }, { x: 120, y: 490 },
          { x: 155, y: 490 }, { x: 195, y: 490 },
        ] },
        { label: "turn down without lifting", path: [
          { x: 195, y: 490 }, { x: 195, y: 400 }, { x: 195, y: 300 },
          { x: 195, y: 200 }, { x: 195, y: 100 }, { x: 195, y: 10 },
        ] },
        { label: "turn and finish rising up-right without lifting", path: [
          { x: 195, y: 10 }, { x: 225, y: 25 }, { x: 255, y: 50 },
          { x: 285, y: 80 }, { x: 315, y: 115 }, { x: 335, y: 145 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 身's upper falling stroke", path: [
        { x: 505, y: 825 }, { x: 500, y: 790 }, { x: 485, y: 755 },
        { x: 465, y: 720 }, { x: 440, y: 685 },
      ] }] },
      { segments: [{ label: "lift, then descend 身's left side", path: [
        { x: 375, y: 680 }, { x: 375, y: 600 }, { x: 375, y: 520 },
        { x: 375, y: 440 }, { x: 375, y: 360 }, { x: 375, y: 285 },
      ] }] },
      { segments: [
        { label: "lift, then draw 身's top horizontal", path: [
          { x: 405, y: 695 }, { x: 450, y: 695 }, { x: 495, y: 695 },
          { x: 540, y: 695 }, { x: 580, y: 695 },
        ] },
        { label: "turn and descend 身's right side without lifting", path: [
          { x: 580, y: 695 }, { x: 580, y: 575 }, { x: 580, y: 455 },
          { x: 580, y: 335 }, { x: 580, y: 215 }, { x: 580, y: 95 },
          { x: 580, y: 10 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 580, y: 10 }, { x: 565, y: -10 }, { x: 540, y: -25 },
          { x: 510, y: -35 }, { x: 475, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 身's upper inner horizontal", path: [
        { x: 390, y: 565 }, { x: 430, y: 565 }, { x: 470, y: 565 },
        { x: 510, y: 565 }, { x: 550, y: 565 }, { x: 580, y: 565 },
      ] }] },
      { segments: [{ label: "lift, then draw 身's lower inner horizontal", path: [
        { x: 390, y: 430 }, { x: 430, y: 430 }, { x: 470, y: 430 },
        { x: 510, y: 430 }, { x: 550, y: 430 }, { x: 580, y: 430 },
      ] }] },
      { segments: [{ label: "lift, then draw 身's wide lower horizontal", path: [
        { x: 290, y: 285 }, { x: 350, y: 285 }, { x: 410, y: 285 },
        { x: 470, y: 285 }, { x: 530, y: 285 }, { x: 585, y: 285 },
      ] }] },
      { segments: [{ label: "lift, then draw 身's lower falling stroke down-left", path: [
        { x: 535, y: 270 }, { x: 510, y: 220 }, { x: 480, y: 170 },
        { x: 440, y: 120 }, { x: 395, y: 75 }, { x: 345, y: 30 },
        { x: 295, y: -10 },
      ] }] },
      { segments: [{ label: "lift, then draw 寸's horizontal", path: [
        { x: 650, y: 585 }, { x: 700, y: 585 }, { x: 750, y: 585 },
        { x: 800, y: 585 }, { x: 850, y: 585 }, { x: 900, y: 585 },
        { x: 950, y: 585 },
      ] }] },
      { segments: [
        { label: "lift, then descend 寸's vertical", path: [
          { x: 855, y: 825 }, { x: 855, y: 700 }, { x: 855, y: 575 },
          { x: 855, y: 450 }, { x: 855, y: 325 }, { x: 855, y: 200 },
          { x: 855, y: 75 }, { x: 855, y: 5 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 855, y: 5 }, { x: 840, y: -15 }, { x: 815, y: -30 },
          { x: 785, y: -40 }, { x: 750, y: -40 }, { x: 720, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then place 寸's dot down-right", path: [
        { x: 680, y: 430 }, { x: 700, y: 390 }, { x: 720, y: 350 },
        { x: 740, y: 310 }, { x: 760, y: 270 },
      ] }] },
    ],
    source: chineseCharacterSource("谢"),
  },
  // 请 writes 讠 before 青. The speech radical keeps both turns inside its
  // second run; 青 closes with a joined top, right side, and leftward base hook:
  // ten strokes, nine lifts, and fourteen visible movements.
  [ductusKey("chinese", "请")]: {
    script: "chinese",
    glyph: "请",
    strokes: [
      { segments: [{ label: "draw 讠's top dot down-right", path: [
        { x: 135, y: 780 }, { x: 165, y: 750 }, { x: 195, y: 715 },
        { x: 225, y: 680 }, { x: 255, y: 650 },
      ] }] },
      { segments: [
        { label: "lift, then draw 讠's short horizontal", path: [
          { x: 45, y: 490 }, { x: 80, y: 490 }, { x: 120, y: 490 },
          { x: 160, y: 490 }, { x: 200, y: 490 }, { x: 235, y: 490 },
        ] },
        { label: "turn down without lifting", path: [
          { x: 235, y: 490 }, { x: 235, y: 400 }, { x: 235, y: 300 },
          { x: 235, y: 200 }, { x: 235, y: 100 }, { x: 235, y: 10 },
        ] },
        { label: "turn and finish rising up-right without lifting", path: [
          { x: 235, y: 10 }, { x: 265, y: 25 }, { x: 295, y: 50 },
          { x: 330, y: 80 }, { x: 360, y: 110 }, { x: 390, y: 145 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 青's top horizontal", path: [
        { x: 385, y: 735 }, { x: 475, y: 735 }, { x: 565, y: 735 },
        { x: 655, y: 735 }, { x: 745, y: 735 }, { x: 835, y: 735 },
        { x: 925, y: 735 },
      ] }] },
      { segments: [{ label: "lift, then draw 青's second horizontal", path: [
        { x: 410, y: 610 }, { x: 490, y: 610 }, { x: 570, y: 610 },
        { x: 650, y: 610 }, { x: 730, y: 610 }, { x: 810, y: 610 },
        { x: 895, y: 610 },
      ] }] },
      { segments: [{ label: "lift, then descend 青's upper vertical", path: [
        { x: 650, y: 835 }, { x: 650, y: 765 }, { x: 650, y: 695 },
        { x: 650, y: 625 }, { x: 650, y: 555 }, { x: 650, y: 485 },
      ] }] },
      { segments: [{ label: "lift, then draw 青's wide middle horizontal", path: [
        { x: 355, y: 485 }, { x: 455, y: 485 }, { x: 555, y: 485 },
        { x: 655, y: 485 }, { x: 755, y: 485 }, { x: 855, y: 485 },
        { x: 955, y: 485 },
      ] }] },
      { segments: [{ label: "lift, then descend 青's lower left side", path: [
        { x: 460, y: 370 }, { x: 460, y: 295 }, { x: 460, y: 220 },
        { x: 460, y: 145 }, { x: 460, y: 70 }, { x: 460, y: -5 },
        { x: 460, y: -70 },
      ] }] },
      { segments: [
        { label: "lift, then draw 青's lower top horizontal", path: [
          { x: 490, y: 370 }, { x: 550, y: 370 }, { x: 610, y: 370 },
          { x: 670, y: 370 }, { x: 730, y: 370 }, { x: 790, y: 370 },
          { x: 845, y: 370 },
        ] },
        { label: "turn and descend the right side without lifting", path: [
          { x: 845, y: 370 }, { x: 845, y: 295 }, { x: 845, y: 220 },
          { x: 845, y: 145 }, { x: 845, y: 70 }, { x: 845, y: 5 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 845, y: 5 }, { x: 830, y: -15 }, { x: 805, y: -30 },
          { x: 775, y: -40 }, { x: 740, y: -40 }, { x: 705, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 青's upper inner horizontal", path: [
        { x: 480, y: 235 }, { x: 540, y: 235 }, { x: 600, y: 235 },
        { x: 660, y: 235 }, { x: 720, y: 235 }, { x: 780, y: 235 },
        { x: 825, y: 235 },
      ] }] },
      { segments: [{ label: "lift, then draw 青's lower inner horizontal", path: [
        { x: 480, y: 100 }, { x: 540, y: 100 }, { x: 600, y: 100 },
        { x: 660, y: 100 }, { x: 720, y: 100 }, { x: 780, y: 100 },
        { x: 825, y: 100 },
      ] }] },
    ],
    source: chineseCharacterSource("请"),
  },
  // 再 opens with the upper horizontal, then builds the central frame before
  // closing with the long bottom bar: six strokes, five lifts, eight movements.
  [ductusKey("chinese", "再")]: {
    script: "chinese",
    glyph: "再",
    strokes: [
      { segments: [{ label: "draw the top horizontal left-to-right", path: [
        { x: 80, y: 745 }, { x: 220, y: 745 }, { x: 360, y: 745 },
        { x: 500, y: 745 }, { x: 640, y: 745 }, { x: 780, y: 745 },
        { x: 920, y: 745 },
      ] }] },
      { segments: [{ label: "lift, then descend the left side", path: [
        { x: 195, y: 575 }, { x: 195, y: 475 }, { x: 195, y: 375 },
        { x: 195, y: 275 }, { x: 195, y: 175 }, { x: 195, y: 75 },
        { x: 195, y: -70 },
      ] }] },
      { segments: [
        { label: "lift, then draw the frame's top horizontal", path: [
          { x: 225, y: 575 }, { x: 315, y: 575 }, { x: 405, y: 575 },
          { x: 495, y: 575 }, { x: 585, y: 575 }, { x: 675, y: 575 },
          { x: 800, y: 575 },
        ] },
        { label: "turn and descend the right side without lifting", path: [
          { x: 800, y: 575 }, { x: 800, y: 475 }, { x: 800, y: 375 },
          { x: 800, y: 275 }, { x: 800, y: 175 }, { x: 800, y: 75 },
          { x: 800, y: 10 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 800, y: 10 }, { x: 785, y: -10 }, { x: 760, y: -25 },
          { x: 730, y: -35 }, { x: 695, y: -35 }, { x: 655, y: -30 },
        ] },
      ] },
      { segments: [{ label: "lift, then descend the central vertical", path: [
        { x: 495, y: 755 }, { x: 495, y: 665 }, { x: 495, y: 575 },
        { x: 495, y: 485 }, { x: 495, y: 395 }, { x: 495, y: 305 },
        { x: 495, y: 205 },
      ] }] },
      { segments: [{ label: "lift, then draw the inner horizontal", path: [
        { x: 210, y: 390 }, { x: 310, y: 390 }, { x: 410, y: 390 },
        { x: 510, y: 390 }, { x: 610, y: 390 }, { x: 710, y: 390 },
        { x: 790, y: 390 },
      ] }] },
      { segments: [{ label: "lift, then close with the long bottom horizontal", path: [
        { x: 45, y: 195 }, { x: 195, y: 195 }, { x: 345, y: 195 },
        { x: 495, y: 195 }, { x: 645, y: 195 }, { x: 795, y: 195 },
        { x: 955, y: 195 },
      ] }] },
    ],
    source: chineseCharacterSource("再"),
  },
  // 见 completes its open upper frame before drawing the two lower runs:
  // four strokes, three lifts, seven movements.
  [ductusKey("chinese", "见")]: {
    script: "chinese",
    glyph: "见",
    strokes: [
      { segments: [{ label: "descend the frame's left side", path: [
        { x: 215, y: 755 }, { x: 215, y: 670 }, { x: 215, y: 585 },
        { x: 215, y: 500 }, { x: 215, y: 415 }, { x: 215, y: 330 },
        { x: 215, y: 235 },
      ] }] },
      { segments: [
        { label: "lift, then draw the frame's top horizontal", path: [
          { x: 220, y: 745 }, { x: 310, y: 745 }, { x: 400, y: 745 },
          { x: 490, y: 745 }, { x: 580, y: 745 }, { x: 670, y: 745 },
          { x: 780, y: 745 },
        ] },
        { label: "turn and descend the right side without lifting", path: [
          { x: 780, y: 745 }, { x: 780, y: 660 }, { x: 780, y: 575 },
          { x: 780, y: 490 }, { x: 780, y: 405 }, { x: 780, y: 320 },
          { x: 780, y: 235 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw the inner left-falling leg", path: [
        { x: 490, y: 600 }, { x: 490, y: 520 }, { x: 485, y: 430 },
        { x: 475, y: 340 }, { x: 450, y: 250 }, { x: 420, y: 170 },
        { x: 380, y: 100 }, { x: 325, y: 45 }, { x: 260, y: 0 },
        { x: 180, y: -40 }, { x: 90, y: -65 },
      ] }] },
      { segments: [
        { label: "lift, then descend the second leg", path: [
          { x: 555, y: 285 }, { x: 555, y: 235 }, { x: 555, y: 185 },
          { x: 555, y: 135 }, { x: 555, y: 85 }, { x: 555, y: 55 },
        ] },
        { label: "bend right along the base without lifting", path: [
          { x: 555, y: 55 }, { x: 570, y: 20 }, { x: 610, y: -10 },
          { x: 665, y: -20 }, { x: 725, y: -20 }, { x: 785, y: -15 },
          { x: 835, y: 5 }, { x: 885, y: 40 },
        ] },
        { label: "finish with an upward hook without lifting", path: [
          { x: 885, y: 40 }, { x: 895, y: 70 }, { x: 905, y: 100 },
          { x: 915, y: 125 }, { x: 925, y: 145 }, { x: 930, y: 150 },
        ] },
      ] },
    ],
    source: chineseCharacterSource("见"),
  },
  // 什 completes both strokes of 亻 before writing 十: four separate strokes,
  // three lifts, and four movements.
  [ductusKey("chinese", "什")]: {
    script: "chinese",
    glyph: "什",
    strokes: [
      { segments: [{ label: "draw 亻's left-falling stroke from the upper centre down-left", path: [
        { x: 280, y: 810 }, { x: 265, y: 760 }, { x: 245, y: 700 },
        { x: 220, y: 640 }, { x: 190, y: 580 }, { x: 155, y: 525 },
        { x: 120, y: 480 }, { x: 85, y: 450 }, { x: 50, y: 430 },
      ] }] },
      { segments: [{ label: "lift, then descend 亻's vertical stroke to the baseline", path: [
        { x: 225, y: 590 }, { x: 225, y: 480 }, { x: 225, y: 370 },
        { x: 225, y: 260 }, { x: 225, y: 150 }, { x: 225, y: 40 },
        { x: 225, y: -65 },
      ] }] },
      { segments: [{ label: "lift, then draw 十's horizontal stroke left-to-right", path: [
        { x: 340, y: 457 }, { x: 440, y: 457 }, { x: 540, y: 457 },
        { x: 640, y: 457 }, { x: 740, y: 457 }, { x: 840, y: 457 },
        { x: 940, y: 457 },
      ] }] },
      { segments: [{ label: "lift, then descend 十's vertical stroke through the horizontal", path: [
        { x: 646, y: 810 }, { x: 646, y: 680 }, { x: 646, y: 550 },
        { x: 646, y: 420 }, { x: 646, y: 290 }, { x: 646, y: 160 },
        { x: 646, y: 30 }, { x: 646, y: -65 },
      ] }] },
    ],
    source: chineseCharacterSource("什"),
  },
  // 么 places its upper falling stroke, joins the second fall to its rightward
  // base sweep, then adds the final dot: three strokes, two lifts, four movements.
  [ductusKey("chinese", "么")]: {
    script: "chinese",
    glyph: "么",
    strokes: [
      { segments: [{ label: "draw the upper left-falling stroke down-left", path: [
        { x: 475, y: 805 }, { x: 455, y: 755 }, { x: 420, y: 700 },
        { x: 375, y: 640 }, { x: 325, y: 580 }, { x: 270, y: 520 },
        { x: 215, y: 470 }, { x: 165, y: 430 }, { x: 120, y: 400 },
        { x: 75, y: 410 },
      ] }] },
      { segments: [
        { label: "lift, then draw the second left-falling stroke down-left", path: [
          { x: 650, y: 580 }, { x: 620, y: 520 }, { x: 575, y: 450 },
          { x: 520, y: 375 }, { x: 455, y: 300 }, { x: 390, y: 225 },
          { x: 325, y: 155 }, { x: 260, y: 95 }, { x: 205, y: 50 },
          { x: 175, y: 30 },
        ] },
        { label: "turn and sweep right along the base without lifting", path: [
          { x: 175, y: 30 }, { x: 270, y: 35 }, { x: 380, y: 45 },
          { x: 490, y: 55 }, { x: 600, y: 70 }, { x: 705, y: 85 },
          { x: 805, y: 105 },
        ] },
      ] },
      { segments: [{ label: "lift, then place the final dot down-right", path: [
        { x: 670, y: 295 }, { x: 715, y: 245 }, { x: 760, y: 185 },
        { x: 805, y: 125 }, { x: 845, y: 65 }, { x: 885, y: 5 },
        { x: 905, y: -30 },
      ] }] },
    ],
    source: chineseCharacterSource("么"),
  },
  // 早 completes 日 before writing 十 below it. The top and right sides of 日
  // stay joined: six strokes, five lifts, and seven learner movements.
  [ductusKey("chinese", "早")]: {
    script: "chinese",
    glyph: "早",
    strokes: [
      { segments: [{ label: "descend 日's left side from the upper left", path: [
        { x: 189, y: 759 }, { x: 189, y: 690 }, { x: 189, y: 620 },
        { x: 189, y: 550 }, { x: 189, y: 480 }, { x: 189, y: 412 },
      ] }] },
      { segments: [
        { label: "lift, then draw 日's top horizontal left-to-right", path: [
          { x: 189, y: 759 }, { x: 290, y: 759 }, { x: 395, y: 759 },
          { x: 500, y: 759 }, { x: 605, y: 759 }, { x: 710, y: 759 },
          { x: 806, y: 759 },
        ] },
        { label: "turn without lifting and descend 日's right side", path: [
          { x: 806, y: 759 }, { x: 806, y: 690 }, { x: 806, y: 620 },
          { x: 806, y: 550 }, { x: 806, y: 480 }, { x: 806, y: 412 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 日's middle horizontal left-to-right", path: [
        { x: 189, y: 587 }, { x: 290, y: 587 }, { x: 395, y: 587 },
        { x: 500, y: 587 }, { x: 605, y: 587 }, { x: 710, y: 587 },
        { x: 806, y: 587 },
      ] }] },
      { segments: [{ label: "lift, then close 日 with its bottom horizontal left-to-right", path: [
        { x: 189, y: 412 }, { x: 290, y: 412 }, { x: 395, y: 412 },
        { x: 500, y: 412 }, { x: 605, y: 412 }, { x: 710, y: 412 },
        { x: 806, y: 412 },
      ] }] },
      { segments: [{ label: "lift, then draw 十's horizontal left-to-right", path: [
        { x: 60, y: 193 }, { x: 190, y: 193 }, { x: 345, y: 193 },
        { x: 500, y: 193 }, { x: 655, y: 193 }, { x: 810, y: 193 },
        { x: 944, y: 193 },
      ] }] },
      { segments: [{ label: "lift, then descend 十's vertical through the horizontal", path: [
        { x: 496, y: 389 }, { x: 496, y: 310 }, { x: 496, y: 230 },
        { x: 496, y: 150 }, { x: 496, y: 70 }, { x: 496, y: -65 },
      ] }] },
    ],
    source: chineseCharacterSource("早"),
  },
  // 上 descends its vertical first, then places the short middle horizontal
  // before the long base: three separate strokes, two lifts, three movements.
  [ductusKey("chinese", "上")]: {
    script: "chinese",
    glyph: "上",
    strokes: [
      { segments: [{ label: "descend the central vertical from top to bottom", path: [
        { x: 466, y: 810 }, { x: 466, y: 680 }, { x: 466, y: 550 },
        { x: 466, y: 420 }, { x: 466, y: 290 }, { x: 466, y: 160 },
        { x: 466, y: 20 },
      ] }] },
      { segments: [{ label: "lift, then draw the short middle horizontal left-to-right", path: [
        { x: 470, y: 478 }, { x: 550, y: 478 }, { x: 630, y: 478 },
        { x: 710, y: 478 }, { x: 790, y: 478 }, { x: 868, y: 478 },
      ] }] },
      { segments: [{ label: "lift, then draw the long base horizontal left-to-right", path: [
        { x: 65, y: 5 }, { x: 210, y: 5 }, { x: 355, y: 5 },
        { x: 500, y: 5 }, { x: 645, y: 5 }, { x: 790, y: 5 },
        { x: 936, y: 5 },
      ] }] },
    ],
    source: chineseCharacterSource("上"),
  },
  // Hanzi Writer Data draws 一 with a single left-to-right héng. There is nothing
  // to lift between, so the ductus is one stroke of one segment -- the shortest
  // entry in this table, and the reason the lesson can say the stroke count IS
  // the number.
  [ductusKey("chinese", "一")]: {
    script: "chinese",
    glyph: "一",
    strokes: [
      {
        segments: [
          {
            label: "draw the horizontal héng stroke straight across the middle, left to right",
            path: [
              { x: 52, y: 390 },
              { x: 127, y: 390 },
              { x: 202, y: 390 },
              { x: 277, y: 390 },
              { x: 352, y: 390 },
              { x: 427, y: 390 },
              { x: 502, y: 390 },
              { x: 577, y: 390 },
              { x: 652, y: 390 },
              { x: 727, y: 390 },
              { x: 802, y: 390 },
              { x: 877, y: 390 },
              { x: 952, y: 390 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("一"),
  },
  // Two héng strokes, top before bottom, the lower one markedly wider. The source's
  // ordered medians give the same two runs; the widths are read off the vendored
  // Noto Sans SC outline rather than the Arphic-derived source graphics.
  [ductusKey("chinese", "二")]: {
    script: "chinese",
    glyph: "二",
    strokes: [
      {
        segments: [
          {
            label: "draw the upper, shorter horizontal héng stroke from left to right",
            path: [
              { x: 152, y: 656 },
              { x: 210, y: 656 },
              { x: 268, y: 656 },
              { x: 326, y: 656 },
              { x: 384, y: 656 },
              { x: 442, y: 656 },
              { x: 500, y: 656 },
              { x: 558, y: 656 },
              { x: 616, y: 656 },
              { x: 674, y: 656 },
              { x: 732, y: 656 },
              { x: 790, y: 656 },
              { x: 848, y: 656 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the lower, longer horizontal héng stroke from left to right",
            path: [
              { x: 66, y: 62 },
              { x: 139, y: 61 },
              { x: 211, y: 62 },
              { x: 284, y: 61 },
              { x: 356, y: 62 },
              { x: 429, y: 61 },
              { x: 501, y: 62 },
              { x: 574, y: 61 },
              { x: 646, y: 62 },
              { x: 719, y: 61 },
              { x: 791, y: 62 },
              { x: 864, y: 61 },
              { x: 936, y: 62 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("二"),
  },
  // Three héng strokes, ordered top, middle, bottom. The middle is the shortest and
  // the base the widest -- the proportions that stop 三 reading as a tally, and the
  // reason this is the last numeral whose strokes can be counted for its value.
  [ductusKey("chinese", "三")]: {
    script: "chinese",
    glyph: "三",
    strokes: [
      {
        segments: [
          {
            label: "draw the top horizontal héng stroke from left to right",
            path: [
              { x: 172, y: 704 },
              { x: 224, y: 705 },
              { x: 276, y: 704 },
              { x: 329, y: 705 },
              { x: 381, y: 704 },
              { x: 433, y: 705 },
              { x: 485, y: 704 },
              { x: 537, y: 705 },
              { x: 589, y: 704 },
              { x: 642, y: 705 },
              { x: 694, y: 704 },
              { x: 746, y: 705 },
              { x: 798, y: 704 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng stroke, the shortest of the three",
            path: [
              { x: 212, y: 378 },
              { x: 258, y: 378 },
              { x: 303, y: 378 },
              { x: 349, y: 378 },
              { x: 394, y: 378 },
              { x: 440, y: 378 },
              { x: 485, y: 378 },
              { x: 531, y: 378 },
              { x: 576, y: 378 },
              { x: 622, y: 378 },
              { x: 667, y: 378 },
              { x: 713, y: 378 },
              { x: 758, y: 378 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the bottom horizontal héng stroke, the longest of the three",
            path: [
              { x: 74, y: 31 },
              { x: 145, y: 30 },
              { x: 216, y: 31 },
              { x: 287, y: 30 },
              { x: 358, y: 31 },
              { x: 429, y: 30 },
              { x: 500, y: 31 },
              { x: 571, y: 30 },
              { x: 642, y: 31 },
              { x: 713, y: 30 },
              { x: 784, y: 31 },
              { x: 855, y: 30 },
              { x: 926, y: 31 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("三"),
  },
  // Five strokes. Medians 1-2 build the box -- the left wall, then the top and right
  // side in ONE turning héngzhé traced here as two joined segments, which is why the
  // corner counts as one stroke and not two. Medians 3-4 are the two inner pieces,
  // and median 5 closes the bottom last.
  [ductusKey("chinese", "四")]: {
    script: "chinese",
    glyph: "四",
    strokes: [
      {
        segments: [
          {
            label: "draw the left vertical shù stroke from top to bottom",
            path: [
              { x: 135, y: 690 },
              { x: 126, y: 629 },
              { x: 125, y: 568 },
              { x: 126, y: 508 },
              { x: 125, y: 447 },
              { x: 126, y: 386 },
              { x: 125, y: 325 },
              { x: 126, y: 264 },
              { x: 125, y: 203 },
              { x: 126, y: 143 },
              { x: 134, y: 82 },
              { x: 125, y: 21 },
              { x: 126, y: -40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the horizontal-turning héngzhé stroke across the top",
            path: [
              { x: 100, y: 706 },
              { x: 164, y: 717 },
              { x: 229, y: 716 },
              { x: 293, y: 717 },
              { x: 357, y: 707 },
              { x: 421, y: 702 },
              { x: 486, y: 717 },
              { x: 550, y: 716 },
              { x: 614, y: 707 },
              { x: 678, y: 716 },
              { x: 743, y: 717 },
              { x: 807, y: 716 },
              { x: 871, y: 707 },
            ],
          },
          {
            label: "and down the right side without lifting",
            path: [
              { x: 871, y: 707 },
              { x: 870, y: 630 },
              { x: 870, y: 570 },
              { x: 870, y: 510 },
              { x: 870, y: 450 },
              { x: 870, y: 390 },
              { x: 870, y: 330 },
              { x: 870, y: 270 },
              { x: 870, y: 210 },
              { x: 870, y: 150 },
              { x: 862, y: 90 },
              { x: 858, y: 30 },
              { x: 870, y: -30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the short inner left-falling piě stroke",
            path: [
              { x: 388, y: 670 },
              { x: 386, y: 630 },
              { x: 385, y: 591 },
              { x: 383, y: 551 },
              { x: 379, y: 512 },
              { x: 375, y: 472 },
              { x: 368, y: 433 },
              { x: 358, y: 393 },
              { x: 344, y: 353 },
              { x: 328, y: 314 },
              { x: 308, y: 274 },
              { x: 281, y: 235 },
              { x: 241, y: 195 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the inner stroke down",
            path: [
              { x: 600, y: 670 },
              { x: 600, y: 643 },
              { x: 600, y: 615 },
              { x: 600, y: 588 },
              { x: 600, y: 560 },
              { x: 600, y: 533 },
              { x: 600, y: 505 },
              { x: 600, y: 478 },
              { x: 600, y: 450 },
              { x: 600, y: 423 },
              { x: 600, y: 395 },
              { x: 600, y: 368 },
              { x: 602, y: 340 },
            ],
          },
          {
            label: "and turning up to the right at its foot",
            path: [
              { x: 602, y: 340 },
              { x: 618, y: 312 },
              { x: 635, y: 312 },
              { x: 652, y: 290 },
              { x: 669, y: 289 },
              { x: 686, y: 288 },
              { x: 703, y: 289 },
              { x: 720, y: 288 },
              { x: 737, y: 289 },
              { x: 754, y: 288 },
              { x: 771, y: 289 },
              { x: 788, y: 290 },
              { x: 805, y: 292 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then close the bottom with a horizontal héng stroke from left to right",
            path: [
              { x: 175, y: 65 },
              { x: 229, y: 65 },
              { x: 283, y: 65 },
              { x: 337, y: 65 },
              { x: 391, y: 65 },
              { x: 445, y: 65 },
              { x: 499, y: 65 },
              { x: 553, y: 65 },
              { x: 607, y: 65 },
              { x: 661, y: 65 },
              { x: 715, y: 65 },
              { x: 769, y: 65 },
              { x: 823, y: 65 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("四"),
  },
  // Four strokes for the number five. The top bar, then a shù descending from it and
  // leaning left, then a héngzhé that crosses right and turns down, then the widest
  // stroke in the character closing it along the bottom. Where the descender crosses
  // the middle bar the traced band is clamped to one stroke's width, or its centre
  // would land between the two.
  [ductusKey("chinese", "五")]: {
    script: "chinese",
    glyph: "五",
    strokes: [
      {
        segments: [
          {
            label: "draw the top horizontal héng stroke from left to right",
            path: [
              { x: 130, y: 706 },
              { x: 191, y: 706 },
              { x: 253, y: 706 },
              { x: 314, y: 706 },
              { x: 375, y: 706 },
              { x: 436, y: 697 },
              { x: 498, y: 705 },
              { x: 559, y: 705 },
              { x: 620, y: 705 },
              { x: 681, y: 705 },
              { x: 743, y: 705 },
              { x: 804, y: 705 },
              { x: 865, y: 705 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the shù stroke descending from the top bar, leaning left",
            path: [
              { x: 446, y: 665 },
              { x: 440, y: 615 },
              { x: 432, y: 564 },
              { x: 425, y: 514 },
              { x: 416, y: 463 },
              { x: 416, y: 413 },
              { x: 400, y: 363 },
              { x: 391, y: 312 },
              { x: 382, y: 262 },
              { x: 373, y: 211 },
              { x: 365, y: 161 },
              { x: 355, y: 110 },
              { x: 345, y: 60 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the horizontal-turning héngzhé stroke across to the right",
            path: [
              { x: 190, y: 414 },
              { x: 235, y: 414 },
              { x: 281, y: 414 },
              { x: 326, y: 414 },
              { x: 372, y: 405 },
              { x: 417, y: 405 },
              { x: 463, y: 414 },
              { x: 508, y: 414 },
              { x: 553, y: 414 },
              { x: 599, y: 414 },
              { x: 644, y: 414 },
              { x: 690, y: 414 },
              { x: 735, y: 406 },
            ],
          },
          {
            label: "and then down",
            path: [
              { x: 735, y: 406 },
              { x: 724, y: 390 },
              { x: 733, y: 360 },
              { x: 730, y: 330 },
              { x: 727, y: 300 },
              { x: 725, y: 270 },
              { x: 723, y: 240 },
              { x: 720, y: 210 },
              { x: 716, y: 180 },
              { x: 714, y: 150 },
              { x: 710, y: 120 },
              { x: 708, y: 90 },
              { x: 704, y: 60 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the long bottom horizontal héng stroke, the widest in the character",
            path: [
              { x: 62, y: 12 },
              { x: 135, y: 12 },
              { x: 208, y: 12 },
              { x: 281, y: 12 },
              { x: 354, y: 21 },
              { x: 427, y: 11 },
              { x: 500, y: 11 },
              { x: 573, y: 11 },
              { x: 646, y: 11 },
              { x: 719, y: 20 },
              { x: 792, y: 12 },
              { x: 865, y: 12 },
              { x: 938, y: 12 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("五"),
  },
  // The four-frame Commons sequence writes the complete left body in one
  // continuous run, lifts for the middle shoulder, descends the right stem,
  // then closes with the short shirorekha: four strokes and three lifts.
  [ductusKey("devanagari", "अ")]: {
    script: "devanagari",
    glyph: "अ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 570, y: 585 }, { x: 615, y: 585 },
        { x: 660, y: 585 }, { x: 705, y: 585 }, { x: 750, y: 585 },
        { x: 775, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("अ"),
  },
  // The five Commons buildup frames preserve the joined left body of अ, then
  // add the shoulder, inner stem, trailing stem, and headline as four lifted
  // runs: five strokes and four lifts in all.
  [ductusKey("devanagari", "आ")]: {
    script: "devanagari",
    glyph: "आ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the inner stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then descend the trailing stem", path: [
        { x: 893, y: 590 }, { x: 893, y: 500 }, { x: 893, y: 410 },
        { x: 893, y: 320 }, { x: 893, y: 230 }, { x: 893, y: 140 },
        { x: 893, y: 50 }, { x: 893, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 610, y: 585 }, { x: 695, y: 585 },
        { x: 780, y: 585 }, { x: 865, y: 585 }, { x: 950, y: 585 },
        { x: 1030, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("आ"),
  },
  // The Commons diagram writes the upright, both bowls, and tail as one
  // continuous body, then lifts once to draw the headline left-to-right.
  [ductusKey("devanagari", "इ")]: {
    script: "devanagari",
    glyph: "इ",
    strokes: [
      {
        segments: [
          { label: "descend the upright from the headline", path: [
            { x: 363, y: 590 }, { x: 363, y: 540 }, { x: 363, y: 490 },
            { x: 363, y: 440 },
          ] },
          { label: "turn left and curve around the upper bowl without lifting", path: [
            { x: 363, y: 440 }, { x: 320, y: 430 }, { x: 270, y: 430 },
            { x: 210, y: 430 }, { x: 160, y: 425 }, { x: 120, y: 405 },
            { x: 90, y: 375 }, { x: 85, y: 340 }, { x: 100, y: 305 },
            { x: 130, y: 275 }, { x: 170, y: 250 },
          ] },
          { label: "sweep right through the waist and around the lower bowl", path: [
            { x: 170, y: 250 }, { x: 220, y: 260 }, { x: 275, y: 260 },
            { x: 325, y: 245 }, { x: 370, y: 220 }, { x: 400, y: 185 },
            { x: 405, y: 150 }, { x: 390, y: 115 }, { x: 360, y: 85 },
            { x: 320, y: 60 }, { x: 275, y: 42 }, { x: 225, y: 35 },
            { x: 180, y: 40 }, { x: 140, y: 55 }, { x: 105, y: 75 },
            { x: 80, y: 85 },
          ] },
          { label: "finish down-right through the tail without lifting", path: [
            { x: 80, y: 85 }, { x: 120, y: 80 }, { x: 160, y: 60 },
            { x: 200, y: 35 }, { x: 230, y: 0 }, { x: 260, y: -35 },
            { x: 290, y: -75 }, { x: 320, y: -110 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 500, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("इ"),
  },
  // The three Commons panels reuse इ's continuous body, then add the upper
  // curl and headline as two separately placed runs.
  [ductusKey("devanagari", "ई")]: {
    script: "devanagari",
    glyph: "ई",
    strokes: [
      {
        segments: [
          { label: "descend the upright from the headline", path: [
            { x: 363, y: 590 }, { x: 363, y: 540 }, { x: 363, y: 490 },
            { x: 363, y: 440 },
          ] },
          { label: "turn left and curve around the upper bowl without lifting", path: [
            { x: 363, y: 440 }, { x: 320, y: 430 }, { x: 270, y: 430 },
            { x: 210, y: 430 }, { x: 160, y: 425 }, { x: 120, y: 405 },
            { x: 90, y: 375 }, { x: 85, y: 340 }, { x: 100, y: 305 },
            { x: 130, y: 275 }, { x: 170, y: 250 },
          ] },
          { label: "sweep right through the waist and around the lower bowl", path: [
            { x: 170, y: 250 }, { x: 220, y: 260 }, { x: 275, y: 260 },
            { x: 325, y: 245 }, { x: 370, y: 220 }, { x: 400, y: 185 },
            { x: 405, y: 150 }, { x: 390, y: 115 }, { x: 360, y: 85 },
            { x: 320, y: 60 }, { x: 275, y: 42 }, { x: 225, y: 35 },
            { x: 180, y: 40 }, { x: 140, y: 55 }, { x: 105, y: 75 },
            { x: 80, y: 85 },
          ] },
          { label: "finish down-right through the tail without lifting", path: [
            { x: 80, y: 85 }, { x: 120, y: 80 }, { x: 160, y: 60 },
            { x: 200, y: 35 }, { x: 230, y: 0 }, { x: 260, y: -35 },
            { x: 290, y: -75 }, { x: 320, y: -110 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the upper curl upward and around to the right", path: [
        { x: 352, y: 620 }, { x: 330, y: 660 }, { x: 310, y: 710 },
        { x: 290, y: 760 }, { x: 300, y: 810 }, { x: 330, y: 850 },
        { x: 370, y: 865 }, { x: 410, y: 860 }, { x: 450, y: 850 },
        { x: 480, y: 835 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 500, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ई"),
  },
  // The two Commons panels keep the upper bowl and lower loop in one
  // continuous body, then lift once to draw the headline left-to-right.
  [ductusKey("devanagari", "उ")]: {
    script: "devanagari",
    glyph: "उ",
    strokes: [
      {
        segments: [
          { label: "curve down and left around the upper bowl", path: [
            { x: 350, y: 555 }, { x: 390, y: 530 }, { x: 420, y: 500 },
            { x: 435, y: 460 }, { x: 420, y: 420 }, { x: 380, y: 380 },
            { x: 330, y: 345 }, { x: 275, y: 325 }, { x: 235, y: 320 },
          ] },
          { label: "sweep back through the waist and around the lower loop without lifting", path: [
            { x: 235, y: 320 }, { x: 280, y: 325 }, { x: 330, y: 335 },
            { x: 380, y: 325 }, { x: 420, y: 290 }, { x: 450, y: 240 },
            { x: 465, y: 180 }, { x: 450, y: 125 }, { x: 420, y: 85 },
            { x: 375, y: 55 }, { x: 325, y: 40 }, { x: 275, y: 45 },
            { x: 225, y: 65 }, { x: 180, y: 95 }, { x: 140, y: 135 },
            { x: 110, y: 180 }, { x: 85, y: 230 }, { x: 70, y: 280 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 558, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("उ"),
  },
  // The three Commons panels reuse उ's continuous body, then add the
  // right-hand loop and headline as two separately placed runs.
  [ductusKey("devanagari", "ऊ")]: {
    script: "devanagari",
    glyph: "ऊ",
    strokes: [
      {
        segments: [
          { label: "curve down and left around the upper bowl", path: [
            { x: 350, y: 555 }, { x: 390, y: 530 }, { x: 420, y: 500 },
            { x: 435, y: 460 }, { x: 420, y: 420 }, { x: 380, y: 380 },
            { x: 330, y: 345 }, { x: 275, y: 325 }, { x: 235, y: 320 },
          ] },
          { label: "sweep back through the waist and around the lower loop without lifting", path: [
            { x: 235, y: 320 }, { x: 280, y: 325 }, { x: 330, y: 335 },
            { x: 380, y: 325 }, { x: 420, y: 290 }, { x: 450, y: 240 },
            { x: 465, y: 180 }, { x: 450, y: 125 }, { x: 420, y: 85 },
            { x: 375, y: 55 }, { x: 325, y: 40 }, { x: 275, y: 45 },
            { x: 225, y: 65 }, { x: 180, y: 95 }, { x: 140, y: 135 },
            { x: 110, y: 180 }, { x: 85, y: 230 }, { x: 70, y: 280 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the right-hand loop up, around, and down-left", path: [
        { x: 455, y: 250 }, { x: 490, y: 280 }, { x: 535, y: 305 },
        { x: 585, y: 310 }, { x: 630, y: 295 }, { x: 670, y: 270 },
        { x: 700, y: 235 }, { x: 715, y: 195 }, { x: 715, y: 150 },
        { x: 705, y: 105 }, { x: 685, y: 65 }, { x: 660, y: 25 },
        { x: 635, y: -5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 135, y: 585 }, { x: 265, y: 585 },
        { x: 395, y: 585 }, { x: 525, y: 585 }, { x: 655, y: 585 },
        { x: 795, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ऊ"),
  },
  // The three Commons panels draw the long left stem and tail continuously,
  // then place the shorter hooked stem and headline as separate runs.
  [ductusKey("devanagari", "ए")]: {
    script: "devanagari",
    glyph: "ए",
    strokes: [
      {
        segments: [
          { label: "descend the long left stem from the headline", path: [
            { x: 120, y: 585 }, { x: 120, y: 530 }, { x: 120, y: 470 },
            { x: 120, y: 410 }, { x: 120, y: 350 }, { x: 125, y: 290 },
          ] },
          { label: "curve right through the lower shoulder and sweep down the tail without lifting", path: [
            { x: 125, y: 290 }, { x: 145, y: 245 }, { x: 185, y: 205 },
            { x: 235, y: 175 }, { x: 285, y: 145 }, { x: 335, y: 115 },
            { x: 380, y: 85 }, { x: 415, y: 50 }, { x: 435, y: 10 },
            { x: 435, y: -30 }, { x: 420, y: -70 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then descend the shorter right stem into its inward hook", path: [
        { x: 435, y: 585 }, { x: 435, y: 530 }, { x: 435, y: 470 },
        { x: 435, y: 410 }, { x: 430, y: 350 }, { x: 410, y: 300 },
        { x: 380, y: 260 }, { x: 350, y: 235 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 563, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ए"),
  },
  // The four Commons panels reuse ए's long body and shorter hooked stem,
  // then add the upper arc and headline as two separately placed runs.
  [ductusKey("devanagari", "ऐ")]: {
    script: "devanagari",
    glyph: "ऐ",
    strokes: [
      {
        segments: [
          { label: "descend the long left stem from the headline", path: [
            { x: 120, y: 585 }, { x: 120, y: 530 }, { x: 120, y: 470 },
            { x: 120, y: 410 }, { x: 120, y: 350 }, { x: 125, y: 290 },
          ] },
          { label: "curve right through the lower shoulder and sweep down the tail without lifting", path: [
            { x: 125, y: 290 }, { x: 145, y: 245 }, { x: 185, y: 205 },
            { x: 235, y: 175 }, { x: 285, y: 145 }, { x: 335, y: 115 },
            { x: 380, y: 85 }, { x: 415, y: 50 }, { x: 435, y: 10 },
            { x: 435, y: -30 }, { x: 420, y: -70 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then descend the shorter right stem into its inward hook", path: [
        { x: 435, y: 585 }, { x: 435, y: 530 }, { x: 435, y: 470 },
        { x: 435, y: 410 }, { x: 430, y: 350 }, { x: 410, y: 300 },
        { x: 380, y: 260 }, { x: 350, y: 235 },
      ] }] },
      { segments: [{ label: "lift, then sweep the upper arc upward and left", path: [
        { x: 430, y: 620 }, { x: 415, y: 680 }, { x: 390, y: 745 },
        { x: 360, y: 800 }, { x: 325, y: 840 }, { x: 285, y: 860 },
        { x: 245, y: 865 }, { x: 205, y: 855 }, { x: 170, y: 835 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 563, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ऐ"),
  },
  // The six Commons panels reuse आ's joined left body, separate shoulder,
  // inner stem, and trailing stem, then add the upper arc and headline as two
  // separately placed runs: six strokes and five lifts in all.
  [ductusKey("devanagari", "ओ")]: {
    script: "devanagari",
    glyph: "ओ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the inner stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then descend the trailing stem", path: [
        { x: 893, y: 590 }, { x: 893, y: 500 }, { x: 893, y: 410 },
        { x: 893, y: 320 }, { x: 893, y: 230 }, { x: 893, y: 140 },
        { x: 893, y: 50 }, { x: 893, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep the upper arc upward and left", path: [
        { x: 890, y: 620 }, { x: 880, y: 680 }, { x: 860, y: 735 },
        { x: 835, y: 785 }, { x: 805, y: 825 }, { x: 770, y: 850 },
        { x: 730, y: 862 }, { x: 690, y: 860 }, { x: 655, y: 850 },
        { x: 625, y: 840 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 610, y: 585 }, { x: 695, y: 585 },
        { x: 780, y: 585 }, { x: 865, y: 585 }, { x: 950, y: 585 },
        { x: 1030, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ओ"),
  },
  // The seven Commons panels reuse आ's four base runs, then separately sweep
  // the lower and taller upper arcs upward and left before the final headline:
  // seven strokes and six lifts in all.
  [ductusKey("devanagari", "औ")]: {
    script: "devanagari",
    glyph: "औ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the inner stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then descend the trailing stem", path: [
        { x: 893, y: 590 }, { x: 893, y: 500 }, { x: 893, y: 410 },
        { x: 893, y: 320 }, { x: 893, y: 230 }, { x: 893, y: 140 },
        { x: 893, y: 50 }, { x: 893, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep the lower upper arc upward and left", path: [
        { x: 890, y: 620 }, { x: 875, y: 650 }, { x: 850, y: 680 },
        { x: 820, y: 705 }, { x: 785, y: 730 }, { x: 745, y: 745 },
        { x: 705, y: 750 }, { x: 670, y: 745 }, { x: 640, y: 735 },
        { x: 620, y: 725 },
      ] }] },
      { segments: [{ label: "lift, then sweep the taller upper arc upward and left", path: [
        { x: 890, y: 620 }, { x: 880, y: 680 }, { x: 860, y: 735 },
        { x: 835, y: 785 }, { x: 805, y: 825 }, { x: 770, y: 850 },
        { x: 730, y: 862 }, { x: 690, y: 860 }, { x: 655, y: 850 },
        { x: 625, y: 840 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 610, y: 585 }, { x: 695, y: 585 },
        { x: 780, y: 585 }, { x: 865, y: 585 }, { x: 950, y: 585 },
        { x: 1030, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("औ"),
  },
  // Opiaterein's animation writes the left bowl counterclockwise, then places
  // the central stem, right-hand arch, and headline as three separate runs.
  // The Central Hindi Directorate's 2019 deskbook independently shows the
  // same four-part buildup: four strokes and three lifts in all.
  [ductusKey("devanagari", "क")]: {
    script: "devanagari",
    glyph: "क",
    strokes: [
      { segments: [{ label: "sweep left over the top and around the bowl", path: [
        { x: 355, y: 430 }, { x: 315, y: 430 },
        { x: 270, y: 435 }, { x: 225, y: 430 }, { x: 180, y: 415 },
        { x: 140, y: 390 }, { x: 110, y: 360 }, { x: 90, y: 325 },
        { x: 86, y: 290 }, { x: 95, y: 250 }, { x: 115, y: 215 },
        { x: 145, y: 185 }, { x: 180, y: 165 }, { x: 220, y: 154 },
        { x: 260, y: 155 }, { x: 300, y: 170 }, { x: 335, y: 195 },
        { x: 360, y: 225 }, { x: 377, y: 260 }, { x: 387, y: 290 },
      ] }] },
      { segments: [{ label: "lift, then descend the central stem", path: [
        { x: 417, y: 551 }, { x: 417, y: 480 }, { x: 417, y: 400 },
        { x: 417, y: 320 }, { x: 417, y: 240 }, { x: 417, y: 160 },
        { x: 417, y: 80 }, { x: 417, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep the right-hand arch clockwise", path: [
        { x: 455, y: 350 }, { x: 490, y: 365 }, { x: 530, y: 370 },
        { x: 570, y: 365 }, { x: 610, y: 345 }, { x: 645, y: 320 },
        { x: 670, y: 285 }, { x: 685, y: 245 }, { x: 680, y: 205 },
        { x: 660, y: 165 }, { x: 640, y: 125 }, { x: 620, y: 95 },
        { x: 600, y: 70 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 115, y: 585 }, { x: 225, y: 585 },
        { x: 335, y: 585 }, { x: 445, y: 585 }, { x: 555, y: 585 },
        { x: 665, y: 585 }, { x: 778, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("क"),
  },
  // Opiaterein's animation writes the counterclockwise loop and carries the
  // same run up its joined stem, then separately descends the right stem and
  // finishes the headline. The Central Hindi Directorate's 2019 deskbook
  // independently shows the same three-part buildup: three strokes, two lifts.
  [ductusKey("devanagari", "ग")]: {
    script: "devanagari",
    glyph: "ग",
    strokes: [
      { segments: [{ label: "sweep counterclockwise around the loop and up the joined stem", path: [
        { x: 168, y: 315 }, { x: 140, y: 322 }, { x: 110, y: 312 },
        { x: 85, y: 290 }, { x: 76, y: 262 }, { x: 86, y: 232 },
        { x: 112, y: 208 }, { x: 142, y: 198 }, { x: 168, y: 205 },
        { x: 168, y: 250 }, { x: 168, y: 320 }, { x: 168, y: 400 },
        { x: 168, y: 475 }, { x: 168, y: 550 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 434, y: 551 }, { x: 434, y: 475 }, { x: 434, y: 395 },
        { x: 434, y: 315 }, { x: 434, y: 235 }, { x: 434, y: 155 },
        { x: 434, y: 75 }, { x: 434, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 572, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ग"),
  },
  // Opiaterein's animation joins the short upper bar directly to the rounded
  // body, then separately descends the right stem and finishes the headline.
  // The Central Hindi Directorate deskbook corroborates component order while
  // staging the bar and body separately: three animated strokes, two lifts.
  [ductusKey("devanagari", "च")]: {
    script: "devanagari",
    glyph: "च",
    strokes: [
      { segments: [{ label: "draw the upper bar right and curve around the open body", path: [
        { x: 45, y: 412 }, { x: 100, y: 412 }, { x: 160, y: 412 },
        { x: 220, y: 412 }, { x: 280, y: 412 }, { x: 340, y: 412 },
        { x: 320, y: 395 }, { x: 300, y: 380 }, { x: 270, y: 372 },
        { x: 235, y: 365 }, { x: 215, y: 350 }, { x: 200, y: 330 },
        { x: 187, y: 305 }, { x: 178, y: 275 }, { x: 177, y: 250 },
        { x: 180, y: 218 },
        { x: 200, y: 185 }, { x: 235, y: 160 }, { x: 280, y: 145 },
        { x: 325, y: 145 }, { x: 370, y: 160 }, { x: 410, y: 182 },
        { x: 447, y: 210 }, { x: 470, y: 238 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 505, y: 551 }, { x: 505, y: 475 }, { x: 505, y: 395 },
        { x: 505, y: 315 }, { x: 505, y: 235 }, { x: 505, y: 155 },
        { x: 505, y: 75 }, { x: 505, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 545, y: 585 }, { x: 644, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("च"),
  },
  // Opiaterein's animation sweeps the shoulder right-to-left and carries the
  // same run down around the open body, then separately descends the right stem
  // and finishes the headline. The Central Hindi Directorate deskbook shows
  // the same three-part buildup: three strokes, two lifts.
  [ductusKey("devanagari", "त")]: {
    script: "devanagari",
    glyph: "त",
    strokes: [
      { segments: [{ label: "sweep left across the shoulder and curve down to the open tip", path: [
        { x: 400, y: 364 }, { x: 350, y: 364 }, { x: 300, y: 364 },
        { x: 247, y: 364 }, { x: 205, y: 363 }, { x: 165, y: 345 },
        { x: 130, y: 315 }, { x: 105, y: 280 }, { x: 86, y: 242 },
        { x: 88, y: 205 }, { x: 103, y: 165 }, { x: 125, y: 125 },
        { x: 152, y: 88 }, { x: 184, y: 52 }, { x: 219, y: 14 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 440, y: 551 }, { x: 440, y: 475 }, { x: 440, y: 395 },
        { x: 440, y: 315 }, { x: 440, y: 235 }, { x: 440, y: 155 },
        { x: 440, y: 75 }, { x: 440, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 579, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("त"),
  },
  // Opiaterein's animation descends the short stem, then joins the outer body
  // directly through the inward curl and tail before the final headline. The
  // Central Hindi Directorate deskbook corroborates component order while
  // staging the body and curl-tail separately: three animated strokes, two lifts.
  [ductusKey("devanagari", "द")]: {
    script: "devanagari",
    glyph: "द",
    strokes: [
      { segments: [{ label: "descend the short stem", path: [
        { x: 395, y: 551 }, { x: 395, y: 505 }, { x: 395, y: 460 },
        { x: 395, y: 420 },
      ] }] },
      { segments: [{ label: "lift, then sweep around the body, inner curl, and tail", path: [
        { x: 395, y: 420 }, { x: 350, y: 420 }, { x: 300, y: 420 },
        { x: 245, y: 418 }, { x: 190, y: 400 }, { x: 145, y: 370 },
        { x: 110, y: 335 }, { x: 90, y: 295 }, { x: 90, y: 255 },
        { x: 95, y: 210 }, { x: 125, y: 170 }, { x: 170, y: 140 },
        { x: 215, y: 115 }, { x: 260, y: 110 }, { x: 300, y: 112 },
        { x: 340, y: 118 }, { x: 385, y: 130 }, { x: 420, y: 155 },
        { x: 440, y: 185 }, { x: 435, y: 215 }, { x: 410, y: 235 },
        { x: 380, y: 235 }, { x: 355, y: 220 }, { x: 348, y: 195 },
        { x: 355, y: 170 }, { x: 375, y: 150 }, { x: 400, y: 128 },
        { x: 415, y: 98 }, { x: 435, y: 55 }, { x: 458, y: 10 },
        { x: 482, y: -38 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 80, y: 585 }, { x: 155, y: 585 },
        { x: 230, y: 585 }, { x: 305, y: 585 }, { x: 380, y: 585 },
        { x: 455, y: 585 }, { x: 536, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("द"),
  },
  // Opiaterein's animation writes the upper spiral and shoulder, lower bowl,
  // right stem, and headline as four separate runs. The Central Hindi
  // Directorate deskbook independently shows the same buildup: three lifts.
  [ductusKey("devanagari", "ध")]: {
    script: "devanagari",
    glyph: "ध",
    strokes: [
      { segments: [{ label: "curl around the upper spiral and sweep right through the shoulder", path: [
        { x: 285, y: 450 }, { x: 300, y: 475 }, { x: 305, y: 505 },
        { x: 300, y: 535 }, { x: 285, y: 560 }, { x: 260, y: 585 },
        { x: 225, y: 600 }, { x: 185, y: 605 }, { x: 145, y: 590 },
        { x: 110, y: 565 }, { x: 85, y: 530 }, { x: 75, y: 490 },
        { x: 80, y: 450 }, { x: 95, y: 420 }, { x: 115, y: 395 },
        { x: 145, y: 375 }, { x: 175, y: 355 }, { x: 210, y: 340 },
        { x: 250, y: 335 }, { x: 290, y: 340 }, { x: 325, y: 350 },
      ] }] },
      { segments: [{ label: "lift, then sweep down and around the lower bowl", path: [
        { x: 170, y: 330 }, { x: 140, y: 320 }, { x: 125, y: 295 },
        { x: 125, y: 265 }, { x: 130, y: 230 }, { x: 140, y: 195 },
        { x: 155, y: 165 }, { x: 160, y: 140 }, { x: 205, y: 120 },
        { x: 250, y: 112 }, { x: 300, y: 112 }, { x: 350, y: 122 },
        { x: 395, y: 145 }, { x: 435, y: 180 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 485, y: 551 }, { x: 485, y: 475 }, { x: 485, y: 395 },
        { x: 485, y: 315 }, { x: 485, y: 235 }, { x: 485, y: 155 },
        { x: 485, y: 75 }, { x: 485, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 388, y: 585 }, { x: 430, y: 585 }, { x: 475, y: 585 },
        { x: 520, y: 585 }, { x: 570, y: 585 }, { x: 625, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ध"),
  },
  // Opiaterein's animation writes the clockwise loop and rightward shoulder,
  // right stem, and headline as three separate runs. The Central Hindi
  // Directorate deskbook independently shows the same buildup: two lifts.
  [ductusKey("devanagari", "न")]: {
    script: "devanagari",
    glyph: "न",
    strokes: [
      { segments: [{ label: "circle clockwise around the left loop and sweep right", path: [
        { x: 185, y: 255 }, { x: 178, y: 225 }, { x: 158, y: 205 },
        { x: 130, y: 202 }, { x: 100, y: 215 }, { x: 72, y: 242 },
        { x: 52, y: 275 }, { x: 48, y: 310 }, { x: 58, y: 338 },
        { x: 82, y: 350 }, { x: 115, y: 350 }, { x: 155, y: 335 },
        { x: 205, y: 335 }, { x: 260, y: 335 }, { x: 320, y: 335 },
        { x: 380, y: 335 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 425, y: 551 }, { x: 425, y: 475 }, { x: 425, y: 395 },
        { x: 425, y: 315 }, { x: 425, y: 235 }, { x: 425, y: 155 },
        { x: 425, y: 75 }, { x: 425, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 80, y: 585 }, { x: 155, y: 585 },
        { x: 230, y: 585 }, { x: 305, y: 585 }, { x: 380, y: 585 },
        { x: 455, y: 585 }, { x: 565, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("न"),
  },
  // Opiaterein's animation descends the left stem and curves right around the
  // lower bowl, then separately descends the right stem and finishes the
  // headline. The Central Hindi Directorate deskbook independently shows the
  // same three-part buildup and directions: three strokes, two lifts.
  [ductusKey("devanagari", "प")]: {
    script: "devanagari",
    glyph: "प",
    strokes: [
      { segments: [{ label: "descend the left stem and curve right around the lower bowl", path: [
        { x: 120, y: 551 }, { x: 120, y: 480 }, { x: 120, y: 410 },
        { x: 120, y: 355 }, { x: 128, y: 310 }, { x: 145, y: 270 },
        { x: 170, y: 238 }, { x: 205, y: 215 }, { x: 245, y: 202 },
        { x: 285, y: 202 }, { x: 325, y: 215 }, { x: 360, y: 238 },
        { x: 385, y: 265 }, { x: 402, y: 295 }, { x: 408, y: 320 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 438, y: 551 }, { x: 438, y: 475 }, { x: 438, y: 395 },
        { x: 438, y: 315 }, { x: 438, y: 235 }, { x: 438, y: 155 },
        { x: 438, y: 75 }, { x: 438, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 578, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("प"),
  },
  // JackPotte's animation circles counterclockwise around the oval, then
  // separately descends the right stem, crosses down-right through the body,
  // and finishes the headline. The Central Hindi Directorate deskbook shows
  // the same four-part buildup and directions: four strokes, three lifts.
  [ductusKey("devanagari", "ब")]: {
    script: "devanagari",
    glyph: "ब",
    strokes: [
      { segments: [{ label: "circle counterclockwise around the oval body", path: [
        { x: 350, y: 390 }, { x: 320, y: 415 }, { x: 275, y: 432 }, { x: 225, y: 432 },
        { x: 175, y: 415 }, { x: 135, y: 385 }, { x: 105, y: 345 },
        { x: 88, y: 300 }, { x: 88, y: 255 }, { x: 105, y: 215 },
        { x: 135, y: 182 }, { x: 175, y: 158 }, { x: 225, y: 147 },
        { x: 275, y: 150 }, { x: 320, y: 168 }, { x: 350, y: 198 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 442, y: 551 }, { x: 442, y: 475 }, { x: 442, y: 395 },
        { x: 442, y: 315 }, { x: 442, y: 235 }, { x: 442, y: 155 },
        { x: 442, y: 75 }, { x: 442, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then cross the body down and right", path: [
        { x: 175, y: 405 }, { x: 205, y: 365 }, { x: 235, y: 325 },
        { x: 265, y: 285 }, { x: 295, y: 245 }, { x: 325, y: 205 },
        { x: 354, y: 176 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 580, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ब"),
  },
  // JackPotte's animation keeps the clockwise upper loop, descending trunk,
  // clockwise lower bowl, and rightward crossbar in one continuous run, then
  // separately descends the right stem and finishes the headline. The Central
  // Hindi Directorate deskbook confirms the component order but stages the two
  // body parts separately: three animation-backed strokes, two lifts.
  [ductusKey("devanagari", "भ")]: {
    script: "devanagari",
    glyph: "भ",
    strokes: [
      { segments: [{ label: "circle clockwise through both loops and sweep right", path: [
        { x: 200, y: 410 }, { x: 165, y: 414 }, { x: 135, y: 425 },
        { x: 95, y: 455 }, { x: 75, y: 495 },
        { x: 78, y: 540 }, { x: 100, y: 575 }, { x: 135, y: 595 },
        { x: 180, y: 602 }, { x: 225, y: 592 }, { x: 260, y: 565 },
        { x: 285, y: 528 }, { x: 292, y: 485 }, { x: 285, y: 450 },
        { x: 292, y: 405 }, { x: 292, y: 360 }, { x: 292, y: 315 },
        { x: 292, y: 265 }, { x: 286, y: 220 }, { x: 260, y: 188 },
        { x: 225, y: 184 }, { x: 205, y: 215 }, { x: 180, y: 242 },
        { x: 182, y: 278 }, { x: 200, y: 300 }, { x: 235, y: 285 },
        { x: 275, y: 285 }, { x: 325, y: 285 }, { x: 380, y: 285 },
        { x: 440, y: 285 }, { x: 495, y: 285 }, { x: 530, y: 285 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 575, y: 551 }, { x: 575, y: 475 }, { x: 575, y: 395 },
        { x: 575, y: 315 }, { x: 575, y: 235 }, { x: 575, y: 155 },
        { x: 575, y: 75 }, { x: 575, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 405, y: 585 }, { x: 455, y: 585 }, { x: 510, y: 585 },
        { x: 565, y: 585 }, { x: 620, y: 585 }, { x: 675, y: 585 },
        { x: 715, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("भ"),
  },
  // JackPotte's animation joins the descending left stem directly to the
  // clockwise lower loop and rightward crossbar, then separately descends the
  // right stem and finishes the headline. The Central Hindi Directorate
  // deskbook confirms the component order but stages the left stem and lower
  // body separately: three animation-backed strokes, two lifts.
  [ductusKey("devanagari", "म")]: {
    script: "devanagari",
    glyph: "म",
    strokes: [
      { segments: [{ label: "descend the left stem, circle clockwise through the loop, and sweep right", path: [
        { x: 166, y: 551 }, { x: 166, y: 475 }, { x: 166, y: 405 },
        { x: 166, y: 350 }, { x: 167, y: 315 }, { x: 167, y: 265 },
        { x: 161, y: 220 }, { x: 135, y: 188 }, { x: 100, y: 184 },
        { x: 80, y: 215 }, { x: 55, y: 242 }, { x: 57, y: 278 },
        { x: 75, y: 300 }, { x: 110, y: 285 }, { x: 150, y: 285 },
        { x: 200, y: 285 }, { x: 255, y: 285 }, { x: 315, y: 285 },
        { x: 370, y: 285 }, { x: 405, y: 285 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 468, y: 551 }, { x: 468, y: 475 }, { x: 468, y: 395 },
        { x: 468, y: 315 }, { x: 468, y: 235 }, { x: 468, y: 155 },
        { x: 468, y: 75 }, { x: 468, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 610, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("म"),
  },
  // Opiaterein's animation and the Central Hindi Directorate deskbook agree
  // on four runs: the clockwise inner curl, the restarted lower bowl, the
  // descending right stem, and the headline. JackPotte documents a joined
  // two-lift body as a real variation; this path follows the corroborated
  // four-stroke form.
  [ductusKey("devanagari", "य")]: {
    script: "devanagari",
    glyph: "य",
    strokes: [
      { segments: [{ label: "curve clockwise around the inner curl", path: [
        { x: 165, y: 551 }, { x: 195, y: 540 }, { x: 215, y: 510 },
        { x: 220, y: 475 }, { x: 207, y: 440 }, { x: 185, y: 410 },
        { x: 150, y: 382 }, { x: 105, y: 360 }, { x: 55, y: 355 },
      ] }] },
      { segments: [{ label: "lift, then curve around the lower bowl to the right", path: [
        { x: 55, y: 350 }, { x: 80, y: 310 }, { x: 110, y: 270 },
        { x: 150, y: 235 }, { x: 205, y: 190 }, { x: 270, y: 165 },
        { x: 335, y: 173 }, { x: 385, y: 202 }, { x: 425, y: 245 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 450, y: 551 }, { x: 450, y: 475 }, { x: 450, y: 395 },
        { x: 450, y: 315 }, { x: 450, y: 235 }, { x: 450, y: 155 },
        { x: 450, y: 75 }, { x: 450, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 590, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("य"),
  },
  // Opiaterein's animation and the Central Hindi Directorate deskbook agree
  // on three runs: the descending stem and clockwise lower loop, the restarted
  // diagonal tail, and the headline. JackPotte documents a joined loop-and-tail
  // variation; this path follows the corroborated three-stroke form.
  [ductusKey("devanagari", "र")]: {
    script: "devanagari",
    glyph: "र",
    strokes: [
      { segments: [{ label: "descend and curl clockwise around the lower loop", path: [
        { x: 285, y: 551 }, { x: 285, y: 510 }, { x: 286, y: 470 },
        { x: 280, y: 430 }, { x: 265, y: 390 }, { x: 245, y: 360 },
        { x: 220, y: 340 }, { x: 195, y: 325 }, { x: 175, y: 342 },
        { x: 150, y: 360 }, { x: 120, y: 375 }, { x: 90, y: 370 },
        { x: 65, y: 350 }, { x: 55, y: 325 }, { x: 63, y: 300 },
        { x: 82, y: 280 }, { x: 105, y: 260 }, { x: 130, y: 245 },
        { x: 155, y: 245 },
      ] }] },
      { segments: [{ label: "lift, then draw the diagonal tail down-right", path: [
        { x: 145, y: 235 }, { x: 170, y: 205 }, { x: 200, y: 170 },
        { x: 235, y: 135 }, { x: 270, y: 100 }, { x: 305, y: 65 },
        { x: 345, y: 30 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 420, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("र"),
  },
  // Opiaterein's animation and the Central Hindi Directorate deskbook agree
  // on four runs: the clockwise open loop, diagonal arm, descending right stem,
  // and headline. JackPotte documents a stem-first order variation; this path
  // follows the corroborated loop-first form.
  [ductusKey("devanagari", "ल")]: {
    script: "devanagari",
    glyph: "ल",
    strokes: [
      { segments: [{ label: "curve up and clockwise around the open left loop", path: [
        { x: 255, y: 5 }, { x: 220, y: 25 }, { x: 185, y: 50 },
        { x: 150, y: 80 }, { x: 120, y: 115 }, { x: 95, y: 155 },
        { x: 75, y: 205 }, { x: 70, y: 255 }, { x: 82, y: 305 },
        { x: 110, y: 345 }, { x: 150, y: 375 }, { x: 195, y: 392 },
        { x: 235, y: 392 }, { x: 270, y: 380 }, { x: 300, y: 360 },
        { x: 325, y: 330 }, { x: 345, y: 295 }, { x: 350, y: 260 },
        { x: 330, y: 240 }, { x: 300, y: 260 },
      ] }] },
      { segments: [{ label: "lift, then sweep the diagonal arm up-right", path: [
        { x: 300, y: 260 }, { x: 320, y: 280 }, { x: 340, y: 300 },
        { x: 365, y: 325 }, { x: 390, y: 350 }, { x: 420, y: 375 },
        { x: 455, y: 393 }, { x: 495, y: 395 }, { x: 510, y: 395 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 548, y: 551 }, { x: 548, y: 475 }, { x: 548, y: 395 },
        { x: 548, y: 315 }, { x: 548, y: 235 }, { x: 548, y: 155 },
        { x: 548, y: 75 }, { x: 548, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 565, y: 585 }, { x: 690, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ल"),
  },
  // JackPotte's animation and the Central Hindi Directorate deskbook agree on
  // three parts: the counterclockwise loop, descending right stem, and final
  // headline. The animation supplies the within-run directions and two lifts.
  [ductusKey("devanagari", "व")]: {
    script: "devanagari",
    glyph: "व",
    strokes: [
      { segments: [{ label: "circle counterclockwise around the left loop", path: [
        { x: 350, y: 415 }, { x: 305, y: 428 }, { x: 255, y: 430 },
        { x: 205, y: 422 }, { x: 160, y: 402 }, { x: 125, y: 375 },
        { x: 100, y: 340 }, { x: 87, y: 300 }, { x: 88, y: 260 },
        { x: 105, y: 220 }, { x: 140, y: 185 }, { x: 190, y: 160 },
        { x: 240, y: 150 }, { x: 290, y: 158 }, { x: 335, y: 180 },
        { x: 370, y: 215 }, { x: 392, y: 260 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 427, y: 551 }, { x: 427, y: 475 }, { x: 427, y: 395 },
        { x: 427, y: 315 }, { x: 427, y: 235 }, { x: 427, y: 155 },
        { x: 427, y: 75 }, { x: 427, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 495, y: 585 }, { x: 565, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("व"),
  },
  // Both animations and the Central Hindi Directorate deskbook agree on three
  // parts: one joined double-loop body and tail, the descending right stem,
  // and the final headline. Opiaterein's holds make the two lifts explicit.
  [ductusKey("devanagari", "श")]: {
    script: "devanagari",
    glyph: "श",
    strokes: [
      { segments: [{ label: "trace the joined double-loop body and diagonal tail", path: [
        { x: 240, y: 380 }, { x: 220, y: 395 }, { x: 175, y: 410 },
        { x: 135, y: 440 }, { x: 110, y: 480 }, { x: 105, y: 520 },
        { x: 120, y: 560 }, { x: 155, y: 590 }, { x: 200, y: 605 },
        { x: 245, y: 600 }, { x: 285, y: 580 }, { x: 315, y: 545 },
        { x: 335, y: 500 }, { x: 340, y: 450 }, { x: 330, y: 400 },
        { x: 310, y: 350 }, { x: 275, y: 310 }, { x: 235, y: 280 },
        { x: 190, y: 260 }, { x: 175, y: 265 }, { x: 155, y: 278 },
        { x: 115, y: 290 }, { x: 75, y: 280 }, { x: 50, y: 255 },
        { x: 55, y: 225 }, { x: 85, y: 200 }, { x: 125, y: 192 },
        { x: 165, y: 210 }, { x: 200, y: 245 }, { x: 205, y: 220 },
        { x: 235, y: 185 }, { x: 270, y: 145 }, { x: 330, y: 80 },
        { x: 350, y: 25 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 550, y: 550 }, { x: 550, y: 475 }, { x: 550, y: 395 },
        { x: 550, y: 315 }, { x: 550, y: 235 }, { x: 550, y: 155 },
        { x: 550, y: 75 }, { x: 550, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 425, y: 585 }, { x: 480, y: 585 }, { x: 535, y: 585 },
        { x: 590, y: 585 }, { x: 645, y: 585 }, { x: 690, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("श"),
  },
  // JackPotte's animation joins the descending left stem, hook, and tail, then
  // restarts for the crossbar, right stem, and headline. The Directorate
  // deskbook confirms that order while staging the hook and tail separately.
  [ductusKey("devanagari", "स")]: {
    script: "devanagari",
    glyph: "स",
    strokes: [
      { segments: [{ label: "descend through the hook and diagonal tail", path: [
        { x: 250, y: 540 }, { x: 255, y: 505 }, { x: 265, y: 465 },
        { x: 265, y: 425 }, { x: 255, y: 385 }, { x: 235, y: 350 },
        { x: 205, y: 320 }, { x: 170, y: 305 }, { x: 135, y: 315 },
        { x: 100, y: 340 }, { x: 70, y: 340 }, { x: 60, y: 315 },
        { x: 75, y: 285 }, { x: 110, y: 260 }, { x: 135, y: 240 },
        { x: 140, y: 210 }, { x: 155, y: 180 }, { x: 180, y: 145 },
        { x: 210, y: 110 }, { x: 240, y: 75 }, { x: 265, y: 35 },
        { x: 285, y: 0 },
      ] }] },
      { segments: [{ label: "lift, then draw the middle crossbar left-to-right", path: [
        { x: 230, y: 300 }, { x: 280, y: 285 }, { x: 340, y: 280 },
        { x: 400, y: 280 }, { x: 460, y: 285 }, { x: 520, y: 300 },
        { x: 550, y: 310 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 546, y: 550 }, { x: 546, y: 475 }, { x: 546, y: 395 },
        { x: 546, y: 315 }, { x: 546, y: 235 }, { x: 546, y: 155 },
        { x: 546, y: 75 }, { x: 546, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 495, y: 585 }, { x: 565, y: 585 },
        { x: 635, y: 585 }, { x: 685, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("स"),
  },
  // Opiaterein's animation joins the descending right stem, leftward shoulder,
  // and clockwise hooked body, then restarts for the down-right outer tail and
  // the headline. The Directorate deskbook confirms that component order while
  // staging the joined first body across more buildup steps.
  [ductusKey("devanagari", "ह")]: {
    script: "devanagari",
    glyph: "ह",
    strokes: [
      { segments: [{ label: "descend, sweep left, and curve around the hooked body", path: [
        { x: 402, y: 550 }, { x: 402, y: 510 }, { x: 402, y: 470 },
        { x: 402, y: 430 }, { x: 360, y: 430 }, { x: 315, y: 430 },
        { x: 270, y: 430 }, { x: 225, y: 430 }, { x: 180, y: 420 },
        { x: 140, y: 400 }, { x: 110, y: 370 }, { x: 105, y: 340 },
        { x: 110, y: 310 }, { x: 135, y: 285 }, { x: 175, y: 265 },
        { x: 225, y: 258 }, { x: 280, y: 260 }, { x: 335, y: 245 },
        { x: 385, y: 220 }, { x: 425, y: 185 }, { x: 445, y: 145 },
        { x: 445, y: 110 }, { x: 425, y: 70 }, { x: 390, y: 40 },
      ] }] },
      { segments: [{ label: "lift, then sweep down-left and through the diagonal tail", path: [
        { x: 150, y: 245 }, { x: 125, y: 220 }, { x: 100, y: 185 },
        { x: 88, y: 145 }, { x: 90, y: 105 }, { x: 110, y: 65 },
        { x: 145, y: 25 }, { x: 190, y: -10 }, { x: 240, y: -40 },
        { x: 295, y: -70 }, { x: 350, y: -100 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 70, y: 585 }, { x: 135, y: 585 },
        { x: 200, y: 585 }, { x: 265, y: 585 }, { x: 330, y: 585 },
        { x: 395, y: 585 }, { x: 460, y: 585 }, { x: 540, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ह"),
  },
  // The native-teacher demonstration keeps lowercase а in one pen-down run:
  // its rounded body closes at the right and flows into the finishing stem.
  // Noto Sans Cyrillic uses a double-storey printed outline, so the source's
  // opening loop is fitted through the font's extra shoulder without adding a
  // lift that the handwriting demonstration does not contain.
  [ductusKey("cyrillic", "а")]: {
    script: "cyrillic",
    glyph: "а",
    strokes: [
      {
        segments: [
          {
            label: "sweep over the shoulder and around the round body",
            path: [
              { x: 110, y: 455 }, { x: 155, y: 495 }, { x: 215, y: 515 },
              { x: 285, y: 515 }, { x: 345, y: 490 }, { x: 395, y: 445 },
              { x: 420, y: 395 }, { x: 420, y: 345 }, { x: 390, y: 305 },
              { x: 325, y: 300 }, { x: 250, y: 300 }, { x: 175, y: 300 },
              { x: 145, y: 285 }, { x: 95, y: 245 }, { x: 75, y: 190 },
              { x: 85, y: 125 }, { x: 120, y: 70 }, { x: 175, y: 35 },
              { x: 240, y: 25 }, { x: 305, y: 45 }, { x: 355, y: 85 },
              { x: 395, y: 140 }, { x: 420, y: 205 }, { x: 420, y: 265 },
              { x: 420, y: 305 },
            ],
          },
          {
            label: "continue down the right-hand finishing stem",
            path: [
              { x: 420, y: 305 }, { x: 430, y: 250 }, { x: 435, y: 190 },
              { x: 435, y: 125 }, { x: 435, y: 65 }, { x: 440, y: 20 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("а"),
  },
  // RussianIrina closes the rounded lowercase б body and immediately climbs
  // into its top flag. The source's direct diagonal crossing is a handwritten
  // form; Noto Sans Cyrillic joins the ascender at the upper-left shoulder, so
  // this one-run fit carries the pen left along the printed shoulder first.
  [ductusKey("cyrillic", "б")]: {
    script: "cyrillic",
    glyph: "б",
    strokes: [
      {
        segments: [
          {
            label: "circle counterclockwise around the rounded lower body",
            path: [
              { x: 430, y: 480 }, { x: 350, y: 495 }, { x: 270, y: 495 },
              { x: 190, y: 470 }, { x: 125, y: 420 }, { x: 90, y: 340 },
              { x: 85, y: 250 }, { x: 105, y: 165 }, { x: 155, y: 95 },
              { x: 225, y: 45 }, { x: 305, y: 25 }, { x: 380, y: 45 },
              { x: 445, y: 90 }, { x: 490, y: 155 }, { x: 510, y: 235 },
              { x: 505, y: 315 }, { x: 480, y: 400 }, { x: 430, y: 480 },
            ],
          },
          {
            label: "continue through the rising shoulder and sweep the top flag right",
            path: [
              { x: 430, y: 480 }, { x: 350, y: 490 }, { x: 270, y: 490 },
              { x: 195, y: 485 }, { x: 195, y: 465 }, { x: 195, y: 450 },
              { x: 170, y: 445 }, { x: 140, y: 445 }, { x: 140, y: 480 },
              { x: 140, y: 520 }, { x: 150, y: 545 }, { x: 165, y: 585 },
              { x: 185, y: 620 }, { x: 215, y: 660 }, { x: 260, y: 690 },
              { x: 320, y: 710 }, { x: 390, y: 720 }, { x: 460, y: 730 },
              { x: 500, y: 735 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("б"),
  },
  // RussianIrina starts lowercase в at the baseline, climbs through its tall
  // handwritten ascender loop, descends to the baseline, and continues around
  // the lower bowl without lifting. Noto Sans Cyrillic prints two compact bowls
  // on a straight stem, so the same one-run order is fitted through the upper
  // bowl, down the stem, and counterclockwise around the lower bowl.
  [ductusKey("cyrillic", "в")]: {
    script: "cyrillic",
    glyph: "в",
    strokes: [
      {
        segments: [
          {
            label: "climb through the upper loop and descend to the baseline",
            path: [
              { x: 130, y: 20 }, { x: 130, y: 100 }, { x: 130, y: 200 },
              { x: 130, y: 300 }, { x: 130, y: 400 }, { x: 130, y: 500 },
              { x: 220, y: 500 }, { x: 310, y: 500 }, { x: 380, y: 480 },
              { x: 430, y: 445 }, { x: 455, y: 400 }, { x: 450, y: 355 },
              { x: 420, y: 320 }, { x: 365, y: 300 }, { x: 295, y: 290 },
              { x: 220, y: 290 }, { x: 150, y: 290 }, { x: 130, y: 260 },
              { x: 130, y: 180 }, { x: 130, y: 100 }, { x: 130, y: 20 },
            ],
          },
          {
            label: "continue counterclockwise around the rounded lower bowl",
            path: [
              { x: 130, y: 20 }, { x: 220, y: 35 }, { x: 310, y: 35 },
              { x: 385, y: 50 }, { x: 440, y: 80 }, { x: 470, y: 120 },
              { x: 475, y: 165 }, { x: 455, y: 205 }, { x: 415, y: 235 },
              { x: 360, y: 260 }, { x: 295, y: 270 }, { x: 220, y: 270 },
              { x: 150, y: 270 }, { x: 130, y: 260 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("в"),
  },
  // RussianIrina writes lowercase г as one rounded two-hump cursive run. The
  // bundled Noto glyph is the block-like isolated form, so its zero-lift order
  // is preserved by climbing the upright, sweeping and retracing the top bar,
  // then descending the upright. Connected cursive restores the exit hump.
  [ductusKey("cyrillic", "г")]: {
    script: "cyrillic",
    glyph: "г",
    strokes: [
      {
        segments: [
          {
            label: "climb the upright and sweep the top bar right",
            path: [
              { x: 130, y: 20 }, { x: 130, y: 120 }, { x: 130, y: 240 },
              { x: 130, y: 360 }, { x: 130, y: 500 }, { x: 220, y: 500 },
              { x: 310, y: 500 }, { x: 390, y: 500 },
            ],
          },
          {
            label: "reverse along the top and descend to the baseline",
            path: [
              { x: 390, y: 500 }, { x: 310, y: 500 }, { x: 220, y: 500 },
              { x: 130, y: 500 }, { x: 130, y: 360 }, { x: 130, y: 240 },
              { x: 130, y: 120 }, { x: 130, y: 20 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("г"),
  },
  // RussianIrina writes lowercase д as one cursive body-to-descender run. The
  // bundled Noto glyph is the block-like isolated form, so its zero-lift order
  // is preserved by circling the body before retracing both feet through their
  // joined base shelf. Connected cursive restores the below-baseline loop.
  [ductusKey("cyrillic", "д")]: {
    script: "cyrillic",
    glyph: "д",
    strokes: [
      {
        segments: [
          {
            label: "circle counterclockwise around the closed body",
            path: [
              { x: 470, y: 462 }, { x: 390, y: 500 }, { x: 300, y: 500 },
              { x: 205, y: 500 }, { x: 190, y: 420 }, { x: 185, y: 330 },
              { x: 175, y: 240 }, { x: 150, y: 150 }, { x: 110, y: 74 },
              { x: 190, y: 35 }, { x: 290, y: 35 }, { x: 390, y: 35 },
              { x: 470, y: 74 }, { x: 470, y: 170 }, { x: 470, y: 270 },
              { x: 470, y: 370 }, { x: 470, y: 462 },
            ],
          },
          {
            label: "descend, retrace both feet, and finish along the base shelf",
            path: [
              { x: 470, y: 462 }, { x: 470, y: 330 }, { x: 470, y: 200 },
              { x: 470, y: 74 }, { x: 550, y: 35 }, { x: 550, y: -50 },
              { x: 550, y: -110 }, { x: 550, y: -50 }, { x: 550, y: 35 },
              { x: 450, y: 35 }, { x: 350, y: 35 }, { x: 250, y: 35 },
              { x: 150, y: 35 }, { x: 55, y: 35 }, { x: 55, y: -50 },
              { x: 55, y: -110 }, { x: 55, y: -50 }, { x: 55, y: 35 },
              { x: 150, y: 35 }, { x: 250, y: 35 }, { x: 350, y: 35 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("д"),
  },
  // RussianIrina writes lowercase е as one upper-loop-to-lower-bowl cursive
  // run. Noto Sans Cyrillic prints a compact e with a long middle bar, so the
  // sourced zero-lift order is fitted by sweeping and reversing through that
  // bar before continuing counterclockwise around the lower bowl.
  [ductusKey("cyrillic", "е")]: {
    script: "cyrillic",
    glyph: "е",
    strokes: [
      {
        segments: [
          {
            label: "curve around the upper bowl and sweep through the middle",
            path: [
              { x: 430, y: 380 }, { x: 390, y: 455 }, { x: 320, y: 505 },
              { x: 245, y: 505 }, { x: 175, y: 470 }, { x: 115, y: 410 },
              { x: 85, y: 340 }, { x: 85, y: 285 }, { x: 150, y: 285 },
              { x: 240, y: 285 }, { x: 330, y: 285 }, { x: 440, y: 285 },
            ],
          },
          {
            label: "reverse through the middle and circle the lower bowl",
            path: [
              { x: 440, y: 285 }, { x: 330, y: 285 }, { x: 240, y: 285 },
              { x: 150, y: 285 }, { x: 85, y: 260 }, { x: 80, y: 185 },
              { x: 105, y: 115 }, { x: 160, y: 55 }, { x: 230, y: 25 },
              { x: 305, y: 25 }, { x: 375, y: 40 }, { x: 440, y: 70 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("е"),
  },
  // RussianIrina writes lowercase ё by completing the same looped body as е,
  // then lifting for the left dot and once more for the right dot. The Noto
  // Sans Cyrillic fit reuses the printed e route and places both circular dots
  // as separate runs in the demonstrated left-to-right order.
  [ductusKey("cyrillic", "ё")]: {
    script: "cyrillic",
    glyph: "ё",
    strokes: [
      {
        segments: [
          {
            label: "curve around the upper bowl and sweep through the middle",
            path: [
              { x: 430, y: 380 }, { x: 390, y: 455 }, { x: 320, y: 505 },
              { x: 245, y: 505 }, { x: 175, y: 470 }, { x: 115, y: 410 },
              { x: 85, y: 340 }, { x: 85, y: 285 }, { x: 150, y: 285 },
              { x: 240, y: 285 }, { x: 330, y: 285 }, { x: 440, y: 285 },
            ],
          },
          {
            label: "reverse through the middle and circle the lower bowl",
            path: [
              { x: 440, y: 285 }, { x: 330, y: 285 }, { x: 240, y: 285 },
              { x: 150, y: 285 }, { x: 85, y: 260 }, { x: 80, y: 185 },
              { x: 105, y: 115 }, { x: 160, y: 55 }, { x: 230, y: 25 },
              { x: 305, y: 25 }, { x: 375, y: 40 }, { x: 440, y: 70 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift and place the left dot",
            path: [
              { x: 197, y: 674 }, { x: 203, y: 674 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again and place the right dot",
            path: [
              { x: 379, y: 674 }, { x: 385, y: 674 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ё"),
  },
  // RussianIrina writes lowercase ж as one continuous rounded left-to-centre-
  // to-right run. Noto Sans Cyrillic prints a straight upright with four arms,
  // so the sourced zero-lift order is fitted by retracing each side junction
  // and the central upright before continuing into the opposite wing.
  [ductusKey("cyrillic", "ж")]: {
    script: "cyrillic",
    glyph: "ж",
    strokes: [
      {
        segments: [
          {
            label: "trace the left wings and rise through the centre",
            path: [
              { x: 60, y: 30 }, { x: 110, y: 100 }, { x: 190, y: 220 },
              { x: 265, y: 275 }, { x: 210, y: 350 }, { x: 140, y: 455 },
              { x: 75, y: 510 }, { x: 140, y: 455 }, { x: 210, y: 350 },
              { x: 265, y: 275 }, { x: 340, y: 275 }, { x: 380, y: 275 },
              { x: 380, y: 380 }, { x: 380, y: 510 }, { x: 380, y: 380 },
              { x: 380, y: 275 }, { x: 380, y: 150 }, { x: 380, y: 30 },
              { x: 380, y: 150 }, { x: 380, y: 275 },
            ],
          },
          {
            label: "retrace the centre and trace the right wings",
            path: [
              { x: 380, y: 275 }, { x: 495, y: 275 }, { x: 560, y: 370 },
              { x: 630, y: 470 },
              { x: 690, y: 510 }, { x: 630, y: 470 }, { x: 560, y: 370 },
              { x: 495, y: 275 }, { x: 560, y: 180 }, { x: 630, y: 80 },
              { x: 700, y: 30 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ж"),
  },
  // RussianIrina writes lowercase з as one continuous smaller-upper-lobe to
  // larger-lower-lobe run with a cursive exit. Noto Sans Cyrillic omits the
  // exit, so the sourced zero-lift order is fitted by circling both printed
  // lobes through their middle junction and retracing to the lower-right tip.
  [ductusKey("cyrillic", "з")]: {
    script: "cyrillic",
    glyph: "з",
    strokes: [
      {
        segments: [
          {
            label: "circle the smaller upper lobe and descend through the middle",
            path: [
              { x: 80, y: 485 }, { x: 155, y: 510 }, { x: 225, y: 510 },
              { x: 300, y: 500 }, { x: 365, y: 460 }, { x: 390, y: 410 },
              { x: 385, y: 360 }, { x: 345, y: 320 }, { x: 285, y: 285 },
              { x: 200, y: 280 },
            ],
          },
          {
            label: "circle the larger lower lobe and finish at the lower right",
            path: [
              { x: 200, y: 280 }, { x: 285, y: 275 }, { x: 360, y: 245 },
              { x: 405, y: 200 }, { x: 410, y: 145 }, { x: 385, y: 90 },
              { x: 325, y: 45 }, { x: 245, y: 25 }, { x: 160, y: 25 },
              { x: 85, y: 45 }, { x: 160, y: 25 }, { x: 245, y: 25 },
              { x: 325, y: 45 }, { x: 405, y: 75 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("з"),
  },
  // RussianIrina writes lowercase и as one continuous left-stem, rising-
  // diagonal, right-stem run with cursive entry and exit joins. The bundled
  // printed glyph omits those joins, so the sourced zero-lift order is fitted
  // directly through its two stems and backwards-N diagonal.
  [ductusKey("cyrillic", "и")]: {
    script: "cyrillic",
    glyph: "и",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 125, y: 510 }, { x: 125, y: 390 }, { x: 125, y: 270 },
              { x: 125, y: 150 }, { x: 125, y: 25 }, { x: 160, y: 25 },
              { x: 190, y: 40 },
            ],
          },
          {
            label: "rise diagonally to the upper right",
            path: [
              { x: 190, y: 40 }, { x: 225, y: 100 }, { x: 270, y: 180 },
              { x: 315, y: 255 }, { x: 360, y: 335 }, { x: 405, y: 410 },
              { x: 450, y: 485 }, { x: 475, y: 510 },
            ],
          },
          {
            label: "descend the right stem and finish at the baseline",
            path: [
              { x: 475, y: 510 }, { x: 475, y: 390 }, { x: 475, y: 270 },
              { x: 475, y: 150 }, { x: 475, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("и"),
  },
  // RussianIrina writes the lowercase й body with the same continuous motion
  // as и, then lifts once and adds its breve from left to right. The fitted
  // path preserves that body-before-breve order across the bundled printed
  // backwards-N body and its separate curved mark.
  [ductusKey("cyrillic", "й")]: {
    script: "cyrillic",
    glyph: "й",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 125, y: 510 }, { x: 125, y: 390 }, { x: 125, y: 270 },
              { x: 125, y: 150 }, { x: 125, y: 25 }, { x: 160, y: 25 },
              { x: 190, y: 40 },
            ],
          },
          {
            label: "rise diagonally to the upper right",
            path: [
              { x: 190, y: 40 }, { x: 225, y: 100 }, { x: 270, y: 180 },
              { x: 315, y: 255 }, { x: 360, y: 335 }, { x: 405, y: 410 },
              { x: 450, y: 485 }, { x: 475, y: 510 },
            ],
          },
          {
            label: "descend the right stem and finish at the baseline",
            path: [
              { x: 475, y: 510 }, { x: 475, y: 390 }, { x: 475, y: 270 },
              { x: 475, y: 150 }, { x: 475, y: 25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the breve from left to right",
            path: [
              { x: 195, y: 715 }, { x: 220, y: 675 }, { x: 265, y: 640 },
              { x: 310, y: 635 }, { x: 355, y: 640 }, { x: 400, y: 675 },
              { x: 430, y: 715 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("й"),
  },
  // RussianIrina writes lowercase к in one continuous school-hand motion:
  // descend the left stem, rise into the upper arm and return to the middle,
  // then continue through the lower arm. The fitted path preserves that order
  // while tracing the bundled printed vertical and its two angular diagonals.
  [ductusKey("cyrillic", "к")]: {
    script: "cyrillic",
    glyph: "к",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 510 },
              { x: 129, y: 390 },
              { x: 129, y: 270 },
              { x: 129, y: 150 },
              { x: 129, y: 25 },
            ],
          },
          {
            label: "rise through the upper arm and return to the middle junction",
            path: [
              { x: 129, y: 25 },
              { x: 129, y: 120 },
              { x: 129, y: 220 },
              { x: 190, y: 274 },
              { x: 250, y: 310 },
              { x: 370, y: 400 },
              { x: 435, y: 490 },
              { x: 465, y: 510 },
              { x: 420, y: 470 },
              { x: 360, y: 400 },
              { x: 300, y: 320 },
              { x: 250, y: 274 },
              { x: 190, y: 274 },
            ],
          },
          {
            label: "continue down-right through the lower arm to the baseline",
            path: [
              { x: 190, y: 274 },
              { x: 250, y: 250 },
              { x: 300, y: 210 },
              { x: 350, y: 150 },
              { x: 410, y: 70 },
              { x: 475, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("к"),
  },
  // RussianIrina writes lowercase л in one continuous school-hand motion:
  // curl around the baseline hook, rise to the apex, then descend the right
  // leg. The fitted path preserves that order while tracing the bundled
  // printed glyph's curved left leg, horizontal shoulder, and upright stem.
  [ductusKey("cyrillic", "л")]: {
    script: "cyrillic",
    glyph: "л",
    strokes: [
      {
        segments: [
          {
            label: "curve from the baseline hook up the left leg",
            path: [
              { x: 25, y: 30 },
              { x: 55, y: 25 },
              { x: 85, y: 35 },
              { x: 110, y: 75 },
              { x: 125, y: 140 },
              { x: 135, y: 240 },
              { x: 145, y: 360 },
              { x: 160, y: 460 },
              { x: 175, y: 500 },
            ],
          },
          {
            label: "sweep right along the top shoulder",
            path: [
              { x: 175, y: 500 },
              { x: 260, y: 500 },
              { x: 360, y: 500 },
              { x: 458, y: 500 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 458, y: 500 },
              { x: 458, y: 380 },
              { x: 458, y: 260 },
              { x: 458, y: 140 },
              { x: 458, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("л"),
  },
  // RussianIrina writes lowercase м in one continuous school-hand motion:
  // rise from the entry hook to the first apex, descend and rise through the
  // second arch, then descend the right leg. The fitted path preserves that
  // order while tracing the bundled printed stems and deep central V.
  [ductusKey("cyrillic", "м")]: {
    script: "cyrillic",
    glyph: "м",
    strokes: [
      {
        segments: [
          {
            label: "rise from the baseline through the left stem",
            path: [
              { x: 126, y: 25 },
              { x: 126, y: 140 },
              { x: 126, y: 260 },
              { x: 126, y: 380 },
              { x: 126, y: 500 },
              { x: 185, y: 500 },
            ],
          },
          {
            label: "descend diagonally to the central valley",
            path: [
              { x: 185, y: 500 },
              { x: 230, y: 400 },
              { x: 275, y: 290 },
              { x: 325, y: 170 },
              { x: 380, y: 50 },
            ],
          },
          {
            label: "rise diagonally to the second apex",
            path: [
              { x: 380, y: 50 },
              { x: 435, y: 170 },
              { x: 490, y: 290 },
              { x: 535, y: 400 },
              { x: 585, y: 500 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 585, y: 500 },
              { x: 642, y: 500 },
              { x: 642, y: 380 },
              { x: 642, y: 260 },
              { x: 642, y: 140 },
              { x: 642, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("м"),
  },
  // RussianIrina writes lowercase н in one continuous school-hand motion:
  // descend the left stem, turn upward through the rounded middle bridge,
  // rise to the right shoulder, then descend the right stem. The fitted path
  // preserves that order across the bundled printed stems and middle bar.
  [ductusKey("cyrillic", "н")]: {
    script: "cyrillic",
    glyph: "н",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 510 },
              { x: 129, y: 390 },
              { x: 129, y: 274 },
              { x: 129, y: 150 },
              { x: 129, y: 25 },
            ],
          },
          {
            label: "retrace to the middle bridge and rise to the upper right",
            path: [
              { x: 129, y: 25 },
              { x: 129, y: 140 },
              { x: 129, y: 274 },
              { x: 220, y: 274 },
              { x: 310, y: 274 },
              { x: 400, y: 274 },
              { x: 485, y: 274 },
              { x: 485, y: 390 },
              { x: 485, y: 510 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 485, y: 510 },
              { x: 485, y: 390 },
              { x: 485, y: 274 },
              { x: 485, y: 150 },
              { x: 485, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("н"),
  },
  // RussianIrina writes lowercase о as one continuous counterclockwise oval:
  // begin on the upper-right shoulder, pass over the top and down the left,
  // sweep through the bottom, then rise on the right and close. The fitted
  // path preserves that order in the bundled printed oval.
  [ductusKey("cyrillic", "о")]: {
    script: "cyrillic",
    glyph: "о",
    strokes: [
      {
        segments: [
          {
            label: "curve left over the top and descend the left side",
            path: [
              { x: 430, y: 450 },
              { x: 380, y: 490 },
              { x: 300, y: 510 },
              { x: 220, y: 500 },
              { x: 145, y: 450 },
              { x: 105, y: 380 },
              { x: 95, y: 270 },
              { x: 105, y: 170 },
              { x: 150, y: 90 },
              { x: 220, y: 40 },
              { x: 300, y: 25 },
            ],
          },
          {
            label: "sweep through the bottom and rise to close the oval",
            path: [
              { x: 300, y: 25 },
              { x: 380, y: 40 },
              { x: 450, y: 90 },
              { x: 490, y: 170 },
              { x: 500, y: 268 },
              { x: 490, y: 360 },
              { x: 450, y: 440 },
              { x: 380, y: 490 },
              { x: 300, y: 510 },
              { x: 380, y: 490 },
              { x: 430, y: 450 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("о"),
  },
  // RussianIrina writes lowercase п in one continuous school-hand motion:
  // descend the left stem, turn upward into the rounded top shoulder, then
  // descend the right stem. The fitted path preserves that order across the
  // bundled printed stems and horizontal top bar.
  [ductusKey("cyrillic", "п")]: {
    script: "cyrillic",
    glyph: "п",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 500 },
              { x: 129, y: 380 },
              { x: 129, y: 260 },
              { x: 129, y: 140 },
              { x: 129, y: 25 },
            ],
          },
          {
            label: "retrace to the top shoulder and sweep right",
            path: [
              { x: 129, y: 25 },
              { x: 129, y: 140 },
              { x: 129, y: 260 },
              { x: 129, y: 380 },
              { x: 129, y: 500 },
              { x: 220, y: 500 },
              { x: 310, y: 500 },
              { x: 400, y: 500 },
              { x: 477, y: 500 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 477, y: 500 },
              { x: 477, y: 380 },
              { x: 477, y: 260 },
              { x: 477, y: 140 },
              { x: 477, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("п"),
  },
  // RussianIrina writes lowercase р in one continuous school-hand motion:
  // descend below the baseline, retrace upward, then curve through the rounded
  // shoulder and baseline exit. The fitted path preserves that stem-before-bowl
  // order while closing the bowl around the bundled printed outline.
  [ductusKey("cyrillic", "р")]: {
    script: "cyrillic",
    glyph: "р",
    strokes: [
      {
        segments: [
          {
            label: "descend the stem below the baseline",
            path: [
              { x: 129, y: 510 },
              { x: 129, y: 350 },
              { x: 129, y: 190 },
              { x: 129, y: 30 },
              { x: 129, y: -100 },
              { x: 129, y: -200 },
            ],
          },
          {
            label: "retrace to the upper shoulder and curve right",
            path: [
              { x: 129, y: -200 },
              { x: 129, y: -80 },
              { x: 129, y: 40 },
              { x: 129, y: 180 },
              { x: 129, y: 320 },
              { x: 129, y: 450 },
              { x: 157, y: 450 },
              { x: 173, y: 463 },
              { x: 190, y: 475 },
              { x: 220, y: 490 },
              { x: 280, y: 510 },
              { x: 370, y: 500 },
              { x: 450, y: 450 },
              { x: 500, y: 370 },
              { x: 515, y: 270 },
            ],
          },
          {
            label: "sweep around the bowl and return to the stem",
            path: [
              { x: 515, y: 270 },
              { x: 505, y: 170 },
              { x: 455, y: 90 },
              { x: 380, y: 40 },
              { x: 300, y: 25 },
              { x: 230, y: 45 },
              { x: 185, y: 100 },
              { x: 177, y: 175 },
              { x: 165, y: 220 },
              { x: 150, y: 269 },
              { x: 129, y: 269 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("р"),
  },
  // RussianIrina writes lowercase с in one continuous counterclockwise motion:
  // curve from the upper-right tip across the top, descend the left side, then
  // sweep through the bottom into the lower-right exit. The fitted path keeps
  // that open-curve order across the bundled wider printed outline.
  [ductusKey("cyrillic", "с")]: {
    script: "cyrillic",
    glyph: "с",
    strokes: [
      {
        segments: [
          {
            label: "curve left over the top and descend the left side",
            path: [
              { x: 438, y: 480 },
              { x: 380, y: 510 },
              { x: 306, y: 509 },
              { x: 230, y: 500 },
              { x: 160, y: 450 },
              { x: 110, y: 380 },
              { x: 94, y: 263 },
            ],
          },
          {
            label: "sweep through the bottom and rise to the lower-right tip",
            path: [
              { x: 94, y: 263 },
              { x: 100, y: 160 },
              { x: 145, y: 80 },
              { x: 220, y: 35 },
              { x: 307, y: 27 },
              { x: 380, y: 35 },
              { x: 439, y: 51 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("с"),
  },
  // RussianIrina writes lowercase т as one joined, rounded m-like school-hand
  // run: descend the first stem, pass through two arches, then descend and exit.
  // The fitted path preserves the initial descent and zero lifts while routing
  // that continuous motion through the bundled printed central stem and top bar.
  [ductusKey("cyrillic", "т")]: {
    script: "cyrillic",
    glyph: "т",
    strokes: [
      {
        segments: [
          {
            label: "descend the central stem to the baseline",
            path: [
              { x: 231, y: 499 },
              { x: 231, y: 380 },
              { x: 231, y: 260 },
              { x: 231, y: 140 },
              { x: 231, y: 25 },
            ],
          },
          {
            label: "retrace to the top junction and sweep left",
            path: [
              { x: 231, y: 25 },
              { x: 231, y: 140 },
              { x: 231, y: 260 },
              { x: 231, y: 380 },
              { x: 231, y: 499 },
              { x: 150, y: 499 },
              { x: 52, y: 499 },
            ],
          },
          {
            label: "retrace through the junction and sweep to the right tip",
            path: [
              { x: 52, y: 499 },
              { x: 150, y: 499 },
              { x: 231, y: 499 },
              { x: 320, y: 499 },
              { x: 413, y: 499 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("т"),
  },
  // RussianIrina writes lowercase у as one joined y-like school-hand run:
  // descend the left arm, rise through the right arm, then retrace into a
  // looped descender and exit. The fitted path preserves that zero-lift order
  // while following the printed arms and its unlooped left-curving terminal.
  [ductusKey("cyrillic", "у")]: {
    script: "cyrillic",
    glyph: "у",
    strokes: [
      {
        segments: [
          {
            label: "descend the left arm to the middle junction",
            path: [
              { x: 47, y: 500 },
              { x: 80, y: 430 },
              { x: 115, y: 340 },
              { x: 155, y: 240 },
              { x: 215, y: 100 },
            ],
          },
          {
            label: "turn and rise through the right arm",
            path: [
              { x: 215, y: 100 },
              { x: 260, y: 95 },
              { x: 315, y: 220 },
              { x: 365, y: 350 },
              { x: 460, y: 500 },
            ],
          },
          {
            label: "retrace to the junction and descend below the baseline",
            path: [
              { x: 460, y: 500 },
              { x: 410, y: 400 },
              { x: 360, y: 270 },
              { x: 310, y: 145 },
              { x: 260, y: 60 },
              { x: 235, y: -40 },
              { x: 220, y: -85 },
            ],
          },
          {
            label: "curve left through the descender terminal",
            path: [
              { x: 220, y: -85 },
              { x: 205, y: -125 },
              { x: 175, y: -165 },
              { x: 135, y: -195 },
              { x: 85, y: -200 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("у"),
  },
  // RussianIrina writes lowercase ф in two runs: the long central stem first,
  // then, after one lift, a linked left-loop-to-right-loop body. The fitted
  // path preserves that order while expanding the loops into the printed bowls.
  [ductusKey("cyrillic", "ф")]: {
    script: "cyrillic",
    glyph: "ф",
    strokes: [
      {
        segments: [
          {
            label: "descend the long central stem below the baseline",
            path: [
              { x: 368, y: 720 },
              { x: 368, y: 500 },
              { x: 368, y: 265 },
              { x: 368, y: 0 },
              { x: 368, y: -200 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift and curve over and around the left bowl",
            path: [
              { x: 368, y: 265 },
              { x: 368, y: 470 },
              { x: 325, y: 500 },
              { x: 240, y: 505 },
              { x: 160, y: 440 },
              { x: 95, y: 350 },
              { x: 95, y: 265 },
            ],
          },
          {
            label: "sweep through the lower-left curve to the centre",
            path: [
              { x: 95, y: 265 },
              { x: 95, y: 180 },
              { x: 160, y: 95 },
              { x: 240, y: 35 },
              { x: 325, y: 25 },
              { x: 368, y: 25 },
            ],
          },
          {
            label: "continue through the lower-right curve",
            path: [
              { x: 368, y: 25 },
              { x: 411, y: 25 },
              { x: 500, y: 35 },
              { x: 580, y: 100 },
              { x: 640, y: 180 },
              { x: 640, y: 265 },
            ],
          },
          {
            label: "rise over the right bowl to the upper junction",
            path: [
              { x: 640, y: 265 },
              { x: 640, y: 350 },
              { x: 580, y: 430 },
              { x: 500, y: 500 },
              { x: 411, y: 500 },
              { x: 368, y: 470 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ф"),
  },
  // RussianIrina writes lowercase х as two facing top-to-bottom curves: the
  // left run first, then the right run after one lift. The fitted path keeps
  // that run order while straightening the curves into the printed X arms.
  [ductusKey("cyrillic", "х")]: {
    script: "cyrillic",
    glyph: "х",
    strokes: [
      {
        segments: [
          {
            label: "descend from the upper-left tip to the central crossing",
            path: [
              { x: 68, y: 536 },
              { x: 160, y: 408 },
              { x: 256, y: 274 },
            ],
          },
          {
            label: "sweep down-left from the crossing to the lower-left tip",
            path: [
              { x: 256, y: 274 },
              { x: 158, y: 138 },
              { x: 58, y: 0 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift and descend from the upper-right tip to the crossing",
            path: [
              { x: 442, y: 536 },
              { x: 350, y: 408 },
              { x: 256, y: 274 },
            ],
          },
          {
            label: "sweep down-right from the crossing to the lower-right tip",
            path: [
              { x: 256, y: 274 },
              { x: 354, y: 138 },
              { x: 452, y: 0 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("х"),
  },
  // RussianIrina writes lowercase ц in one run: left stem down, joined rise
  // and descent through the right stem, then the tail. The fitted path squares
  // those joins and keeps the printed descender connected by retracing.
  [ductusKey("cyrillic", "ц")]: {
    script: "cyrillic",
    glyph: "ц",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 130, y: 536 },
              { x: 130, y: 280 },
              { x: 130, y: 37 },
            ],
          },
          {
            label: "sweep along the base and rise through the right stem",
            path: [
              { x: 130, y: 37 },
              { x: 300, y: 37 },
              { x: 477, y: 37 },
              { x: 477, y: 280 },
              { x: 477, y: 536 },
            ],
          },
          {
            label: "retrace the right stem and cross the tail shoulder",
            path: [
              { x: 477, y: 536 },
              { x: 477, y: 280 },
              { x: 477, y: 37 },
              { x: 560, y: 37 },
            ],
          },
          {
            label: "descend the short tail below the baseline",
            path: [
              { x: 560, y: 37 },
              { x: 560, y: -50 },
              { x: 560, y: -140 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ц"),
  },
  // RussianIrina writes lowercase ч in one run: short left stem down, a
  // rounded joined rise, then the full right stem down into an exit. The
  // fitted path opens that bridge into the printed shallow bowl.
  [ductusKey("cyrillic", "ч")]: {
    script: "cyrillic",
    glyph: "ч",
    strokes: [
      {
        segments: [
          {
            label: "descend the short left stem to the middle join",
            path: [
              { x: 104, y: 536 },
              { x: 104, y: 450 },
              { x: 104, y: 363 },
            ],
          },
          {
            label: "sweep through the bowl and rise along the right stem",
            path: [
              { x: 104, y: 363 },
              { x: 104, y: 255 },
              { x: 276, y: 218 },
              { x: 460, y: 250 },
              { x: 460, y: 390 },
              { x: 460, y: 536 },
            ],
          },
          {
            label: "descend the full right stem to the baseline",
            path: [
              { x: 460, y: 536 },
              { x: 460, y: 270 },
              { x: 460, y: 0 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ч"),
  },
  // RussianIrina writes lowercase ш in one run: descend each stem from left
  // to right and rise through the two rounded joins. The fitted path squares
  // those joins into the printed glyph's horizontal baseline bars.
  [ductusKey("cyrillic", "ш")]: {
    script: "cyrillic",
    glyph: "ш",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 126, y: 536 },
              { x: 126, y: 270 },
              { x: 126, y: 37 },
            ],
          },
          {
            label: "cross the first base join and rise through the middle stem",
            path: [
              { x: 126, y: 37 },
              { x: 286, y: 37 },
              { x: 447, y: 37 },
              { x: 447, y: 270 },
              { x: 447, y: 536 },
            ],
          },
          {
            label: "retrace the middle stem to the baseline",
            path: [
              { x: 447, y: 536 },
              { x: 447, y: 270 },
              { x: 447, y: 37 },
            ],
          },
          {
            label: "cross the second base join and rise through the right stem",
            path: [
              { x: 447, y: 37 },
              { x: 607, y: 37 },
              { x: 768, y: 37 },
              { x: 768, y: 270 },
              { x: 768, y: 536 },
            ],
          },
          {
            label: "retrace the right stem to the baseline",
            path: [
              { x: 768, y: 536 },
              { x: 768, y: 270 },
              { x: 768, y: 37 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ш"),
  },
  // RussianIrina writes lowercase щ like ш and continues from the right stem
  // directly into its looped tail. The fitted path squares the joins and keeps
  // the printed descender connected through the tail shoulder.
  [ductusKey("cyrillic", "щ")]: {
    script: "cyrillic",
    glyph: "щ",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 536 },
              { x: 129, y: 270 },
              { x: 129, y: 37 },
            ],
          },
          {
            label: "cross the first base join and rise through the middle stem",
            path: [
              { x: 129, y: 37 },
              { x: 287, y: 37 },
              { x: 445, y: 37 },
              { x: 445, y: 270 },
              { x: 445, y: 536 },
            ],
          },
          {
            label: "retrace the middle stem to the baseline",
            path: [
              { x: 445, y: 536 },
              { x: 445, y: 270 },
              { x: 445, y: 37 },
            ],
          },
          {
            label: "cross the second base join and rise through the right stem",
            path: [
              { x: 445, y: 37 },
              { x: 603, y: 37 },
              { x: 760, y: 37 },
              { x: 760, y: 270 },
              { x: 760, y: 536 },
            ],
          },
          {
            label: "retrace the right stem and cross the tail shoulder",
            path: [
              { x: 760, y: 536 },
              { x: 760, y: 270 },
              { x: 760, y: 37 },
              { x: 845, y: 37 },
            ],
          },
          {
            label: "descend the short tail below the baseline",
            path: [
              { x: 845, y: 37 },
              { x: 845, y: -50 },
              { x: 845, y: -140 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("щ"),
  },
  // RussianIrina writes lowercase ъ in one run: a narrow entry loop and top
  // shoulder flow into the descending stem, which turns directly through the
  // counterclockwise lower bowl. The fitted path squares the entry into the
  // printed top flag while preserving that flag-to-stem-to-bowl order.
  [ductusKey("cyrillic", "ъ")]: {
    script: "cyrillic",
    glyph: "ъ",
    strokes: [
      {
        segments: [
          {
            label: "sweep right along the broad top flag",
            path: [
              { x: 15, y: 499 },
              { x: 80, y: 499 },
              { x: 145, y: 499 },
              { x: 207, y: 499 },
            ],
          },
          {
            label: "descend the main stem to the baseline",
            path: [
              { x: 207, y: 499 },
              { x: 207, y: 300 },
              { x: 207, y: 150 },
              { x: 207, y: 36 },
            ],
          },
          {
            label: "sweep right along the lower bowl",
            path: [
              { x: 207, y: 36 },
              { x: 280, y: 36 },
              { x: 357, y: 36 },
              { x: 440, y: 45 },
              { x: 510, y: 85 },
            ],
          },
          {
            label: "curve upward around the bowl's right side",
            path: [
              { x: 510, y: 85 },
              { x: 533, y: 130 },
              { x: 533, y: 173 },
              { x: 515, y: 220 },
              { x: 470, y: 265 },
              { x: 410, y: 294 },
              { x: 357, y: 301 },
            ],
          },
          {
            label: "return left through the upper bowl to close against the stem",
            path: [
              { x: 357, y: 301 },
              { x: 305, y: 301 },
              { x: 251, y: 301 },
              { x: 207, y: 301 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ъ"),
  },
  // RussianIrina writes lowercase ы in two runs: the descending left stem
  // turns directly through a counterclockwise lower bowl, then a lifted right
  // stem descends into a curled exit. The fitted path keeps that body-first
  // order while straightening both stems and closing the printed bowl.
  [ductusKey("cyrillic", "ы")]: {
    script: "cyrillic",
    glyph: "ы",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 537 },
              { x: 129, y: 360 },
              { x: 129, y: 180 },
              { x: 129, y: 37 },
            ],
          },
          {
            label: "sweep right along the lower bowl",
            path: [
              { x: 129, y: 37 },
              { x: 200, y: 37 },
              { x: 276, y: 37 },
              { x: 350, y: 45 },
              { x: 425, y: 88 },
            ],
          },
          {
            label: "curve upward around the bowl's right side",
            path: [
              { x: 425, y: 88 },
              { x: 451, y: 130 },
              { x: 451, y: 176 },
              { x: 435, y: 220 },
              { x: 395, y: 263 },
              { x: 335, y: 297 },
              { x: 276, y: 304 },
            ],
          },
          {
            label: "return left through the upper bowl to close against the stem",
            path: [
              { x: 276, y: 304 },
              { x: 225, y: 304 },
              { x: 173, y: 304 },
              { x: 129, y: 304 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the separate right stem",
            path: [
              { x: 630, y: 537 },
              { x: 630, y: 360 },
              { x: 630, y: 180 },
              { x: 630, y: 37 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ы"),
  },
  // RussianIrina writes lowercase ь in one run: the descending stem turns
  // directly through a counterclockwise lower bowl and closes against itself.
  // The fitted path keeps that stem-first order while straightening the
  // upright and widening the printed bowl.
  [ductusKey("cyrillic", "ь")]: {
    script: "cyrillic",
    glyph: "ь",
    strokes: [
      {
        segments: [
          {
            label: "descend the stem to the baseline",
            path: [
              { x: 129, y: 536 },
              { x: 129, y: 360 },
              { x: 129, y: 180 },
              { x: 129, y: 36 },
            ],
          },
          {
            label: "sweep right along the lower bowl",
            path: [
              { x: 129, y: 36 },
              { x: 200, y: 36 },
              { x: 279, y: 36 },
              { x: 355, y: 44 },
              { x: 430, y: 87 },
            ],
          },
          {
            label: "curve upward around the bowl's right side",
            path: [
              { x: 430, y: 87 },
              { x: 456, y: 128 },
              { x: 456, y: 173 },
              { x: 440, y: 217 },
              { x: 400, y: 258 },
              { x: 340, y: 291 },
              { x: 279, y: 301 },
            ],
          },
          {
            label: "return left through the upper bowl to close against the stem",
            path: [
              { x: 279, y: 301 },
              { x: 225, y: 301 },
              { x: 173, y: 301 },
              { x: 129, y: 301 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ь"),
  },
  // RussianIrina writes lowercase э in two runs: the outer backwards-C curve
  // travels from upper left around the right side to lower left, then a lifted
  // tongue travels right-to-left. The fitted path widens the curve and
  // straightens the printed middle bar without changing that order.
  [ductusKey("cyrillic", "э")]: {
    script: "cyrillic",
    glyph: "э",
    strokes: [
      {
        segments: [
          {
            label: "sweep right across the upper curve",
            path: [
              { x: 82, y: 472 },
              { x: 150, y: 500 },
              { x: 230, y: 505 },
              { x: 315, y: 480 },
              { x: 378, y: 420 },
            ],
          },
          {
            label: "continue down around the outer right side",
            path: [
              { x: 378, y: 420 },
              { x: 420, y: 350 },
              { x: 425, y: 270 },
              { x: 415, y: 185 },
              { x: 378, y: 110 },
            ],
          },
          {
            label: "sweep left through the lower curve",
            path: [
              { x: 378, y: 110 },
              { x: 315, y: 45 },
              { x: 230, y: 25 },
              { x: 150, y: 35 },
              { x: 82, y: 72 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle tongue right-to-left",
            path: [
              { x: 356, y: 276 },
              { x: 290, y: 276 },
              { x: 225, y: 276 },
              { x: 160, y: 276 },
              { x: 95, y: 276 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("э"),
  },
  // RussianIrina writes lowercase ю in one run: the descending left stem
  // turns through a rising connector and continues clockwise around the oval.
  // The fitted path retraces to the printed middle bar while preserving that
  // zero-lift stem-to-connector-to-oval order.
  [ductusKey("cyrillic", "ю")]: {
    script: "cyrillic",
    glyph: "ю",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 536 },
              { x: 129, y: 360 },
              { x: 129, y: 180 },
              { x: 129, y: 0 },
            ],
          },
          {
            label: "retrace upward and sweep right along the middle bar",
            path: [
              { x: 129, y: 0 },
              { x: 129, y: 140 },
              { x: 129, y: 276 },
              { x: 210, y: 276 },
              { x: 322, y: 276 },
            ],
          },
          {
            label: "curve upward around the oval and across its top",
            path: [
              { x: 322, y: 276 },
              { x: 330, y: 390 },
              { x: 395, y: 475 },
              { x: 514, y: 510 },
              { x: 625, y: 475 },
              { x: 695, y: 390 },
            ],
          },
          {
            label: "continue down around the oval's right side",
            path: [
              { x: 695, y: 390 },
              { x: 715, y: 320 },
              { x: 715, y: 245 },
              { x: 695, y: 165 },
              { x: 650, y: 90 },
            ],
          },
          {
            label: "sweep left through the bottom and rise to close",
            path: [
              { x: 650, y: 90 },
              { x: 585, y: 30 },
              { x: 514, y: 27 },
              { x: 405, y: 55 },
              { x: 340, y: 145 },
              { x: 322, y: 276 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ю"),
  },
  // RussianIrina writes lowercase я in one run: rise from the baseline,
  // circle the upper loop counterclockwise, then descend the diagonal leg.
  // The printed fit uses its right upright as that rise and preserves the
  // source's zero-lift rise-to-loop-to-leg order.
  [ductusKey("cyrillic", "я")]: {
    script: "cyrillic",
    glyph: "я",
    strokes: [
      {
        segments: [
          {
            label: "climb the right stem from the baseline to the top",
            path: [
              { x: 449, y: 0 },
              { x: 449, y: 140 },
              { x: 449, y: 300 },
              { x: 449, y: 440 },
              { x: 449, y: 499 },
            ],
          },
          {
            label: "curve counterclockwise around the upper bowl",
            path: [
              { x: 449, y: 499 },
              { x: 360, y: 499 },
              { x: 265, y: 499 },
              { x: 175, y: 475 },
              { x: 115, y: 425 },
              { x: 105, y: 370 },
              { x: 120, y: 325 },
              { x: 180, y: 290 },
              { x: 280, y: 243 },
              { x: 405, y: 243 },
            ],
          },
          {
            label: "sweep left through the bowl's lower join",
            path: [
              { x: 405, y: 243 },
              { x: 340, y: 243 },
              { x: 277, y: 225 },
              { x: 187, y: 218 },
            ],
          },
          {
            label: "descend the diagonal leg to the lower-left tip",
            path: [
              { x: 187, y: 218 },
              { x: 155, y: 170 },
              { x: 120, y: 120 },
              { x: 85, y: 70 },
              { x: 35, y: 0 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("я"),
  },
  // t30apps animates Gujarati અ as a joined body first, then a separately
  // descending right stem. The fitted medians preserve that one-lift order
  // while following the broader joins and foot of the bundled Noto glyph.
  [ductusKey("gujarati", "અ")]: {
    script: "gujarati",
    glyph: "અ",
    strokes: [
      {
        segments: [
          {
            label: "sweep clockwise around the open left curve",
            path: [
              { x: 55, y: 550 },
              { x: 115, y: 570 },
              { x: 180, y: 565 },
              { x: 240, y: 535 },
              { x: 295, y: 480 },
              { x: 310, y: 420 },
              { x: 295, y: 360 },
              { x: 255, y: 310 },
              { x: 205, y: 280 },
              { x: 155, y: 275 },
              { x: 110, y: 300 },
              { x: 75, y: 300 },
            ],
          },
          {
            label: "continue through the lower body and rise into the middle shoulder",
            path: [
              { x: 75, y: 300 },
              { x: 115, y: 245 },
              { x: 165, y: 180 },
              { x: 230, y: 130 },
              { x: 310, y: 100 },
              { x: 390, y: 110 },
              { x: 455, y: 155 },
              { x: 500, y: 225 },
              { x: 526, y: 310 },
              { x: 526, y: 410 },
            ],
          },
          {
            label: "retrace down and sweep through the small right arch",
            path: [
              { x: 526, y: 410 },
              { x: 526, y: 340 },
              { x: 555, y: 285 },
              { x: 610, y: 265 },
              { x: 660, y: 275 },
              { x: 708, y: 315 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the right stem into its foot",
            path: [
              { x: 748, y: 570 },
              { x: 748, y: 450 },
              { x: 748, y: 320 },
              { x: 748, y: 190 },
              { x: 750, y: 110 },
              { x: 775, y: 60 },
              { x: 815, y: 35 },
              { x: 865, y: 35 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("અ"),
  },
  // t30apps builds Gujarati આ from the joined અ body, lifts for અ's right
  // stem, then lifts again for the added trailing ā stem. The fitted medians
  // retain that three-run order across the wider bundled Noto glyph.
  [ductusKey("gujarati", "આ")]: {
    script: "gujarati",
    glyph: "આ",
    strokes: [
      {
        segments: [
          {
            label: "sweep clockwise around the open left curve",
            path: [
              { x: 55, y: 550 },
              { x: 115, y: 570 },
              { x: 180, y: 565 },
              { x: 240, y: 535 },
              { x: 295, y: 480 },
              { x: 310, y: 420 },
              { x: 295, y: 360 },
              { x: 255, y: 310 },
              { x: 205, y: 280 },
              { x: 155, y: 275 },
              { x: 110, y: 300 },
              { x: 75, y: 300 },
            ],
          },
          {
            label: "continue through the lower body and rise into the middle shoulder",
            path: [
              { x: 75, y: 300 },
              { x: 115, y: 245 },
              { x: 165, y: 180 },
              { x: 230, y: 130 },
              { x: 310, y: 100 },
              { x: 390, y: 110 },
              { x: 455, y: 155 },
              { x: 500, y: 225 },
              { x: 526, y: 310 },
              { x: 526, y: 410 },
            ],
          },
          {
            label: "retrace down and sweep through the small right arch",
            path: [
              { x: 526, y: 410 },
              { x: 526, y: 340 },
              { x: 555, y: 285 },
              { x: 610, y: 265 },
              { x: 660, y: 275 },
              { x: 708, y: 315 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the first right stem into its foot",
            path: [
              { x: 748, y: 570 },
              { x: 748, y: 450 },
              { x: 748, y: 320 },
              { x: 748, y: 190 },
              { x: 750, y: 110 },
              { x: 775, y: 60 },
              { x: 815, y: 35 },
              { x: 865, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then descend the trailing ā stem into its foot",
            path: [
              { x: 1013, y: 570 },
              { x: 1013, y: 450 },
              { x: 1013, y: 320 },
              { x: 1013, y: 190 },
              { x: 1015, y: 110 },
              { x: 1040, y: 60 },
              { x: 1080, y: 35 },
              { x: 1130, y: 35 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("આ"),
  },
  // t30apps animates Gujarati ઇ as one unbroken run through the upper loop,
  // central crossing, broad lower loop, and rising hook. These fitted medians
  // preserve that zero-lift order inside the bundled Noto glyph's wider body.
  [ductusKey("gujarati", "ઇ")]: {
    script: "gujarati",
    glyph: "ઇ",
    strokes: [
      {
        segments: [
          {
            label: "circle the small upper-left loop down to the middle crossing",
            path: [
              { x: 220, y: 565 },
              { x: 165, y: 565 },
              { x: 115, y: 535 },
              { x: 85, y: 480 },
              { x: 85, y: 420 },
              { x: 120, y: 365 },
              { x: 170, y: 330 },
              { x: 220, y: 320 },
            ],
          },
          {
            label: "continue through the narrow crossing",
            path: [
              { x: 220, y: 320 },
              { x: 265, y: 322 },
              { x: 310, y: 322 },
            ],
          },
          {
            label: "sweep clockwise around the broad lower loop",
            path: [
              { x: 310, y: 322 },
              { x: 245, y: 285 },
              { x: 185, y: 245 },
              { x: 145, y: 190 },
              { x: 145, y: 130 },
              { x: 190, y: 75 },
              { x: 260, y: 40 },
              { x: 340, y: 30 },
              { x: 420, y: 50 },
              { x: 490, y: 100 },
              { x: 535, y: 170 },
              { x: 550, y: 245 },
            ],
          },
          {
            label: "rise along the right side into the upper hook",
            path: [
              { x: 550, y: 245 },
              { x: 540, y: 315 },
              { x: 505, y: 390 },
              { x: 465, y: 460 },
              { x: 440, y: 525 },
              { x: 445, y: 590 },
              { x: 475, y: 650 },
              { x: 515, y: 690 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઇ"),
  },
  // t30apps gives Gujarati ઈ the same unbroken loops as ઇ, then extends the
  // rising hook into a high clockwise curl. The fitted median preserves that
  // zero-lift order across the taller bundled Noto outline.
  [ductusKey("gujarati", "ઈ")]: {
    script: "gujarati",
    glyph: "ઈ",
    strokes: [
      {
        segments: [
          {
            label: "circle the small upper-left loop down to the middle crossing",
            path: [
              { x: 220, y: 565 },
              { x: 165, y: 565 },
              { x: 115, y: 535 },
              { x: 85, y: 480 },
              { x: 85, y: 420 },
              { x: 120, y: 365 },
              { x: 170, y: 330 },
              { x: 220, y: 320 },
            ],
          },
          {
            label: "continue through the narrow crossing",
            path: [
              { x: 220, y: 320 },
              { x: 265, y: 322 },
              { x: 310, y: 322 },
            ],
          },
          {
            label: "sweep clockwise around the broad lower loop",
            path: [
              { x: 310, y: 322 },
              { x: 245, y: 285 },
              { x: 185, y: 245 },
              { x: 145, y: 190 },
              { x: 145, y: 130 },
              { x: 190, y: 75 },
              { x: 260, y: 40 },
              { x: 340, y: 30 },
              { x: 420, y: 50 },
              { x: 490, y: 100 },
              { x: 535, y: 170 },
              { x: 550, y: 245 },
            ],
          },
          {
            label: "rise and curl clockwise around the extended top hook",
            path: [
              { x: 550, y: 245 },
              { x: 535, y: 330 },
              { x: 500, y: 420 },
              { x: 455, y: 510 },
              { x: 415, y: 600 },
              { x: 385, y: 690 },
              { x: 385, y: 760 },
              { x: 420, y: 825 },
              { x: 480, y: 860 },
              { x: 545, y: 855 },
              { x: 600, y: 820 },
              { x: 640, y: 765 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઈ"),
  },
  // t30apps animates Gujarati ઉ as one unbroken run through its small upper
  // bowl, middle cusp, broad lower bowl, and tall returning outer curve. This
  // fitted median preserves that zero-lift order inside the wider Noto outline.
  [ductusKey("gujarati", "ઉ")]: {
    script: "gujarati",
    glyph: "ઉ",
    strokes: [
      {
        segments: [
          {
            label: "circle clockwise through the small upper bowl to the middle cusp",
            path: [
              { x: 270, y: 550 },
              { x: 330, y: 565 },
              { x: 400, y: 565 },
              { x: 470, y: 540 },
              { x: 520, y: 500 },
              { x: 535, y: 450 },
              { x: 520, y: 400 },
              { x: 475, y: 365 },
              { x: 425, y: 335 },
              { x: 370, y: 315 },
              { x: 330, y: 305 },
            ],
          },
          {
            label: "continue right and sweep clockwise around the broad lower bowl",
            path: [
              { x: 330, y: 305 },
              { x: 390, y: 310 },
              { x: 445, y: 290 },
              { x: 495, y: 250 },
              { x: 525, y: 200 },
              { x: 525, y: 145 },
              { x: 490, y: 90 },
              { x: 435, y: 50 },
              { x: 365, y: 30 },
            ],
          },
          {
            label: "climb around the tall outer-left curve and finish at the upper right",
            path: [
              { x: 365, y: 30 },
              { x: 285, y: 35 },
              { x: 215, y: 75 },
              { x: 160, y: 140 },
              { x: 120, y: 225 },
              { x: 95, y: 325 },
              { x: 95, y: 430 },
              { x: 115, y: 535 },
              { x: 155, y: 635 },
              { x: 220, y: 720 },
              { x: 300, y: 775 },
              { x: 390, y: 795 },
              { x: 470, y: 785 },
              { x: 525, y: 755 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઉ"),
  },
  // t30apps repeats the complete zero-lift Gujarati ઉ run for ઊ, then carries
  // the same pen across the high shoulder and down the long right-side tail.
  // The fitted median keeps that extension inside the wider Noto outline.
  [ductusKey("gujarati", "ઊ")]: {
    script: "gujarati",
    glyph: "ઊ",
    strokes: [
      {
        segments: [
          {
            label: "write ઉ through its upper bowl, middle cusp, and lower bowl",
            path: [
              { x: 270, y: 550 },
              { x: 330, y: 565 },
              { x: 400, y: 565 },
              { x: 470, y: 540 },
              { x: 520, y: 500 },
              { x: 535, y: 450 },
              { x: 520, y: 400 },
              { x: 475, y: 365 },
              { x: 425, y: 335 },
              { x: 370, y: 315 },
              { x: 330, y: 305 },
              { x: 390, y: 310 },
              { x: 445, y: 290 },
              { x: 495, y: 250 },
              { x: 525, y: 200 },
              { x: 525, y: 145 },
              { x: 490, y: 90 },
              { x: 435, y: 50 },
              { x: 365, y: 30 },
            ],
          },
          {
            label: "continue around the tall outer-left curve",
            path: [
              { x: 365, y: 30 },
              { x: 285, y: 35 },
              { x: 215, y: 75 },
              { x: 160, y: 140 },
              { x: 120, y: 225 },
              { x: 95, y: 325 },
              { x: 95, y: 430 },
              { x: 115, y: 535 },
              { x: 155, y: 635 },
              { x: 220, y: 720 },
              { x: 300, y: 775 },
              { x: 390, y: 795 },
              { x: 470, y: 785 },
              { x: 525, y: 755 },
            ],
          },
          {
            label: "cross the high shoulder and descend the long right tail into its foot",
            path: [
              { x: 525, y: 755 },
              { x: 600, y: 725 },
              { x: 660, y: 670 },
              { x: 710, y: 600 },
              { x: 750, y: 520 },
              { x: 754, y: 400 },
              { x: 754, y: 280 },
              { x: 754, y: 160 },
              { x: 760, y: 90 },
              { x: 790, y: 45 },
              { x: 835, y: 35 },
              { x: 875, y: 35 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઊ"),
  },
  // t30apps writes Gujarati ઋ as a bent left body, lifts for the central
  // stem, then lifts again for the right loop and descending tail. These
  // medians retain that three-path order inside the bundled Noto outline.
  [ductusKey("gujarati", "ઋ")]: {
    script: "gujarati",
    glyph: "ઋ",
    strokes: [
      {
        segments: [
          {
            label: "sweep right along the upper body, then turn diagonally down-left",
            path: [
              { x: 35, y: 475 }, { x: 95, y: 495 }, { x: 160, y: 495 },
              { x: 220, y: 480 }, { x: 275, y: 450 }, { x: 325, y: 405 },
              { x: 375, y: 350 }, { x: 330, y: 310 }, { x: 275, y: 275 },
              { x: 220, y: 240 }, { x: 165, y: 205 }, { x: 115, y: 165 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the central stem into its foot",
            path: [
              { x: 447, y: 575 }, { x: 447, y: 460 }, { x: 447, y: 350 },
              { x: 447, y: 240 }, { x: 447, y: 140 }, { x: 460, y: 80 },
              { x: 500, y: 40 }, { x: 550, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, circle the right loop, and descend through the tail",
            path: [
              { x: 500, y: 385 }, { x: 555, y: 360 }, { x: 615, y: 350 },
              { x: 665, y: 365 }, { x: 710, y: 400 }, { x: 720, y: 445 },
              { x: 700, y: 465 }, { x: 675, y: 455 }, { x: 660, y: 425 },
              { x: 675, y: 390 }, { x: 720, y: 360 }, { x: 755, y: 320 },
              { x: 765, y: 270 }, { x: 750, y: 220 }, { x: 715, y: 175 },
              { x: 675, y: 145 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઋ"),
  },
  // t30apps writes Gujarati એ as a joined body, a separately descended right
  // stem, then a separate high arc. These fitted medians preserve that
  // three-path, two-lift order across the wider bundled Noto outline.
  [ductusKey("gujarati", "એ")]: {
    script: "gujarati",
    glyph: "એ",
    strokes: [
      {
        segments: [
          {
            label: "circle clockwise around the left bowl",
            path: [
              { x: 65, y: 560 },
              { x: 125, y: 580 },
              { x: 190, y: 570 },
              { x: 250, y: 535 },
              { x: 295, y: 480 },
              { x: 312, y: 420 },
              { x: 295, y: 360 },
              { x: 250, y: 315 },
              { x: 195, y: 280 },
              { x: 135, y: 270 },
              { x: 85, y: 285 },
              { x: 55, y: 310 },
              { x: 70, y: 325 },
              { x: 110, y: 315 },
              { x: 150, y: 285 },
            ],
          },
          {
            label: "continue through the lower body and small right arch",
            path: [
              { x: 150, y: 285 },
              { x: 180, y: 220 },
              { x: 225, y: 155 },
              { x: 285, y: 115 },
              { x: 350, y: 105 },
              { x: 415, y: 130 },
              { x: 470, y: 185 },
              { x: 505, y: 255 },
              { x: 515, y: 325 },
              { x: 495, y: 380 },
              { x: 520, y: 315 },
              { x: 570, y: 270 },
              { x: 625, y: 265 },
              { x: 680, y: 290 },
              { x: 710, y: 335 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the full-height right stem into its foot",
            path: [
              { x: 748, y: 590 },
              { x: 748, y: 470 },
              { x: 748, y: 350 },
              { x: 748, y: 230 },
              { x: 748, y: 130 },
              { x: 760, y: 75 },
              { x: 795, y: 40 },
              { x: 835, y: 35 },
              { x: 870, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again and sweep the high arcing mark from left to right",
            path: [
              { x: 515, y: 850 },
              { x: 565, y: 865 },
              { x: 615, y: 855 },
              { x: 660, y: 825 },
              { x: 695, y: 780 },
              { x: 720, y: 725 },
              { x: 742, y: 665 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("એ"),
  },
  // t30apps repeats Gujarati એ's body, right stem, and first high arc for ઐ,
  // then adds a fourth, higher arc. These fitted medians preserve that
  // four-path, three-lift order inside the stacked Noto marks.
  [ductusKey("gujarati", "ઐ")]: {
    script: "gujarati",
    glyph: "ઐ",
    strokes: [
      {
        segments: [
          {
            label: "write એ through its joined bowl, lower body, and right arch",
            path: [
              { x: 65, y: 560 }, { x: 125, y: 580 }, { x: 190, y: 570 },
              { x: 250, y: 535 }, { x: 295, y: 480 }, { x: 312, y: 420 },
              { x: 295, y: 360 }, { x: 250, y: 315 }, { x: 195, y: 280 },
              { x: 135, y: 270 }, { x: 85, y: 285 }, { x: 55, y: 310 },
              { x: 70, y: 325 }, { x: 110, y: 315 }, { x: 150, y: 285 },
              { x: 180, y: 220 }, { x: 225, y: 155 }, { x: 285, y: 115 },
              { x: 350, y: 105 }, { x: 415, y: 130 }, { x: 470, y: 185 },
              { x: 505, y: 255 }, { x: 515, y: 325 }, { x: 495, y: 380 },
              { x: 520, y: 315 }, { x: 570, y: 270 }, { x: 625, y: 265 },
              { x: 680, y: 290 }, { x: 710, y: 335 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the full-height right stem into its foot",
            path: [
              { x: 748, y: 590 }, { x: 748, y: 470 }, { x: 748, y: 350 },
              { x: 748, y: 230 }, { x: 748, y: 130 }, { x: 760, y: 75 },
              { x: 795, y: 40 }, { x: 835, y: 35 }, { x: 870, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again and sweep the lower high arc from left to right",
            path: [
              { x: 425, y: 735 }, { x: 475, y: 745 }, { x: 525, y: 735 },
              { x: 575, y: 715 }, { x: 625, y: 690 }, { x: 670, y: 665 },
              { x: 710, y: 655 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift once more and sweep the higher arc from left to right",
            path: [
              { x: 535, y: 850 }, { x: 580, y: 865 }, { x: 625, y: 855 },
              { x: 665, y: 825 }, { x: 695, y: 780 }, { x: 720, y: 725 },
              { x: 740, y: 665 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઐ"),
  },
  // t30apps writes Gujarati ઓ as the complete three-run આ sequence followed
  // by a separate high arc. These fitted medians preserve that four-path,
  // three-lift order across the wider bundled Noto outline.
  [ductusKey("gujarati", "ઓ")]: {
    script: "gujarati",
    glyph: "ઓ",
    strokes: [
      {
        segments: [
          {
            label: "write આ through its open left curve",
            path: [
              { x: 55, y: 550 }, { x: 115, y: 570 }, { x: 180, y: 565 },
              { x: 240, y: 535 }, { x: 295, y: 480 }, { x: 310, y: 420 },
              { x: 295, y: 360 }, { x: 255, y: 310 }, { x: 205, y: 280 },
              { x: 155, y: 275 }, { x: 110, y: 300 }, { x: 75, y: 300 },
            ],
          },
          {
            label: "continue through the lower body and middle shoulder",
            path: [
              { x: 75, y: 300 }, { x: 115, y: 245 }, { x: 165, y: 180 },
              { x: 230, y: 130 }, { x: 310, y: 100 }, { x: 390, y: 110 },
              { x: 455, y: 155 }, { x: 500, y: 225 }, { x: 526, y: 310 },
              { x: 526, y: 410 },
            ],
          },
          {
            label: "retrace down and sweep through the small right arch",
            path: [
              { x: 526, y: 410 }, { x: 526, y: 340 }, { x: 555, y: 285 },
              { x: 610, y: 265 }, { x: 660, y: 275 }, { x: 708, y: 315 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the first right stem into its foot",
            path: [
              { x: 748, y: 570 }, { x: 748, y: 450 }, { x: 748, y: 320 },
              { x: 748, y: 190 }, { x: 750, y: 110 }, { x: 775, y: 60 },
              { x: 815, y: 35 }, { x: 865, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then descend the trailing stem into its foot",
            path: [
              { x: 1013, y: 570 }, { x: 1013, y: 450 }, { x: 1013, y: 320 },
              { x: 1013, y: 190 }, { x: 1015, y: 110 }, { x: 1040, y: 60 },
              { x: 1080, y: 35 }, { x: 1130, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift once more and sweep the high arc from left to right",
            path: [
              { x: 785, y: 850 }, { x: 825, y: 870 }, { x: 865, y: 870 },
              { x: 905, y: 850 }, { x: 940, y: 810 }, { x: 970, y: 760 },
              { x: 995, y: 700 }, { x: 1015, y: 650 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઓ"),
  },
  // t30apps repeats Gujarati ઓ's body, stems, and first high arc for ઔ, then
  // adds a fifth, higher arc. These fitted medians preserve that five-path,
  // four-lift order inside the bundled Noto glyph's stacked marks.
  [ductusKey("gujarati", "ઔ")]: {
    script: "gujarati",
    glyph: "ઔ",
    strokes: [
      {
        segments: [
          {
            label: "write ઓ through its open left curve, lower body, and arch",
            path: [
              { x: 55, y: 550 }, { x: 115, y: 570 }, { x: 180, y: 565 },
              { x: 240, y: 535 }, { x: 295, y: 480 }, { x: 310, y: 420 },
              { x: 295, y: 360 }, { x: 255, y: 310 }, { x: 205, y: 280 },
              { x: 155, y: 275 }, { x: 110, y: 300 }, { x: 75, y: 300 },
              { x: 115, y: 245 }, { x: 165, y: 180 }, { x: 230, y: 130 },
              { x: 310, y: 100 }, { x: 390, y: 110 }, { x: 455, y: 155 },
              { x: 500, y: 225 }, { x: 526, y: 310 }, { x: 526, y: 410 },
              { x: 526, y: 340 }, { x: 555, y: 285 }, { x: 610, y: 265 },
              { x: 660, y: 275 }, { x: 708, y: 315 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the first right stem into its foot",
            path: [
              { x: 748, y: 570 }, { x: 748, y: 450 }, { x: 748, y: 320 },
              { x: 748, y: 190 }, { x: 750, y: 110 }, { x: 775, y: 60 },
              { x: 815, y: 35 }, { x: 865, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then descend the trailing stem into its foot",
            path: [
              { x: 1013, y: 570 }, { x: 1013, y: 450 }, { x: 1013, y: 320 },
              { x: 1013, y: 190 }, { x: 1015, y: 110 }, { x: 1040, y: 60 },
              { x: 1080, y: 35 }, { x: 1130, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift once more and sweep the lower high arc left to right",
            path: [
              { x: 700, y: 740 }, { x: 740, y: 748 }, { x: 780, y: 745 },
              { x: 820, y: 735 }, { x: 860, y: 715 }, { x: 900, y: 690 },
              { x: 940, y: 665 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again and sweep the higher arc from left to right",
            path: [
              { x: 800, y: 850 }, { x: 840, y: 870 }, { x: 880, y: 870 },
              { x: 920, y: 850 }, { x: 955, y: 810 }, { x: 985, y: 760 },
              { x: 1010, y: 700 }, { x: 1025, y: 650 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઔ"),
  },
  // t30apps writes Gujarati ક as a continuous upper-loop-to-lower-body run,
  // then lifts for a rising diagonal cross-stroke. These medians preserve the
  // two-path order while fitting the broader bundled Noto outline.
  [ductusKey("gujarati", "ક")]: {
    script: "gujarati",
    glyph: "ક",
    strokes: [
      {
        segments: [
          {
            label: "circle the upper loop and continue through the rounded lower body",
            path: [
              { x: 370, y: 555 }, { x: 320, y: 565 }, { x: 270, y: 565 },
              { x: 220, y: 555 }, { x: 180, y: 530 }, { x: 150, y: 495 },
              { x: 145, y: 455 }, { x: 160, y: 415 }, { x: 200, y: 380 },
              { x: 250, y: 350 }, { x: 305, y: 320 }, { x: 355, y: 285 },
              { x: 395, y: 240 }, { x: 415, y: 190 }, { x: 410, y: 140 },
              { x: 385, y: 95 }, { x: 340, y: 60 }, { x: 285, y: 40 },
              { x: 230, y: 40 }, { x: 180, y: 55 }, { x: 130, y: 80 },
              { x: 75, y: 115 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then sweep the diagonal cross-stroke lower-left to upper-right",
            path: [
              { x: 65, y: 225 }, { x: 125, y: 250 }, { x: 190, y: 275 },
              { x: 255, y: 300 }, { x: 320, y: 330 }, { x: 385, y: 360 },
              { x: 445, y: 390 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ક"),
  },
  // t30apps writes Gujarati ખ as one joined left-lobe-and-curl run, then
  // lifts for the full-height right spine and its lower foot. These medians
  // preserve the two-path order while fitting the bundled Noto outline.
  [ductusKey("gujarati", "ખ")]: {
    script: "gujarati",
    glyph: "ખ",
    strokes: [
      {
        segments: [
          {
            label: "descend through the left lobe and curl right through the middle",
            path: [
              { x: 45, y: 555 }, { x: 90, y: 550 }, { x: 125, y: 525 },
              { x: 135, y: 480 }, { x: 133, y: 425 }, { x: 133, y: 360 },
              { x: 135, y: 300 }, { x: 155, y: 245 }, { x: 200, y: 210 },
              { x: 255, y: 195 }, { x: 310, y: 205 }, { x: 350, y: 240 },
              { x: 375, y: 285 }, { x: 388, y: 335 }, { x: 395, y: 390 },
              { x: 420, y: 330 }, { x: 455, y: 300 }, { x: 495, y: 298 },
              { x: 540, y: 310 }, { x: 585, y: 340 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the right spine and turn through its lower foot",
            path: [
              { x: 610, y: 560 }, { x: 610, y: 500 }, { x: 610, y: 430 },
              { x: 610, y: 350 }, { x: 610, y: 270 }, { x: 610, y: 190 },
              { x: 612, y: 120 }, { x: 630, y: 75 }, { x: 670, y: 45 },
              { x: 710, y: 38 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ખ"),
  },
  // t30apps writes Gujarati ગ as one rounded left-body run, then lifts for
  // the full-height right spine and its lower foot. These medians preserve
  // the two-path order while fitting the bundled Noto outline.
  [ductusKey("gujarati", "ગ")]: {
    script: "gujarati",
    glyph: "ગ",
    strokes: [
      {
        segments: [
          {
            label: "circle the rounded body from upper left to lower left",
            path: [
              { x: 80, y: 555 }, { x: 130, y: 570 }, { x: 185, y: 570 },
              { x: 235, y: 555 }, { x: 275, y: 525 }, { x: 305, y: 485 },
              { x: 325, y: 435 }, { x: 335, y: 380 }, { x: 330, y: 330 },
              { x: 315, y: 285 }, { x: 285, y: 245 }, { x: 245, y: 220 },
              { x: 205, y: 210 }, { x: 165, y: 220 }, { x: 125, y: 240 },
              { x: 90, y: 270 }, { x: 60, y: 315 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the right spine and turn through its lower foot",
            path: [
              { x: 520, y: 560 }, { x: 520, y: 500 }, { x: 520, y: 430 },
              { x: 520, y: 350 }, { x: 520, y: 270 }, { x: 520, y: 190 },
              { x: 520, y: 120 }, { x: 540, y: 75 }, { x: 580, y: 45 },
              { x: 620, y: 38 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ગ"),
  },
  // t30apps writes Gujarati ઘ as one joined upper-lobe-to-lower-body run,
  // then lifts for the full-height right spine and its lower foot. These
  // medians preserve the two-path order while fitting the bundled Noto outline.
  [ductusKey("gujarati", "ઘ")]: {
    script: "gujarati",
    glyph: "ઘ",
    strokes: [
      {
        segments: [
          {
            label: "circle the upper lobe, turn through the middle, and round the lower body",
            path: [
              { x: 280, y: 560 }, { x: 220, y: 565 }, { x: 160, y: 555 },
              { x: 110, y: 530 }, { x: 80, y: 490 }, { x: 80, y: 450 },
              { x: 100, y: 415 }, { x: 140, y: 385 }, { x: 185, y: 370 },
              { x: 235, y: 370 }, { x: 285, y: 380 }, { x: 245, y: 375 },
              { x: 200, y: 365 }, { x: 160, y: 340 }, { x: 135, y: 305 },
              { x: 125, y: 265 }, { x: 135, y: 220 }, { x: 160, y: 180 },
              { x: 200, y: 150 }, { x: 250, y: 135 }, { x: 305, y: 140 },
              { x: 355, y: 160 }, { x: 400, y: 195 }, { x: 430, y: 240 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the right spine and turn through its lower foot",
            path: [
              { x: 485, y: 560 }, { x: 485, y: 500 }, { x: 485, y: 430 },
              { x: 485, y: 350 }, { x: 485, y: 270 }, { x: 485, y: 190 },
              { x: 485, y: 120 }, { x: 500, y: 75 }, { x: 535, y: 45 },
              { x: 580, y: 38 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઘ"),
  },
  // t30apps writes Gujarati ઙ as one long S-like body, then lifts for the
  // compact upper-right dot. These medians preserve the two-path order while
  // fitting the bundled Noto outline.
  [ductusKey("gujarati", "ઙ")]: {
    script: "gujarati",
    glyph: "ઙ",
    strokes: [
      {
        segments: [
          {
            label: "sweep from the upper right through the S-like body to the lower left",
            path: [
              { x: 375, y: 560 }, { x: 330, y: 565 }, { x: 280, y: 565 },
              { x: 235, y: 555 }, { x: 200, y: 535 }, { x: 175, y: 505 },
              { x: 155, y: 470 }, { x: 160, y: 435 }, { x: 180, y: 405 },
              { x: 215, y: 380 }, { x: 255, y: 355 }, { x: 300, y: 330 },
              { x: 345, y: 300 }, { x: 380, y: 265 }, { x: 405, y: 225 },
              { x: 415, y: 180 }, { x: 405, y: 135 }, { x: 380, y: 95 },
              { x: 340, y: 65 }, { x: 290, y: 45 }, { x: 240, y: 40 },
              { x: 190, y: 55 }, { x: 145, y: 80 }, { x: 105, y: 115 },
              { x: 65, y: 160 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then circle the separate upper-right dot",
            path: [
              { x: 399, y: 452 }, { x: 420, y: 444 }, { x: 430, y: 424 },
              { x: 420, y: 402 }, { x: 399, y: 392 }, { x: 378, y: 402 },
              { x: 370, y: 424 }, { x: 378, y: 444 }, { x: 399, y: 452 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ઙ"),
  },
  // t30apps writes Gujarati ચ as one joined upper-bowl-to-middle-loop-to-lower-
  // body run, then lifts for the full-height right spine and its lower foot.
  // These medians preserve the two-path order while fitting the Noto outline.
  [ductusKey("gujarati", "ચ")]: {
    script: "gujarati",
    glyph: "ચ",
    strokes: [
      {
        segments: [
          {
            label: "circle the upper bowl, turn through the middle loop, and round the lower body",
            path: [
              { x: 70, y: 550 }, { x: 120, y: 565 }, { x: 175, y: 565 },
              { x: 225, y: 550 }, { x: 265, y: 520 }, { x: 290, y: 480 },
              { x: 305, y: 435 }, { x: 300, y: 395 }, { x: 280, y: 360 },
              { x: 245, y: 330 }, { x: 205, y: 305 }, { x: 165, y: 290 },
              { x: 125, y: 285 }, { x: 90, y: 295 }, { x: 65, y: 285 },
              { x: 75, y: 265 }, { x: 100, y: 260 }, { x: 125, y: 275 },
              { x: 145, y: 240 }, { x: 180, y: 205 }, { x: 225, y: 175 },
              { x: 280, y: 155 }, { x: 335, y: 155 }, { x: 385, y: 170 },
              { x: 430, y: 200 }, { x: 470, y: 245 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the right spine and turn through its lower foot",
            path: [
              { x: 526, y: 560 }, { x: 526, y: 500 }, { x: 526, y: 430 },
              { x: 526, y: 350 }, { x: 526, y: 270 }, { x: 526, y: 190 },
              { x: 526, y: 120 }, { x: 545, y: 75 }, { x: 580, y: 45 },
              { x: 620, y: 38 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("ચ"),
  },
  // HebrewPod101's second handwritten Alef demonstration draws one descending
  // diagonal, lifts, then draws the opposing diagonal across it. This learner
  // path keeps those two pen-down runs while routing the crossing through the
  // branches of the vendored Noto Sans Hebrew block Alef.
  [ductusKey("hebrew", "א")]: {
    script: "hebrew",
    glyph: "א",
    strokes: [
      {
        segments: [
          {
            label: "draw the main diagonal down and right",
            path: [
              { x: 120, y: 560 },
              { x: 180, y: 480 },
              { x: 250, y: 400 },
              { x: 320, y: 310 },
              { x: 390, y: 220 },
              { x: 470, y: 100 },
              { x: 540, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend from the upper-right arm to the crossing",
            path: [
              { x: 540, y: 560 },
              { x: 535, y: 500 },
              { x: 525, y: 430 },
              { x: 500, y: 370 },
              { x: 470, y: 330 },
              { x: 425, y: 290 },
              { x: 385, y: 285 },
            ],
          },
          {
            label: "continue through the crossing and down the lower-left leg",
            path: [
              { x: 385, y: 285 },
              { x: 350, y: 315 },
              { x: 320, y: 340 },
              { x: 280, y: 370 },
              { x: 252, y: 370 },
              { x: 220, y: 350 },
              { x: 175, y: 300 },
              { x: 135, y: 220 },
              { x: 105, y: 120 },
              { x: 85, y: 30 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("א"),
  },
  // The same lesson's block-style Bet joins the top bar directly to the right
  // descent, then lifts once before drawing the baseline left-to-right. Its
  // later dagesh is an optional mark and is not part of base U+05D1 here.
  [ductusKey("hebrew", "ב")]: {
    script: "hebrew",
    glyph: "ב",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 90, y: 555 },
              { x: 170, y: 555 },
              { x: 260, y: 555 },
              { x: 330, y: 540 },
              { x: 390, y: 500 },
              { x: 415, y: 430 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 415, y: 430 },
              { x: 415, y: 330 },
              { x: 415, y: 220 },
              { x: 415, y: 100 },
              { x: 415, y: 40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the baseline from left to right",
            path: [
              { x: 50, y: 40 },
              { x: 150, y: 40 },
              { x: 250, y: 40 },
              { x: 350, y: 40 },
              { x: 450, y: 40 },
              { x: 520, y: 40 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ב"),
  },
  // The dedicated Gimel lesson's printed-form demonstration joins its short
  // top bar to the right stem and short lower-right leg. It then lifts once,
  // restarts at the lower junction, and draws the longer leg down-left. That
  // angular order follows Noto Sans Hebrew while the source note preserves the
  // lesson's visibly different rounded cursive alternative.
  [ductusKey("hebrew", "ג")]: {
    script: "hebrew",
    glyph: "ג",
    strokes: [
      {
        segments: [
          {
            label: "draw the short top bar from left to right",
            path: [
              { x: 105, y: 555 },
              { x: 145, y: 555 },
              { x: 185, y: 550 },
              { x: 220, y: 535 },
              { x: 245, y: 510 },
            ],
          },
          {
            label: "continue down the right stem without lifting",
            path: [
              { x: 245, y: 510 },
              { x: 260, y: 455 },
              { x: 263, y: 380 },
              { x: 263, y: 300 },
              { x: 263, y: 220 },
              { x: 265, y: 150 },
            ],
          },
          {
            label: "continue into the short lower-right leg",
            path: [
              { x: 265, y: 150 },
              { x: 275, y: 110 },
              { x: 286, y: 70 },
              { x: 300, y: 25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, restart at the lower junction, and draw the longer leg down-left",
            path: [
              { x: 235, y: 155 },
              { x: 215, y: 130 },
              { x: 185, y: 100 },
              { x: 150, y: 75 },
              { x: 110, y: 55 },
              { x: 70, y: 42 },
              { x: 38, y: 40 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ג"),
  },
  // The source's cursive Dalet is explicitly one curve: a broad left-to-right
  // arch curls through a small loop and continues into its tail. The learner
  // path preserves that zero-lift run while fitting it to Noto Sans Hebrew's
  // angular block top bar, sharp right heel, and downstroke.
  [ductusKey("hebrew", "ד")]: {
    script: "hebrew",
    glyph: "ד",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 150, y: 555 },
              { x: 240, y: 555 },
              { x: 330, y: 555 },
              { x: 420, y: 555 },
              { x: 480, y: 555 },
            ],
          },
          {
            label: "continue around the sharp right corner and down without lifting",
            path: [
              { x: 480, y: 555 },
              { x: 430, y: 540 },
              { x: 385, y: 510 },
              { x: 370, y: 460 },
              { x: 370, y: 370 },
              { x: 370, y: 270 },
              { x: 370, y: 170 },
              { x: 370, y: 70 },
              { x: 370, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ד"),
  },
  // The dedicated Hei lesson writes the printed body as a left-to-right top
  // bar that turns down the right side, then lifts once for the detached left
  // leg. This angular order follows Noto Sans Hebrew while the source note
  // preserves the lesson's rounded handwritten alternative.
  [ductusKey("hebrew", "ה")]: {
    script: "hebrew",
    glyph: "ה",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 150, y: 555 },
              { x: 240, y: 555 },
              { x: 330, y: 555 },
              { x: 410, y: 555 },
              { x: 480, y: 555 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 480, y: 555 },
              { x: 500, y: 530 },
              { x: 510, y: 480 },
              { x: 510, y: 380 },
              { x: 510, y: 270 },
              { x: 510, y: 160 },
              { x: 510, y: 50 },
              { x: 510, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the detached left leg from top to bottom",
            path: [
              { x: 115, y: 320 },
              { x: 115, y: 260 },
              { x: 115, y: 180 },
              { x: 115, y: 100 },
              { x: 115, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ה"),
  },
  // The dedicated Vav lesson draws the printed head left-to-right and turns
  // directly into the top-to-bottom stem. This two-movement learner path is
  // one continuous zero-lift stroke on the Noto Sans Hebrew outline.
  [ductusKey("hebrew", "ו")]: {
    script: "hebrew",
    glyph: "ו",
    strokes: [
      {
        segments: [
          {
            label: "draw the small head from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 120, y: 555 },
              { x: 175, y: 555 },
            ],
          },
          {
            label: "continue straight down without lifting",
            path: [
              { x: 175, y: 555 },
              { x: 175, y: 480 },
              { x: 175, y: 380 },
              { x: 175, y: 270 },
              { x: 175, y: 160 },
              { x: 175, y: 60 },
              { x: 175, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ו"),
  },
  // The lesson's rounded handwritten Zayin begins with a short rightward rise
  // and continues around the body without lifting. This path preserves that
  // order while following Noto Sans Hebrew's broader head and curved stem.
  [ductusKey("hebrew", "ז")]: {
    script: "hebrew",
    glyph: "ז",
    strokes: [
      {
        segments: [
          {
            label: "draw the short head from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 160, y: 555 },
              { x: 260, y: 555 },
            ],
          },
          {
            label: "continue down through the curved stem without lifting",
            path: [
              { x: 260, y: 555 },
              { x: 220, y: 520 },
              { x: 180, y: 475 },
              { x: 150, y: 425 },
              { x: 132, y: 360 },
              { x: 130, y: 285 },
              { x: 138, y: 205 },
              { x: 148, y: 125 },
              { x: 160, y: 55 },
              { x: 166, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ז"),
  },
  // The printed Heit demonstration joins its left-to-right top bar to the
  // right descent, then lifts once for the left leg. The source also preserves
  // the same order with rounded corners in handwriting.
  [ductusKey("hebrew", "ח")]: {
    script: "hebrew",
    glyph: "ח",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 75, y: 555 },
              { x: 170, y: 555 },
              { x: 280, y: 555 },
              { x: 390, y: 555 },
              { x: 480, y: 555 },
              { x: 540, y: 540 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 540, y: 540 },
              { x: 542, y: 480 },
              { x: 542, y: 380 },
              { x: 542, y: 270 },
              { x: 542, y: 160 },
              { x: 542, y: 55 },
              { x: 542, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the joined left leg from top to bottom",
            path: [
              { x: 142, y: 555 },
              { x: 142, y: 480 },
              { x: 142, y: 380 },
              { x: 142, y: 270 },
              { x: 142, y: 160 },
              { x: 142, y: 55 },
              { x: 142, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ח"),
  },
  // Printed Tet uses an L-shaped left-and-base stroke, then restarts at the
  // lower right and climbs before turning inward. The source's rounded
  // handwriting preserves that unusual bottom-up finish as one continuous run.
  [ductusKey("hebrew", "ט")]: {
    script: "hebrew",
    glyph: "ט",
    strokes: [
      {
        segments: [
          {
            label: "draw the left side from top to bottom",
            path: [
              { x: 103, y: 560 },
              { x: 103, y: 480 },
              { x: 103, y: 380 },
              { x: 103, y: 270 },
              { x: 125, y: 170 },
            ],
          },
          {
            label: "continue around the bottom from left to right without lifting",
            path: [
              { x: 125, y: 170 },
              { x: 160, y: 90 },
              { x: 235, y: 35 },
              { x: 315, y: 25 },
              { x: 400, y: 45 },
              { x: 470, y: 105 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, restart at the lower-right, and climb the right side",
            path: [
              { x: 470, y: 105 },
              { x: 515, y: 170 },
              { x: 537, y: 250 },
              { x: 537, y: 330 },
              { x: 530, y: 420 },
              { x: 500, y: 495 },
              { x: 455, y: 545 },
            ],
          },
          {
            label: "turn down-left into the inward hook without lifting",
            path: [
              { x: 455, y: 545 },
              { x: 410, y: 560 },
              { x: 365, y: 557 },
              { x: 330, y: 540 },
              { x: 315, y: 530 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ט"),
  },
  // Printed Yod is the same tiny comma-like idea as handwriting with a sharper
  // angle: the head runs left-to-right and turns directly down the short stem.
  [ductusKey("hebrew", "י")]: {
    script: "hebrew",
    glyph: "י",
    strokes: [
      {
        segments: [
          {
            label: "draw the small head from left to right",
            path: [
              { x: 60, y: 555 },
              { x: 120, y: 555 },
              { x: 180, y: 555 },
            ],
          },
          {
            label: "continue down through the short angled stem without lifting",
            path: [
              { x: 180, y: 555 },
              { x: 180, y: 480 },
              { x: 180, y: 390 },
              { x: 180, y: 300 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("י"),
  },
  // Printed Kaf sharpens the handwritten half-circle into one continuous
  // top-right-bottom run: across the top, around the right side, then left.
  [ductusKey("hebrew", "כ")]: {
    script: "hebrew",
    glyph: "כ",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 135, y: 555 },
              { x: 209, y: 555 },
            ],
          },
          {
            label: "continue down the rounded right side without lifting",
            path: [
              { x: 209, y: 555 },
              { x: 300, y: 530 },
              { x: 380, y: 470 },
              { x: 420, y: 385 },
              { x: 423, y: 294 },
              { x: 420, y: 205 },
              { x: 380, y: 120 },
              { x: 300, y: 58 },
              { x: 209, y: 38 },
            ],
          },
          {
            label: "turn left along the base without lifting",
            path: [
              { x: 209, y: 38 },
              { x: 135, y: 38 },
              { x: 60, y: 38 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("כ"),
  },
  // Printed Lamed is one angular run: down the tall left stroke, right across
  // the middle, then diagonally down-left. Handwriting rounds this into a loop.
  [ductusKey("hebrew", "ל")]: {
    script: "hebrew",
    glyph: "ל",
    strokes: [
      {
        segments: [
          {
            label: "draw the tall left stroke from top to bottom",
            path: [
              { x: 80, y: 730 },
              { x: 80, y: 660 },
              { x: 80, y: 590 },
              { x: 80, y: 555 },
            ],
          },
          {
            label: "continue right along the middle bar without lifting",
            path: [
              { x: 80, y: 555 },
              { x: 180, y: 555 },
              { x: 300, y: 555 },
              { x: 420, y: 555 },
            ],
          },
          {
            label: "turn diagonally down-left through the lower stroke without lifting",
            path: [
              { x: 420, y: 555 },
              { x: 400, y: 480 },
              { x: 370, y: 390 },
              { x: 340, y: 300 },
              { x: 310, y: 210 },
              { x: 280, y: 120 },
              { x: 250, y: 38 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ל"),
  },
  // Printed Mem starts with its detached angled left part. After one lift, the
  // angular right body climbs, descends, and returns left along the base.
  [ductusKey("hebrew", "מ")]: {
    script: "hebrew",
    glyph: "מ",
    strokes: [
      {
        segments: [
          {
            label: "draw the detached left part from its lower tip up to the corner",
            path: [
              { x: 92, y: 45 },
              { x: 115, y: 205 },
              { x: 145, y: 365 },
              { x: 140, y: 555 },
            ],
          },
          {
            label: "turn down-right through its short inner leg without lifting",
            path: [
              { x: 140, y: 555 },
              { x: 150, y: 520 },
              { x: 160, y: 485 },
              { x: 170, y: 450 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then climb diagonally right through the upper shoulder",
            path: [
              { x: 190, y: 440 },
              { x: 235, y: 515 },
              { x: 300, y: 560 },
              { x: 390, y: 560 },
            ],
          },
          {
            label: "turn down the right side without lifting",
            path: [
              { x: 390, y: 560 },
              { x: 470, y: 525 },
              { x: 530, y: 430 },
              { x: 565, y: 315 },
              { x: 550, y: 180 },
              { x: 500, y: 76 },
            ],
          },
          {
            label: "turn left along the base without lifting, stopping before the left part",
            path: [
              { x: 500, y: 76 },
              { x: 430, y: 48 },
              { x: 355, y: 38 },
              { x: 280, y: 38 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("מ"),
  },
  // Printed Nun joins its small head, right descent, and leftward base in one
  // run. The source's immediately adjacent cursive form rounds the same hook.
  [ductusKey("hebrew", "נ")]: {
    script: "hebrew",
    glyph: "נ",
    strokes: [
      {
        segments: [
          {
            label: "draw the short top head from left to right",
            path: [
              { x: 105, y: 555 },
              { x: 155, y: 555 },
              { x: 210, y: 540 },
              { x: 255, y: 500 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 255, y: 500 },
              { x: 260, y: 400 },
              { x: 260, y: 280 },
              { x: 260, y: 160 },
              { x: 240, y: 80 },
            ],
          },
          {
            label: "turn left along the base without lifting",
            path: [
              { x: 240, y: 80 },
              { x: 190, y: 55 },
              { x: 120, y: 40 },
              { x: 60, y: 40 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("נ"),
  },
  // Printed Samekh closes one continuous clockwise loop. The source's
  // immediately adjacent cursive form rounds the same zero-lift movement.
  [ductusKey("hebrew", "ס")]: {
    script: "hebrew",
    glyph: "ס",
    strokes: [
      {
        segments: [
          {
            label: "draw the flat top from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 170, y: 555 },
              { x: 275, y: 555 },
              { x: 365, y: 550 },
            ],
          },
          {
            label: "round down the right side without lifting",
            path: [
              { x: 365, y: 550 },
              { x: 455, y: 520 },
              { x: 525, y: 430 },
              { x: 550, y: 325 },
              { x: 535, y: 200 },
              { x: 465, y: 90 },
              { x: 365, y: 35 },
            ],
          },
          {
            label: "sweep left along the base without lifting",
            path: [
              { x: 365, y: 35 },
              { x: 285, y: 30 },
              { x: 205, y: 55 },
              { x: 145, y: 115 },
            ],
          },
          {
            label: "climb the left side and close the loop without lifting",
            path: [
              { x: 145, y: 115 },
              { x: 120, y: 210 },
              { x: 120, y: 315 },
              { x: 125, y: 410 },
              { x: 150, y: 490 },
              { x: 120, y: 535 },
              { x: 70, y: 555 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ס"),
  },
  // Printed Ayin descends its right branch into the base, sweeps left, then
  // turns back to climb the left branch in one uninterrupted run.
  [ductusKey("hebrew", "ע")]: {
    script: "hebrew",
    glyph: "ע",
    strokes: [
      {
        segments: [
          {
            label: "descend the right branch and curve left into the base",
            path: [
              { x: 500, y: 560 },
              { x: 495, y: 455 },
              { x: 475, y: 335 },
              { x: 440, y: 225 },
              { x: 390, y: 145 },
              { x: 330, y: 85 },
              { x: 250, y: 45 },
            ],
          },
          {
            label: "sweep left along the base without lifting",
            path: [
              { x: 250, y: 45 },
              { x: 190, y: 25 },
              { x: 125, y: 15 },
              { x: 70, y: 10 },
            ],
          },
          {
            label: "turn back and climb the left branch without lifting",
            path: [
              { x: 70, y: 10 },
              { x: 145, y: 35 },
              { x: 205, y: 80 },
              { x: 210, y: 165 },
              { x: 180, y: 285 },
              { x: 150, y: 410 },
              { x: 115, y: 560 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ע"),
  },
  // Printed Pe draws the outer top, right side, and returning base in one run,
  // then lifts once for its short inner curl. The adjacent cursive form instead
  // coils inward as one rounded spiral.
  [ductusKey("hebrew", "פ")]: {
    script: "hebrew",
    glyph: "פ",
    strokes: [
      {
        segments: [
          {
            label: "draw the outer top from left to right",
            path: [
              { x: 150, y: 560 },
              { x: 220, y: 570 },
              { x: 286, y: 565 },
              { x: 365, y: 535 },
              { x: 430, y: 475 },
            ],
          },
          {
            label: "turn down the right side without lifting",
            path: [
              { x: 430, y: 475 },
              { x: 475, y: 410 },
              { x: 505, y: 330 },
              { x: 505, y: 260 },
              { x: 480, y: 185 },
              { x: 435, y: 120 },
              { x: 370, y: 75 },
            ],
          },
          {
            label: "return left along the base without lifting",
            path: [
              { x: 370, y: 75 },
              { x: 300, y: 45 },
              { x: 220, y: 35 },
              { x: 140, y: 38 },
              { x: 70, y: 38 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the short inner curl from left to right",
            path: [
              { x: 95, y: 400 },
              { x: 98, y: 350 },
              { x: 120, y: 305 },
              { x: 160, y: 270 },
              { x: 205, y: 250 },
              { x: 252, y: 247 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("פ"),
  },
  // Printed Tsadi draws its long left diagonal into the returning base, then
  // lifts once for the short upper-right arm. Its cursive counterpart compresses
  // those branches into one compact rounded run.
  [ductusKey("hebrew", "צ")]: {
    script: "hebrew",
    glyph: "צ",
    strokes: [
      {
        segments: [
          {
            label: "descend the long diagonal from the upper left",
            path: [
              { x: 100, y: 560 },
              { x: 145, y: 505 },
              { x: 195, y: 430 },
              { x: 245, y: 350 },
              { x: 295, y: 270 },
              { x: 345, y: 185 },
              { x: 395, y: 100 },
              { x: 440, y: 40 },
            ],
          },
          {
            label: "turn left along the base without lifting",
            path: [
              { x: 440, y: 40 },
              { x: 350, y: 38 },
              { x: 250, y: 38 },
              { x: 150, y: 38 },
              { x: 55, y: 38 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then curve the upper-right arm down-left into the junction",
            path: [
              { x: 460, y: 560 },
              { x: 458, y: 505 },
              { x: 448, y: 450 },
              { x: 430, y: 390 },
              { x: 405, y: 335 },
              { x: 375, y: 285 },
              { x: 345, y: 260 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("צ"),
  },
  // Printed Qof keeps the top and slanted right body in one run, then lifts
  // once for the separate descending stem. Its cursive counterpart rounds the
  // same idea into one continuous hooked descent.
  [ductusKey("hebrew", "ק")]: {
    script: "hebrew",
    glyph: "ק",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 85, y: 555 },
              { x: 180, y: 555 },
              { x: 280, y: 555 },
              { x: 380, y: 555 },
              { x: 470, y: 555 },
              { x: 560, y: 555 },
            ],
          },
          {
            label: "turn down-left through the right body without lifting",
            path: [
              { x: 560, y: 555 },
              { x: 545, y: 520 },
              { x: 520, y: 460 },
              { x: 500, y: 400 },
              { x: 480, y: 335 },
              { x: 455, y: 260 },
              { x: 430, y: 180 },
              { x: 405, y: 100 },
              { x: 375, y: 10 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the separate inner-left stem below the line",
            path: [
              { x: 140, y: 360 },
              { x: 140, y: 275 },
              { x: 140, y: 180 },
              { x: 140, y: 80 },
              { x: 140, y: -20 },
              { x: 140, y: -105 },
              { x: 140, y: -180 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ק"),
  },
  // Printed Resh carries its short top bar directly around the rounded corner
  // and down the right side. The cursive form keeps the same zero-lift hook.
  [ductusKey("hebrew", "ר")]: {
    script: "hebrew",
    glyph: "ר",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 55, y: 555 },
              { x: 105, y: 555 },
              { x: 155, y: 555 },
              { x: 205, y: 555 },
              { x: 250, y: 555 },
            ],
          },
          {
            label: "round the top-right corner and continue down without lifting",
            path: [
              { x: 250, y: 555 },
              { x: 305, y: 550 },
              { x: 350, y: 530 },
              { x: 385, y: 495 },
              { x: 400, y: 445 },
              { x: 400, y: 350 },
              { x: 400, y: 250 },
              { x: 400, y: 140 },
              { x: 400, y: 10 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ר"),
  },
  // Printed Shin draws its outer right-base-left bowl in one run, then lifts
  // once for the middle branch. The adjacent purple cursive form compresses
  // those parts into a single rounded loop with a short rightward exit.
  [ductusKey("hebrew", "ש")]: {
    script: "hebrew",
    glyph: "ש",
    strokes: [
      {
        segments: [
          {
            label: "descend the right branch and round left along the base",
            path: [
              { x: 620, y: 570 },
              { x: 620, y: 500 },
              { x: 620, y: 420 },
              { x: 620, y: 340 },
              { x: 610, y: 250 },
              { x: 580, y: 170 },
              { x: 530, y: 100 },
              { x: 470, y: 60 },
              { x: 400, y: 35 },
              { x: 330, y: 32 },
              { x: 260, y: 45 },
              { x: 200, y: 80 },
              { x: 160, y: 135 },
            ],
          },
          {
            label: "continue up the left branch without lifting",
            path: [
              { x: 160, y: 135 },
              { x: 135, y: 200 },
              { x: 110, y: 280 },
              { x: 110, y: 380 },
              { x: 110, y: 480 },
              { x: 110, y: 570 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the middle branch into the base",
            path: [
              { x: 365, y: 570 },
              { x: 365, y: 500 },
              { x: 365, y: 430 },
              { x: 355, y: 365 },
              { x: 330, y: 320 },
              { x: 295, y: 285 },
              { x: 250, y: 260 },
              { x: 205, y: 250 },
              { x: 165, y: 250 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ש"),
  },
  // Printed Tav joins its top bar to the right side, then lifts once for the
  // separate left leg and foot. The purple cursive form instead retraces its
  // left stem and arches into the right side in one continuous run.
  [ductusKey("hebrew", "ת")]: {
    script: "hebrew",
    glyph: "ת",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 65, y: 555 },
              { x: 130, y: 555 },
              { x: 210, y: 555 },
              { x: 300, y: 555 },
              { x: 390, y: 555 },
              { x: 430, y: 550 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 430, y: 550 },
              { x: 490, y: 535 },
              { x: 535, y: 500 },
              { x: 560, y: 450 },
              { x: 565, y: 380 },
              { x: 565, y: 280 },
              { x: 565, y: 170 },
              { x: 565, y: 70 },
              { x: 565, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the separate left leg",
            path: [
              { x: 195, y: 520 },
              { x: 195, y: 450 },
              { x: 195, y: 360 },
              { x: 195, y: 270 },
              { x: 195, y: 180 },
              { x: 185, y: 120 },
            ],
          },
          {
            label: "curve left into the small foot without lifting",
            path: [
              { x: 185, y: 120 },
              { x: 165, y: 80 },
              { x: 135, y: 50 },
              { x: 100, y: 38 },
              { x: 70, y: 42 },
              { x: 50, y: 55 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ת"),
  },
  "ا": {
    script: "perso-arabic",
    glyph: "ا",
    strokes: [
      {
        segments: [
          {
            label: "down",
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
  [ductusKey("urdu-nastaliq", "ا")]: {
    script: "urdu-nastaliq",
    glyph: "ا",
    strokes: [
      {
        segments: [
          {
            label: "down",
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
    source: urduAlphabetSource("ا"),
  },
  [ductusKey("arabic", "ا")]: {
    script: "arabic",
    glyph: "ا",
    strokes: [
      {
        segments: [
          {
            label: "down",
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
    source: arabicAlphabetSource("ا"),
  },
  [ductusKey("arabic", "ب")]: {
    script: "arabic",
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
    source: arabicAlphabetSource("ب"),
  },
  [ductusKey("arabic", "ت")]: {
    script: "arabic",
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
    source: arabicAlphabetSource("ت"),
  },
  [ductusKey("arabic", "ج")]: {
    script: "arabic",
    glyph: "ج",
    strokes: [
      {
        segments: [
          {
            label: "draw the short upper head from left to right",
            path: [
              { x: 110, y: 315 },
              { x: 150, y: 335 },
              { x: 210, y: 340 },
              { x: 280, y: 325 },
              { x: 350, y: 305 },
              { x: 420, y: 285 },
              { x: 490, y: 270 },
              { x: 540, y: 270 },
            ],
          },
          {
            label: "continue down and around the bowl",
            path: [
              { x: 540, y: 270 },
              { x: 490, y: 270 },
              { x: 420, y: 285 },
              { x: 350, y: 305 },
              { x: 280, y: 325 },
              { x: 210, y: 340 },
              { x: 150, y: 335 },
              { x: 110, y: 315 },
              { x: 100, y: 290 },
              { x: 130, y: 305 },
              { x: 170, y: 310 },
              { x: 220, y: 305 },
              { x: 270, y: 285 },
              { x: 320, y: 265 },
              { x: 300, y: 245 },
              { x: 260, y: 220 },
              { x: 216, y: 190 },
              { x: 180, y: 130 },
              { x: 145, y: 65 },
              { x: 118, y: -42 },
              { x: 130, y: -110 },
              { x: 180, y: -175 },
              { x: 225, y: -200 },
              { x: 300, y: -245 },
              { x: 400, y: -245 },
              { x: 500, y: -230 },
              { x: 575, y: -210 },
              { x: 608, y: -195 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift once, then place the dot below",
            path: [
              { x: 415, y: -1 },
              { x: 374, y: 38 },
              { x: 330, y: -9 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ج"),
  },
  [ductusKey("arabic", "ح")]: {
    script: "arabic",
    glyph: "ح",
    strokes: [
      {
        segments: [
          {
            label: "draw the short left stem downward",
            path: [
              { x: 110, y: 315 },
              { x: 105, y: 302 },
              { x: 100, y: 290 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift once and restart near the stem's top",
            path: [
              { x: 110, y: 315 },
              { x: 150, y: 335 },
              { x: 210, y: 340 },
              { x: 280, y: 325 },
              { x: 350, y: 305 },
              { x: 420, y: 285 },
              { x: 490, y: 270 },
              { x: 540, y: 270 },
            ],
          },
          {
            label: "continue down and around the bowl",
            path: [
              { x: 540, y: 270 },
              { x: 490, y: 270 },
              { x: 420, y: 285 },
              { x: 350, y: 305 },
              { x: 280, y: 325 },
              { x: 210, y: 340 },
              { x: 150, y: 335 },
              { x: 110, y: 315 },
              { x: 100, y: 290 },
              { x: 130, y: 305 },
              { x: 170, y: 310 },
              { x: 220, y: 305 },
              { x: 270, y: 285 },
              { x: 320, y: 265 },
              { x: 300, y: 245 },
              { x: 260, y: 220 },
              { x: 216, y: 190 },
              { x: 180, y: 130 },
              { x: 145, y: 65 },
              { x: 118, y: -42 },
              { x: 130, y: -110 },
              { x: 180, y: -175 },
              { x: 225, y: -200 },
              { x: 300, y: -245 },
              { x: 400, y: -245 },
              { x: 500, y: -230 },
              { x: 575, y: -210 },
              { x: 608, y: -195 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ح"),
  },
  [ductusKey("arabic", "خ")]: {
    script: "arabic",
    glyph: "خ",
    strokes: [
      {
        segments: [
          {
            label: "draw the short upper head from left to right",
            path: [
              { x: 110, y: 315 },
              { x: 150, y: 335 },
              { x: 210, y: 340 },
              { x: 280, y: 325 },
              { x: 350, y: 305 },
              { x: 420, y: 285 },
              { x: 490, y: 270 },
              { x: 540, y: 270 },
            ],
          },
          {
            label: "continue down and around the bowl",
            path: [
              { x: 540, y: 270 },
              { x: 490, y: 270 },
              { x: 420, y: 285 },
              { x: 350, y: 305 },
              { x: 280, y: 325 },
              { x: 210, y: 340 },
              { x: 150, y: 335 },
              { x: 110, y: 315 },
              { x: 100, y: 290 },
              { x: 130, y: 305 },
              { x: 170, y: 310 },
              { x: 220, y: 305 },
              { x: 270, y: 285 },
              { x: 320, y: 265 },
              { x: 300, y: 245 },
              { x: 260, y: 220 },
              { x: 216, y: 190 },
              { x: 180, y: 130 },
              { x: 145, y: 65 },
              { x: 118, y: -42 },
              { x: 130, y: -110 },
              { x: 180, y: -175 },
              { x: 225, y: -200 },
              { x: 300, y: -245 },
              { x: 400, y: -245 },
              { x: 500, y: -230 },
              { x: 575, y: -210 },
              { x: 608, y: -195 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift once, then place the dot above",
            path: [
              { x: 340, y: 460 },
              { x: 285, y: 510 },
              { x: 338, y: 565 },
              { x: 390, y: 515 },
              { x: 340, y: 460 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("خ"),
  },
  [ductusKey("arabic", "د")]: {
    script: "arabic",
    glyph: "د",
    strokes: [
      {
        segments: [
          {
            label: "begin at the upper tip and descend diagonally down and right through the curved shoulder",
            path: [
              { x: 270, y: 350 },
              { x: 260, y: 325 },
              { x: 260, y: 300 },
              { x: 270, y: 275 },
              { x: 285, y: 245 },
              { x: 300, y: 215 },
              { x: 318, y: 185 },
              { x: 333, y: 155 },
              { x: 343, y: 130 },
              { x: 345, y: 110 },
              { x: 342, y: 100 },
            ],
          },
          {
            label: "turn left along the baseline without lifting",
            path: [
              { x: 342, y: 100 },
              { x: 320, y: 90 },
              { x: 290, y: 75 },
              { x: 250, y: 60 },
              { x: 210, y: 50 },
              { x: 170, y: 40 },
              { x: 130, y: 40 },
              { x: 90, y: 50 },
              { x: 60, y: 65 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("د"),
  },
  [ductusKey("arabic", "ر")]: {
    script: "arabic",
    glyph: "ر",
    strokes: [
      {
        segments: [
          {
            label: "begin at the upper tip and descend through the short stroke",
            path: [
              { x: 250, y: 320 },
              { x: 248, y: 280 },
              { x: 255, y: 235 },
              { x: 270, y: 190 },
              { x: 287, y: 145 },
              { x: 300, y: 95 },
              { x: 304, y: 48 },
            ],
          },
          {
            label: "sweep left through the lower curve without lifting",
            path: [
              { x: 304, y: 48 },
              { x: 298, y: 8 },
              { x: 284, y: -30 },
              { x: 260, y: -68 },
              { x: 226, y: -103 },
              { x: 185, y: -130 },
              { x: 140, y: -146 },
              { x: 95, y: -151 },
              { x: 52, y: -147 },
              { x: 10, y: -136 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ر"),
  },
  [ductusKey("arabic", "س")]: {
    script: "arabic",
    glyph: "س",
    strokes: [
      {
        segments: [
          {
            label: "form the three close teeth from right to left",
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
            label: "flow directly into the final bowl without lifting",
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
    source: arabicAlphabetSource("س"),
  },
  [ductusKey("arabic", "ش")]: {
    script: "arabic",
    glyph: "ش",
    strokes: [
      {
        segments: [
          {
            label: "shape the three close teeth from right to left",
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
            label: "flow directly into the final bowl without lifting",
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
      {
        segments: [
          {
            label: "lift, then place the lower-left dot",
            path: [
              { x: 610, y: 360 },
              { x: 648, y: 410 },
              { x: 686, y: 365 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then place the lower-right dot",
            path: [
              { x: 753, y: 370 },
              { x: 792, y: 423 },
              { x: 830, y: 376 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift a third time, then place the centered upper dot",
            path: [
              { x: 684, y: 446 },
              { x: 720, y: 494 },
              { x: 757, y: 446 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ش"),
  },
  [ductusKey("arabic", "ص")]: {
    script: "arabic",
    glyph: "ص",
    strokes: [
      {
        segments: [
          {
            label: "close the oval clockwise from its lower-left junction",
            path: [
              { x: 535, y: 30 },
              { x: 560, y: 90 },
              { x: 620, y: 160 },
              { x: 700, y: 230 },
              { x: 790, y: 305 },
              { x: 870, y: 320 },
              { x: 950, y: 285 },
              { x: 1010, y: 230 },
              { x: 1015, y: 175 },
              { x: 970, y: 115 },
              { x: 900, y: 70 },
              { x: 810, y: 45 },
              { x: 720, y: 38 },
              { x: 630, y: 42 },
              { x: 535, y: 30 },
            ],
          },
          {
            label: "turn left and rise into the short shoulder without lifting",
            path: [
              { x: 535, y: 30 },
              { x: 530, y: 65 },
              { x: 520, y: 105 },
              { x: 510, y: 145 },
              { x: 495, y: 190 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, restart at the baseline junction, and sweep through the trailing bowl",
            path: [
              { x: 500, y: -54 },
              { x: 475, y: -115 },
              { x: 425, y: -175 },
              { x: 360, y: -215 },
              { x: 280, y: -232 },
              { x: 205, y: -225 },
              { x: 145, y: -185 },
              { x: 105, y: -125 },
              { x: 92, y: -65 },
              { x: 100, y: 20 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ص"),
  },
  [ductusKey("arabic", "ض")]: {
    script: "arabic",
    glyph: "ض",
    strokes: [
      {
        segments: [
          {
            label: "close the oval clockwise from its lower-left junction",
            path: [
              { x: 535, y: 30 },
              { x: 560, y: 90 },
              { x: 620, y: 160 },
              { x: 700, y: 230 },
              { x: 790, y: 305 },
              { x: 870, y: 320 },
              { x: 950, y: 285 },
              { x: 1010, y: 230 },
              { x: 1015, y: 175 },
              { x: 970, y: 115 },
              { x: 900, y: 70 },
              { x: 810, y: 45 },
              { x: 720, y: 38 },
              { x: 630, y: 42 },
              { x: 535, y: 30 },
            ],
          },
          {
            label: "turn left and rise into the short shoulder without lifting",
            path: [
              { x: 535, y: 30 },
              { x: 530, y: 65 },
              { x: 520, y: 105 },
              { x: 510, y: 145 },
              { x: 495, y: 190 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, restart at the baseline junction, and sweep through the trailing bowl",
            path: [
              { x: 500, y: -54 },
              { x: 475, y: -115 },
              { x: 425, y: -175 },
              { x: 360, y: -215 },
              { x: 280, y: -232 },
              { x: 205, y: -225 },
              { x: 145, y: -185 },
              { x: 105, y: -125 },
              { x: 92, y: -65 },
              { x: 100, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then place the upper dot last",
            path: [
              { x: 725, y: 470 },
              { x: 675, y: 515 },
              { x: 725, y: 568 },
              { x: 770, y: 520 },
              { x: 725, y: 470 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ض"),
  },
  [ductusKey("arabic", "ع")]: {
    script: "arabic",
    glyph: "ع",
    strokes: [
      {
        segments: [
          {
            label: "sweep left from the upper-right tip and shape the open head",
            path: [
              { x: 355, y: 400 },
              { x: 315, y: 420 },
              { x: 255, y: 430 },
              { x: 195, y: 415 },
              { x: 145, y: 375 },
              { x: 110, y: 320 },
              { x: 105, y: 270 },
              { x: 135, y: 235 },
              { x: 145, y: 205 },
              { x: 185, y: 175 },
              { x: 250, y: 165 },
              { x: 325, y: 175 },
              { x: 395, y: 205 },
              { x: 450, y: 235 },
              { x: 410, y: 205 },
              { x: 350, y: 175 },
              { x: 285, y: 145 },
              { x: 230, y: 110 },
              { x: 190, y: 75 },
              { x: 175, y: 50 },
            ],
          },
          {
            label: "continue down and around the lower bowl without lifting",
            path: [
              { x: 175, y: 50 },
              { x: 150, y: -5 },
              { x: 135, y: -70 },
              { x: 145, y: -135 },
              { x: 185, y: -195 },
              { x: 245, y: -235 },
              { x: 320, y: -250 },
              { x: 400, y: -245 },
              { x: 480, y: -230 },
              { x: 555, y: -205 },
              { x: 610, y: -180 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ع"),
  },
  [ductusKey("arabic", "ك")]: {
    script: "arabic",
    glyph: "ك",
    strokes: [
      {
        segments: [
          {
            label: "descend the main upright",
            path: [
              { x: 430, y: 630 },
              { x: 435, y: 550 },
              { x: 440, y: 450 },
              { x: 450, y: 350 },
              { x: 465, y: 250 },
              { x: 475, y: 150 },
              { x: 470, y: 80 },
            ],
          },
          {
            label: "turn left along the baseline without lifting",
            path: [
              { x: 470, y: 80 },
              { x: 410, y: 52 },
              { x: 320, y: 40 },
              { x: 220, y: 38 },
              { x: 120, y: 42 },
              { x: 45, y: 58 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the inner arm from upper right down-left",
            path: [
              { x: 255, y: 385 },
              { x: 235, y: 375 },
              { x: 215, y: 360 },
              { x: 195, y: 340 },
              { x: 185, y: 320 },
              { x: 185, y: 305 },
              { x: 215, y: 295 },
              { x: 245, y: 292 },
              { x: 275, y: 285 },
              { x: 282, y: 273 },
              { x: 270, y: 258 },
              { x: 250, y: 240 },
              { x: 225, y: 222 },
              { x: 180, y: 207 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ك"),
  },
  [ductusKey("arabic", "ل")]: {
    script: "arabic",
    glyph: "ل",
    strokes: [
      {
        segments: [
          {
            label: "descend the tall upright",
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
            label: "continue left through the base bowl without lifting",
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
    source: arabicAlphabetSource("ل"),
  },
  [ductusKey("arabic", "ه")]: {
    script: "arabic",
    glyph: "ه",
    strokes: [
      {
        segments: [
          {
            label: "curve down-left and close the lower counter",
            path: [
              { x: 315, y: 400 },
              { x: 285, y: 375 },
              { x: 255, y: 350 },
              { x: 230, y: 325 },
              { x: 205, y: 300 },
              { x: 190, y: 260 },
              { x: 190, y: 210 },
              { x: 205, y: 165 },
              { x: 235, y: 125 },
              { x: 275, y: 105 },
              { x: 320, y: 110 },
              { x: 355, y: 135 },
              { x: 380, y: 175 },
              { x: 390, y: 225 },
              { x: 380, y: 275 },
              { x: 355, y: 320 },
              { x: 315, y: 355 },
            ],
          },
          {
            label: "thread through the centre and close the upper-right counter without lifting",
            path: [
              { x: 315, y: 355 },
              { x: 360, y: 355 },
              { x: 410, y: 340 },
              { x: 455, y: 315 },
              { x: 500, y: 275 },
              { x: 535, y: 225 },
              { x: 555, y: 170 },
              { x: 555, y: 115 },
              { x: 535, y: 70 },
              { x: 535, y: 50 },
              { x: 500, y: 40 },
              { x: 455, y: 30 },
              { x: 415, y: 45 },
              { x: 385, y: 75 },
              { x: 365, y: 100 },
            ],
          },
          {
            label: "sweep left along the baseline without lifting",
            path: [
              { x: 365, y: 100 },
              { x: 345, y: 75 },
              { x: 310, y: 65 },
              { x: 270, y: 65 },
              { x: 225, y: 70 },
              { x: 175, y: 65 },
              { x: 120, y: 65 },
              { x: 70, y: 65 },
              { x: 25, y: 65 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ه"),
  },
  [ductusKey("arabic", "و")]: {
    script: "arabic",
    glyph: "و",
    strokes: [
      {
        segments: [
          {
            label: "sweep left from the lower-right junction and close the small head loop",
            path: [
              { x: 390, y: 60 },
              { x: 340, y: 45 },
              { x: 285, y: 40 },
              { x: 225, y: 45 },
              { x: 175, y: 80 },
              { x: 145, y: 125 },
              { x: 145, y: 165 },
              { x: 170, y: 215 },
              { x: 210, y: 260 },
              { x: 250, y: 285 },
              { x: 300, y: 285 },
              { x: 345, y: 245 },
              { x: 375, y: 185 },
              { x: 390, y: 115 },
              { x: 390, y: 60 },
            ],
          },
          {
            label: "continue down and left through the tail without lifting",
            path: [
              { x: 390, y: 60 },
              { x: 370, y: -5 },
              { x: 340, y: -70 },
              { x: 300, y: -120 },
              { x: 250, y: -160 },
              { x: 195, y: -170 },
              { x: 135, y: -160 },
              { x: 80, y: -140 },
              { x: 45, y: -120 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("و"),
  },
  // Arabic ي (U+064A) shares the isolated bowl skeleton with Urdu ی (U+06CC),
  // but keeps its own source and adds the two lower dots observed in yaa.mov.
  [ductusKey("arabic", "ي")]: {
    script: "arabic",
    glyph: "ي",
    strokes: [
      {
        segments: [
          {
            label: "descend from the upper right into the independent bowl",
            path: [
              { x: 548, y: 285 },
              { x: 510, y: 288 },
              { x: 472, y: 270 },
              { x: 430, y: 238 },
              { x: 395, y: 205 },
              { x: 365, y: 168 },
              { x: 345, y: 125 },
              { x: 330, y: 82 },
              { x: 340, y: 45 },
              { x: 375, y: 25 },
              { x: 420, y: 8 },
              { x: 470, y: -2 },
              { x: 520, y: -24 },
              { x: 555, y: -55 },
            ],
          },
          {
            label: "sweep left through the bowl without lifting",
            path: [
              { x: 555, y: -55 },
              { x: 535, y: -98 },
              { x: 495, y: -145 },
              { x: 445, y: -188 },
              { x: 390, y: -218 },
              { x: 325, y: -238 },
              { x: 255, y: -238 },
              { x: 190, y: -218 },
              { x: 140, y: -180 },
              { x: 105, y: -130 },
              { x: 90, y: -78 },
              { x: 94, y: -25 },
              { x: 105, y: 28 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then place the lower-left dot",
            path: [
              { x: 150, y: -373 },
              { x: 198, y: -323 },
              { x: 245, y: -370 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then place the lower-right dot",
            path: [
              { x: 300, y: -360 },
              { x: 352, y: -310 },
              { x: 400, y: -356 },
            ],
          },
        ],
      },
    ],
    source: arabicAlphabetSource("ي"),
  },
  [ductusKey("urdu-nastaliq", "ج")]: {
    script: "urdu-nastaliq",
    glyph: "ج",
    strokes: [
      {
        segments: [
          {
            label: "place the dot below",
            path: [
              { x: 415, y: -1 },
              { x: 374, y: 38 },
              { x: 330, y: -9 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then sweep left through the pointed hooked head",
            path: [
              { x: 540, y: 270 },
              { x: 490, y: 270 },
              { x: 420, y: 285 },
              { x: 350, y: 305 },
              { x: 280, y: 325 },
              { x: 210, y: 340 },
              { x: 150, y: 335 },
              { x: 110, y: 315 },
              { x: 100, y: 290 },
              { x: 130, y: 305 },
              { x: 170, y: 310 },
              { x: 220, y: 305 },
              { x: 270, y: 285 },
              { x: 320, y: 265 },
              { x: 300, y: 245 },
              { x: 260, y: 220 },
              { x: 216, y: 190 },
            ],
          },
          {
            label: "continue down and around the bowl",
            path: [
              { x: 216, y: 190 },
              { x: 180, y: 130 },
              { x: 145, y: 65 },
              { x: 118, y: -42 },
              { x: 130, y: -110 },
              { x: 180, y: -175 },
              { x: 225, y: -200 },
              { x: 300, y: -245 },
              { x: 400, y: -245 },
              { x: 500, y: -230 },
              { x: 575, y: -210 },
              { x: 608, y: -195 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ج"),
  },
  [ductusKey("urdu-nastaliq", "ر")]: {
    script: "urdu-nastaliq",
    glyph: "ر",
    strokes: [
      {
        segments: [
          {
            label: "draw the downward line",
            path: [
              { x: 250, y: 320 },
              { x: 248, y: 280 },
              { x: 255, y: 235 },
              { x: 270, y: 190 },
              { x: 287, y: 145 },
              { x: 300, y: 95 },
              { x: 304, y: 48 },
            ],
          },
          {
            label: "continue curving to the left",
            path: [
              { x: 304, y: 48 },
              { x: 298, y: 8 },
              { x: 284, y: -30 },
              { x: 260, y: -68 },
              { x: 226, y: -103 },
              { x: 185, y: -130 },
              { x: 140, y: -146 },
              { x: 95, y: -151 },
              { x: 52, y: -147 },
              { x: 10, y: -136 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ر"),
  },
  [ductusKey("urdu-nastaliq", "س")]: {
    script: "urdu-nastaliq",
    glyph: "س",
    strokes: [
      {
        segments: [
          {
            label: "shape the three close teeth from right to left",
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
            label: "flow directly into the final bowl without lifting",
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
    source: urduAlphabetSource("س"),
  },
  [ductusKey("urdu-nastaliq", "ش")]: {
    script: "urdu-nastaliq",
    glyph: "ش",
    strokes: [
      {
        segments: [
          {
            label: "shape the three close teeth from right to left",
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
            label: "flow directly into the final bowl without lifting",
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
      {
        segments: [
          {
            label: "lift, then place the lower-left dot",
            path: [
              { x: 610, y: 360 },
              { x: 648, y: 410 },
              { x: 686, y: 365 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again, then place the lower-right dot",
            path: [
              { x: 753, y: 370 },
              { x: 792, y: 423 },
              { x: 830, y: 376 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift a third time, then place the centered upper dot",
            path: [
              { x: 684, y: 446 },
              { x: 720, y: 494 },
              { x: 757, y: 446 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ش"),
  },
  [ductusKey("urdu-nastaliq", "ک")]: {
    script: "urdu-nastaliq",
    glyph: "ک",
    strokes: [
      {
        segments: [
          {
            label: "draw the independent stem downward",
            path: [
              { x: 620, y: 250 },
              { x: 622, y: 200 },
              { x: 620, y: 150 },
            ],
          },
          {
            label: "flow right to left through the flatter bowl and finish with the hook without lifting",
            path: [
              { x: 620, y: 150 },
              { x: 570, y: 100 },
              { x: 500, y: 65 },
              { x: 400, y: 40 },
              { x: 300, y: 35 },
              { x: 210, y: 50 },
              { x: 140, y: 85 },
              { x: 95, y: 125 },
              { x: 95, y: 185 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the long slash down from the upper right toward the stem",
            path: [
              { x: 680, y: 625 },
              { x: 600, y: 590 },
              { x: 520, y: 550 },
              { x: 440, y: 510 },
              { x: 365, y: 470 },
              { x: 335, y: 425 },
              { x: 355, y: 400 },
              { x: 390, y: 380 },
              { x: 425, y: 360 },
              { x: 460, y: 340 },
              { x: 480, y: 320 },
              { x: 520, y: 300 },
              { x: 540, y: 280 },
              { x: 560, y: 260 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ک"),
  },
  [ductusKey("urdu-nastaliq", "ل")]: {
    script: "urdu-nastaliq",
    glyph: "ل",
    strokes: [
      {
        segments: [
          {
            label: "draw the tall independent upright downward",
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
            label: "continue below the baseline through the leftward bowl and back up without lifting",
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
    source: urduAlphabetSource("ل"),
  },
  [ductusKey("urdu-nastaliq", "م")]: {
    script: "urdu-nastaliq",
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
            label: "continue down the tail below the baseline without lifting",
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
    source: urduAlphabetSource("م"),
  },
  [ductusKey("urdu-nastaliq", "ن")]: {
    script: "urdu-nastaliq",
    glyph: "ن",
    strokes: [
      {
        segments: [
          {
            label: "sweep the independent bowl right to left below the baseline",
            path: [
              { x: 495, y: 210 },
              { x: 475, y: 160 },
              { x: 480, y: 100 },
              { x: 500, y: 40 },
              { x: 510, y: -20 },
              { x: 485, y: -80 },
              { x: 430, y: -140 },
              { x: 360, y: -190 },
              { x: 280, y: -220 },
              { x: 210, y: -215 },
              { x: 150, y: -170 },
              { x: 105, y: -110 },
              { x: 90, y: -60 },
              { x: 95, y: 0 },
              { x: 105, y: 45 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then place the dot near the baseline",
            path: [
              { x: 235, y: 305 },
              { x: 275, y: 345 },
              { x: 315, y: 305 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ن"),
  },
  [ductusKey("urdu-nastaliq", "ں")]: {
    script: "urdu-nastaliq",
    glyph: "ں",
    strokes: [
      {
        segments: [
          {
            label: "sweep the independent dotless bowl right to left below the baseline",
            path: [
              { x: 495, y: 210 },
              { x: 475, y: 160 },
              { x: 480, y: 100 },
              { x: 500, y: 40 },
              { x: 510, y: -20 },
              { x: 485, y: -80 },
              { x: 430, y: -140 },
              { x: 360, y: -190 },
              { x: 280, y: -220 },
              { x: 210, y: -215 },
              { x: 150, y: -170 },
              { x: 105, y: -110 },
              { x: 90, y: -60 },
              { x: 95, y: 0 },
              { x: 105, y: 45 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ں"),
  },
  [ductusKey("urdu-nastaliq", "ہ")]: {
    script: "urdu-nastaliq",
    glyph: "ہ",
    strokes: [
      {
        segments: [
          {
            label: "loop the independent teardrop counterclockwise without lifting",
            path: [
              { x: 250, y: 330 },
              { x: 190, y: 305 },
              { x: 150, y: 280 },
              { x: 120, y: 230 },
              { x: 95, y: 170 },
              { x: 90, y: 115 },
              { x: 115, y: 70 },
              { x: 155, y: 40 },
              { x: 200, y: 30 },
              { x: 245, y: 40 },
              { x: 290, y: 70 },
              { x: 320, y: 110 },
              { x: 330, y: 160 },
              { x: 325, y: 210 },
              { x: 305, y: 255 },
              { x: 275, y: 295 },
              { x: 235, y: 330 },
              { x: 195, y: 360 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ہ"),
  },
  [ductusKey("urdu-nastaliq", "ی")]: {
    script: "urdu-nastaliq",
    glyph: "ی",
    strokes: [
      {
        segments: [
          {
            label: "descend from the upper right through the independent S curve",
            path: [
              { x: 548, y: 285 },
              { x: 510, y: 288 },
              { x: 472, y: 270 },
              { x: 430, y: 238 },
              { x: 395, y: 205 },
              { x: 365, y: 168 },
              { x: 345, y: 125 },
              { x: 330, y: 82 },
              { x: 340, y: 45 },
              { x: 375, y: 25 },
              { x: 420, y: 8 },
              { x: 470, y: -2 },
              { x: 520, y: -24 },
              { x: 555, y: -55 },
            ],
          },
          {
            label: "continue left around the below-baseline bowl and finish at its rising tip",
            path: [
              { x: 555, y: -55 },
              { x: 535, y: -98 },
              { x: 495, y: -145 },
              { x: 445, y: -188 },
              { x: 390, y: -218 },
              { x: 325, y: -238 },
              { x: 255, y: -238 },
              { x: 190, y: -218 },
              { x: 140, y: -180 },
              { x: 105, y: -130 },
              { x: 90, y: -78 },
              { x: 94, y: -25 },
              { x: 105, y: 28 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ی"),
  },
  [ductusKey("urdu-nastaliq", "ے")]: {
    script: "urdu-nastaliq",
    glyph: "ے",
    strokes: [
      {
        segments: [
          {
            label: "descend from the upper right and sweep left across the broad bowl",
            path: [
              { x: 360, y: 280 },
              { x: 350, y: 275 },
              { x: 330, y: 252 },
              { x: 310, y: 238 },
              { x: 292, y: 230 },
              { x: 250, y: 215 },
              { x: 200, y: 195 },
              { x: 150, y: 173 },
              { x: 115, y: 145 },
              { x: 100, y: 110 },
            ],
          },
          {
            label: "curl back underneath at the far left without lifting",
            path: [
              { x: 100, y: 110 },
              { x: 90, y: 95 },
              { x: 82, y: 78 },
              { x: 82, y: 62 },
              { x: 95, y: 55 },
              { x: 120, y: 52 },
            ],
          },
          {
            label: "continue right along the lower fold without lifting",
            path: [
              { x: 120, y: 52 },
              { x: 170, y: 30 },
              { x: 250, y: 20 },
              { x: 350, y: 12 },
              { x: 450, y: 10 },
              { x: 550, y: 20 },
              { x: 650, y: 40 },
              { x: 720, y: 62 },
              { x: 740, y: 90 },
            ],
          },
        ],
      },
    ],
    source: urduAlphabetSource("ے"),
  },
  "ب": {
    script: "perso-arabic",
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
    script: "perso-arabic",
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
    script: "perso-arabic",
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
    script: "perso-arabic",
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
    script: "perso-arabic",
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
  "ن": {
    script: "perso-arabic",
    glyph: "ن",
    strokes: [
      {
        segments: [
          {
            label: "sweep the bowl from right to left",
            path: [
              { x: 495, y: 210 },
              { x: 475, y: 160 },
              { x: 480, y: 100 },
              { x: 500, y: 40 },
              { x: 510, y: -20 },
              { x: 485, y: -80 },
              { x: 430, y: -140 },
              { x: 360, y: -190 },
              { x: 280, y: -220 },
              { x: 210, y: -215 },
              { x: 150, y: -170 },
              { x: 105, y: -110 },
              { x: 90, y: -60 },
              { x: 95, y: 0 },
              { x: 105, y: 45 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then place the dot above",
            path: [
              { x: 235, y: 305 },
              { x: 275, y: 345 },
              { x: 315, y: 305 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("ن"),
  },
  "و": {
    script: "perso-arabic",
    glyph: "و",
    strokes: [
      {
        segments: [
          {
            label: "shape the small head loop",
            path: [
              { x: 220, y: 300 },
              { x: 265, y: 315 },
              { x: 315, y: 285 },
              { x: 355, y: 235 },
              { x: 385, y: 170 },
              { x: 393, y: 115 },
              { x: 380, y: 70 },
              { x: 340, y: 45 },
              { x: 285, y: 40 },
              { x: 225, y: 45 },
              { x: 175, y: 80 },
              { x: 145, y: 125 },
              { x: 145, y: 165 },
              { x: 170, y: 215 },
              { x: 210, y: 260 },
              { x: 250, y: 285 },
              { x: 300, y: 285 },
              { x: 345, y: 245 },
              { x: 375, y: 185 },
              { x: 390, y: 115 },
              { x: 390, y: 60 },
            ],
          },
          {
            label: "flow into the leftward tail without lifting",
            path: [
              { x: 390, y: 60 },
              { x: 370, y: -5 },
              { x: 340, y: -70 },
              { x: 300, y: -120 },
              { x: 250, y: -160 },
              { x: 195, y: -170 },
              { x: 135, y: -160 },
              { x: 80, y: -140 },
              { x: 45, y: -120 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("و"),
  },
  "ه": {
    script: "perso-arabic",
    glyph: "ه",
    strokes: [
      {
        segments: [
          {
            label: "loop the isolated body and finish left without lifting",
            path: [
              { x: 315, y: 400 },
              { x: 285, y: 375 },
              { x: 255, y: 350 },
              { x: 230, y: 325 },
              { x: 205, y: 300 },
              { x: 190, y: 260 },
              { x: 190, y: 210 },
              { x: 205, y: 165 },
              { x: 235, y: 125 },
              { x: 275, y: 105 },
              { x: 320, y: 110 },
              { x: 355, y: 135 },
              { x: 380, y: 175 },
              { x: 390, y: 225 },
              { x: 380, y: 275 },
              { x: 355, y: 320 },
              { x: 315, y: 355 },
              { x: 360, y: 355 },
              { x: 410, y: 340 },
              { x: 455, y: 315 },
              { x: 500, y: 275 },
              { x: 535, y: 225 },
              { x: 555, y: 170 },
              { x: 555, y: 115 },
              { x: 535, y: 70 },
              { x: 535, y: 50 },
              { x: 500, y: 40 },
              { x: 455, y: 30 },
              { x: 415, y: 45 },
              { x: 385, y: 75 },
              { x: 365, y: 100 },
              { x: 345, y: 75 },
              { x: 310, y: 65 },
              { x: 270, y: 65 },
              { x: 225, y: 70 },
              { x: 175, y: 65 },
              { x: 120, y: 65 },
              { x: 70, y: 65 },
              { x: 25, y: 65 },
            ],
          },
        ],
      },
    ],
    source: persianAlphabetSource("ه"),
  },
  அ: {
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
    script: "tamil",
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
