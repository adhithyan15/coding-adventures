// ---------------------------------------------------------------------------
// bengali.ts — the Bengali (Bangla) owner of cited pen paths
// ---------------------------------------------------------------------------
//
// WHERE THE ORDER COMES FROM. HP Labs India's Lipi Toolkit ships a Bangla
// handwriting recognizer ("Lipi Indic Character Recognizers 4.0", MIT licence)
// whose model keeps 12,860 native writers' tablet pen traces as its nearest-
// neighbour prototypes. A trace records where the pen went down, which way it
// travelled and where it lifted. Counting those, letter by letter, is the
// evidence; every claim below is a count or a share of them, recorded in the
// letter's `strokeOrderSource.variation` in data/scripts/bengali.json. No trace
// is copied: the model scales each one to a square, so a trace fixes ORDER,
// DIRECTION and LIFTS but not PROPORTIONS. The points here are therefore fitted
// to the bundled Noto Sans Bengali outline instead (font units, y up), and the
// tests in tests/strokes/bengali.test.ts hold them to that ink.
//
// THE HEADLINE. Bengali letters hang from a bar (the mātrā). In isolated
// letters the traces do NOT treat it one way: across the 32 recognizer classes
// that print a full-width bar, 40% of one-stroke prototypes draw no bar run at
// all, 27% draw it first, 16% last, and a multi-stroke writer lifts it out as
// the last stroke more often (20%) than the first (7%). So one rule, two cases:
//
//   * A placement WINS a majority of a letter's traces and covers the printed
//     bar: the path follows it. ব and র draw it first; খ and থ, whose printed
//     bar is only a flag beside the stem, finish with a short move right into
//     it; এ ও ঞ ঃ ঁ ং have none.
//   * NO placement wins a majority: the body is drawn in its majority order,
//     and the headline is drawn LAST, as its own stroke, left to right. This
//     is a CONVENTION, not a count, and every such letter's variation says so
//     with its counts: it is how this track's own lessons teach the bar (body
//     first, bar last), because in running text the bar is one line across
//     the whole word, as in Devanagari. ই চ ছ জ ড ত দ ন ফ ভ ম য ল হ use it;
//     tests/strokes/bengali.test.ts pins that list.
//
// A majority placement that does NOT cover the printed bar is not followed
// and not overridden by the convention: ক (bar first, 296 of 524, but stopped
// at the stem in 281) stays absent. So do letters whose BODY has no majority
// stroke count (আ গ ট ধ প), a near tie (স) or a split path (শ), and ঝ, whose
// printed headline Noto breaks in two around the rising right stem. The
// reasons are in data/scripts/bengali.json.
// ---------------------------------------------------------------------------

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import bengali from "../../../../../learning/human-languages/data/scripts/bengali.json";

/** The cited source a letter's own inventory row records — one source of truth. */
const bengaliLetterSource = (glyph: string): StrokeSource => {
  const letter = bengali.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Bengali ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

/** The same lookup for a sign (ঃ, ঁ, ং), which lives in `marks`. */
const bengaliMarkSource = (mark: string): StrokeSource => {
  const sign = bengali.marks.find((candidate) => candidate.mark === mark);
  if (!sign || !("strokeOrderSource" in sign) || !sign.strokeOrderSource) {
    throw new Error(`Bengali sign ${mark} has no verified source`);
  }
  return sign.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // এ (independent e). One stroke, clockwise: the curl, the arch, the right side down to its foot,
  // then the base swept right to left and up the hook.
  [
    "bengali:এ",
    {
      script: "bengali",
      glyph: "এ",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn clockwise over the arch",
              path: [
                { x: 345, y: 410 },
                { x: 395, y: 408 },
                { x: 425, y: 375 },
                { x: 420, y: 320 },
                { x: 380, y: 292 },
                { x: 330, y: 292 },
                { x: 290, y: 320 },
                { x: 270, y: 380 },
                { x: 285, y: 460 },
                { x: 330, y: 535 },
                { x: 400, y: 585 },
                { x: 490, y: 597 },
                { x: 570, y: 580 },
                { x: 600, y: 530 },
                { x: 602, y: 470 },
              ],
            },
            {
              label: "come down the right side to its foot",
              path: [
                { x: 602, y: 470 },
                { x: 602, y: 300 },
                { x: 602, y: 150 },
                { x: 600, y: 40 },
              ],
            },
            {
              label: "sweep left along the base and up the left hook",
              path: [
                { x: 600, y: 40 },
                { x: 560, y: 95 },
                { x: 480, y: 124 },
                { x: 360, y: 115 },
                { x: 255, y: 107 },
                { x: 170, y: 125 },
                { x: 110, y: 180 },
                { x: 85, y: 270 },
                { x: 95, y: 365 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("এ"),
    },
  ],
  // ও (independent o). One stroke, clockwise throughout: the top curl and upper bowl, a dip into
  // the middle, the lower bowl, and up the long left arm.
  [
    "bengali:ও",
    {
      script: "bengali",
      glyph: "ও",
      strokes: [
        {
          segments: [
            {
              label: "start inside the top curl and turn clockwise round the upper bowl",
              path: [
                { x: 345, y: 462 },
                { x: 385, y: 440 },
                { x: 392, y: 395 },
                { x: 362, y: 358 },
                { x: 318, y: 355 },
                { x: 285, y: 385 },
                { x: 278, y: 440 },
                { x: 305, y: 505 },
                { x: 370, y: 565 },
                { x: 450, y: 595 },
                { x: 510, y: 597 },
                { x: 585, y: 585 },
                { x: 625, y: 530 },
                { x: 632, y: 470 },
                { x: 628, y: 420 },
              ],
            },
            {
              label: "dip into the middle",
              path: [
                { x: 628, y: 420 },
                { x: 600, y: 380 },
                { x: 545, y: 362 },
                { x: 490, y: 345 },
                { x: 505, y: 305 },
                { x: 560, y: 300 },
                { x: 612, y: 300 },
                { x: 640, y: 262 },
              ],
            },
            {
              label: "round the lower bowl clockwise and up the left arm",
              path: [
                { x: 640, y: 262 },
                { x: 648, y: 215 },
                { x: 640, y: 160 },
                { x: 590, y: 105 },
                { x: 480, y: 80 },
                { x: 360, y: 95 },
                { x: 255, y: 165 },
                { x: 175, y: 290 },
                { x: 110, y: 420 },
                { x: 72, y: 515 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ও"),
    },
  ],
  // খ (kha). One stroke from inside the curl (turned counterclockwise), down to the
  // pointed tip, round to the stem's foot, up the stem and out into the flag.
  [
    "bengali:খ",
    {
      script: "bengali",
      glyph: "খ",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn it counterclockwise",
              path: [
                { x: 160, y: 505 },
                { x: 180, y: 555 },
                { x: 165, y: 605 },
                { x: 120, y: 625 },
                { x: 78, y: 605 },
                { x: 57, y: 555 },
                { x: 70, y: 495 },
                { x: 115, y: 458 },
                { x: 180, y: 445 },
                { x: 250, y: 470 },
                { x: 295, y: 520 },
                { x: 325, y: 590 },
              ],
            },
            {
              label: "cross the middle bend and come down to the pointed left tip",
              path: [
                { x: 325, y: 590 },
                { x: 360, y: 560 },
                { x: 378, y: 500 },
                { x: 375, y: 440 },
                { x: 345, y: 385 },
                { x: 285, y: 345 },
                { x: 200, y: 315 },
                { x: 120, y: 300 },
              ],
            },
            {
              label: "round the lower curve to the foot of the stem",
              path: [
                { x: 120, y: 300 },
                { x: 150, y: 255 },
                { x: 250, y: 235 },
                { x: 340, y: 190 },
                { x: 420, y: 115 },
                { x: 480, y: 40 },
                { x: 527, y: 12 },
              ],
            },
            {
              label: "climb the stem and turn right into the flag",
              path: [
                { x: 527, y: 12 },
                { x: 527, y: 200 },
                { x: 527, y: 400 },
                { x: 527, y: 600 },
                { x: 527, y: 650 },
                { x: 570, y: 600 },
                { x: 660, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("খ"),
    },
  ],
  // থ (tha). As খ, but the curl turns clockwise and opens into a bowl over the top.
  [
    "bengali:থ",
    {
      script: "bengali",
      glyph: "থ",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn it clockwise",
              path: [
                { x: 140, y: 515 },
                { x: 178, y: 492 },
                { x: 182, y: 450 },
                { x: 152, y: 420 },
                { x: 105, y: 416 },
                { x: 70, y: 445 },
                { x: 60, y: 495 },
                { x: 85, y: 560 },
                { x: 150, y: 605 },
                { x: 215, y: 598 },
                { x: 290, y: 590 },
                { x: 345, y: 550 },
                { x: 360, y: 490 },
              ],
            },
            {
              label: "run over the top of the bowl and down to the pointed left tip",
              path: [
                { x: 360, y: 490 },
                { x: 350, y: 420 },
                { x: 300, y: 360 },
                { x: 200, y: 318 },
                { x: 105, y: 300 },
              ],
            },
            {
              label: "round the lower curve to the foot of the stem",
              path: [
                { x: 105, y: 300 },
                { x: 140, y: 255 },
                { x: 240, y: 238 },
                { x: 330, y: 190 },
                { x: 405, y: 120 },
                { x: 465, y: 40 },
                { x: 511, y: 12 },
              ],
            },
            {
              label: "climb the stem and turn right into the flag",
              path: [
                { x: 511, y: 12 },
                { x: 511, y: 200 },
                { x: 511, y: 400 },
                { x: 511, y: 600 },
                { x: 511, y: 640 },
                { x: 555, y: 600 },
                { x: 645, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("থ"),
    },
  ],
  // ঞ (nya). Two strokes: the left part exactly as এ, then the two right lobes, each
  // clockwise, starting from the stem.
  [
    "bengali:ঞ",
    {
      script: "bengali",
      glyph: "ঞ",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn clockwise over the arch",
              path: [
                { x: 345, y: 410 },
                { x: 395, y: 408 },
                { x: 425, y: 375 },
                { x: 420, y: 320 },
                { x: 380, y: 292 },
                { x: 330, y: 292 },
                { x: 290, y: 320 },
                { x: 270, y: 380 },
                { x: 285, y: 460 },
                { x: 330, y: 535 },
                { x: 400, y: 585 },
                { x: 490, y: 597 },
                { x: 570, y: 580 },
                { x: 600, y: 530 },
                { x: 602, y: 470 },
              ],
            },
            {
              label: "come down the right side to its foot",
              path: [
                { x: 602, y: 470 },
                { x: 602, y: 300 },
                { x: 602, y: 150 },
                { x: 600, y: 40 },
              ],
            },
            {
              label: "sweep left along the base and up the left hook",
              path: [
                { x: 600, y: 40 },
                { x: 560, y: 95 },
                { x: 480, y: 124 },
                { x: 360, y: 115 },
                { x: 255, y: 107 },
                { x: 170, y: 125 },
                { x: 110, y: 180 },
                { x: 85, y: 270 },
                { x: 95, y: 365 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then arch from the stem over the upper right lobe",
              path: [
                { x: 650, y: 430 },
                { x: 690, y: 495 },
                { x: 745, y: 545 },
                { x: 800, y: 562 },
                { x: 860, y: 535 },
                { x: 885, y: 470 },
                { x: 880, y: 415 },
              ],
            },
            {
              label: "dip into the middle",
              path: [
                { x: 880, y: 415 },
                { x: 845, y: 378 },
                { x: 790, y: 368 },
                { x: 748, y: 350 },
                { x: 778, y: 318 },
                { x: 840, y: 322 },
                { x: 878, y: 300 },
                { x: 892, y: 262 },
              ],
            },
            {
              label: "round the lower lobe clockwise back to the stem",
              path: [
                { x: 892, y: 262 },
                { x: 892, y: 215 },
                { x: 870, y: 165 },
                { x: 820, y: 145 },
                { x: 750, y: 150 },
                { x: 690, y: 180 },
                { x: 650, y: 230 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ঞ"),
    },
  ],
  // ব (ba). One stroke, headline FIRST (the majority placement for this letter): the
  // bar left to right, the turn into the stem and down the diagonal, the lower
  // curve, and the climb back up the stem.
  [
    "bengali:ব",
    {
      script: "bengali",
      glyph: "ব",
      strokes: [
        {
          segments: [
            {
              label: "draw the headline from left to right",
              path: [
                { x: 0, y: 586 },
                { x: 150, y: 586 },
                { x: 300, y: 586 },
                { x: 450, y: 586 },
                { x: 590, y: 586 },
              ],
            },
            {
              label: "turn into the stem and run down the diagonal to the pointed left tip",
              path: [
                { x: 590, y: 586 },
                { x: 501, y: 562 },
                { x: 462, y: 505 },
                { x: 400, y: 465 },
                { x: 300, y: 420 },
                { x: 200, y: 370 },
                { x: 95, y: 315 },
              ],
            },
            {
              label: "round the lower curve to the foot of the stem",
              path: [
                { x: 95, y: 315 },
                { x: 110, y: 270 },
                { x: 200, y: 250 },
                { x: 290, y: 210 },
                { x: 370, y: 150 },
                { x: 425, y: 80 },
                { x: 462, y: 15 },
              ],
            },
            {
              label: "climb the stem to the headline",
              path: [
                { x: 462, y: 15 },
                { x: 462, y: 150 },
                { x: 462, y: 300 },
                { x: 462, y: 450 },
                { x: 462, y: 600 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ব"),
    },
  ],
  // র (ra). ব's body exactly, then a lift and the dot. The dot is a short touch: the
  // traces do not settle which way it turns, so the path claims nothing.
  [
    "bengali:র",
    {
      script: "bengali",
      glyph: "র",
      strokes: [
        {
          segments: [
            {
              label: "draw the headline from left to right",
              path: [
                { x: 0, y: 586 },
                { x: 150, y: 586 },
                { x: 300, y: 586 },
                { x: 450, y: 586 },
                { x: 590, y: 586 },
              ],
            },
            {
              label: "turn into the stem and run down the diagonal to the pointed left tip",
              path: [
                { x: 590, y: 586 },
                { x: 501, y: 562 },
                { x: 462, y: 505 },
                { x: 400, y: 465 },
                { x: 300, y: 420 },
                { x: 200, y: 370 },
                { x: 95, y: 315 },
              ],
            },
            {
              label: "round the lower curve to the foot of the stem",
              path: [
                { x: 95, y: 315 },
                { x: 110, y: 270 },
                { x: 200, y: 250 },
                { x: 290, y: 210 },
                { x: 370, y: 150 },
                { x: 425, y: 80 },
                { x: 462, y: 15 },
              ],
            },
            {
              label: "climb the stem to the headline",
              path: [
                { x: 462, y: 15 },
                { x: 462, y: 150 },
                { x: 462, y: 300 },
                { x: 462, y: 450 },
                { x: 462, y: 600 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot below",
              path: [
                { x: 180, y: 75 },
                { x: 195, y: 54 },
                { x: 212, y: 35 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("র"),
    },
  ],
  // ঃ (bisarga). Two loops, upper first, each counterclockwise from its top.
  [
    "bengali:ঃ",
    {
      script: "bengali",
      glyph: "ঃ",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper loop counterclockwise from its top",
              path: [
                { x: 219, y: 591 },
                { x: 167, y: 577 },
                { x: 128, y: 538 },
                { x: 114, y: 486 },
                { x: 128, y: 434 },
                { x: 166, y: 395 },
                { x: 219, y: 381 },
                { x: 272, y: 395 },
                { x: 310, y: 433 },
                { x: 324, y: 486 },
                { x: 310, y: 538 },
                { x: 272, y: 577 },
                { x: 219, y: 591 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower loop the same way",
              path: [
                { x: 219, y: 242 },
                { x: 167, y: 228 },
                { x: 128, y: 190 },
                { x: 114, y: 137 },
                { x: 128, y: 84 },
                { x: 166, y: 46 },
                { x: 219, y: 32 },
                { x: 272, y: 46 },
                { x: 310, y: 84 },
                { x: 324, y: 137 },
                { x: 310, y: 190 },
                { x: 272, y: 228 },
                { x: 219, y: 242 },
              ],
            },
          ],
        },
      ],
      source: bengaliMarkSource("ঃ"),
    },
  ],
  // ঁ (chandrabindu). The bowl first (counterclockwise: down, round, up), then a lift and the dot.
  [
    "bengali:ঁ",
    {
      script: "bengali",
      glyph: "ঁ",
      strokes: [
        {
          segments: [
            {
              label: "draw the bowl from its left tip, down, round and up",
              path: [
                { x: -140, y: 862 },
                { x: -132, y: 800 },
                { x: -90, y: 745 },
                { x: -30, y: 720 },
                { x: 30, y: 720 },
                { x: 90, y: 745 },
                { x: 132, y: 800 },
                { x: 142, y: 862 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot",
              path: [
                { x: -12, y: 850 },
                { x: 0, y: 833 },
                { x: 12, y: 816 },
              ],
            },
          ],
        },
      ],
      source: bengaliMarkSource("ঁ"),
    },
  ],
  // ং (anusvar). The ring counterclockwise from its top, then a lift and the tail down to the right.
  [
    "bengali:ং",
    {
      script: "bengali",
      glyph: "ং",
      strokes: [
        {
          segments: [
            {
              label: "draw the ring counterclockwise from its top",
              path: [
                { x: 219, y: 590 },
                { x: 181, y: 579 },
                { x: 149, y: 557 },
                { x: 127, y: 525 },
                { x: 120, y: 486 },
                { x: 127, y: 448 },
                { x: 149, y: 415 },
                { x: 181, y: 394 },
                { x: 219, y: 386 },
                { x: 257, y: 394 },
                { x: 289, y: 415 },
                { x: 311, y: 448 },
                { x: 319, y: 486 },
                { x: 311, y: 525 },
                { x: 289, y: 557 },
                { x: 257, y: 579 },
                { x: 219, y: 590 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the tail from its upper-left end down to the right",
              path: [
                { x: 98, y: 282 },
                { x: 127, y: 263 },
                { x: 160, y: 238 },
                { x: 195, y: 206 },
                { x: 229, y: 170 },
                { x: 261, y: 135 },
                { x: 289, y: 100 },
                { x: 315, y: 67 },
                { x: 341, y: 34 },
                { x: 368, y: 3 },
                { x: 392, y: -25 },
              ],
            },
          ],
        },
      ],
      source: bengaliMarkSource("ং"),
    },
  ],
  // ই (independent i). হ's body first (inside the curl, clockwise over the top, down to the left tip,
  // back along the base and down the tail), then a lift and the hook climbed from the headline up to
  // its tip, then, BY CONVENTION, a lift and the headline last, left to right.
  [
    "bengali:ই",
    {
      script: "bengali",
      glyph: "ই",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn clockwise over the top",
              path: [
                { x: 150, y: 375 },
                { x: 160, y: 425 },
                { x: 200, y: 460 },
                { x: 250, y: 468 },
                { x: 320, y: 462 },
                { x: 380, y: 442 },
              ],
            },
            {
              label: "come down the right side and round to the left tip",
              path: [
                { x: 380, y: 442 },
                { x: 396, y: 402 },
                { x: 416, y: 363 },
                { x: 422, y: 319 },
                { x: 418, y: 275 },
                { x: 397, y: 236 },
                { x: 365, y: 206 },
                { x: 325, y: 187 },
                { x: 283, y: 175 },
                { x: 241, y: 160 },
                { x: 197, y: 159 },
                { x: 154, y: 167 },
                { x: 110, y: 175 },
              ],
            },
            {
              label: "sweep back right along the base and down the tail",
              path: [
                { x: 110, y: 175 },
                { x: 152, y: 167 },
                { x: 194, y: 160 },
                { x: 235, y: 148 },
                { x: 265, y: 117 },
                { x: 303, y: 98 },
                { x: 341, y: 79 },
                { x: 379, y: 58 },
                { x: 415, y: 36 },
                { x: 451, y: 12 },
                { x: 486, y: -13 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then from the headline climb the hook and curve left to its tip",
              path: [
                { x: 386, y: 591 },
                { x: 390, y: 636 },
                { x: 383, y: 681 },
                { x: 353, y: 716 },
                { x: 312, y: 735 },
                { x: 266, y: 739 },
                { x: 220, y: 739 },
                { x: 175, y: 742 },
                { x: 129, y: 745 },
                { x: 85, y: 759 },
                { x: 50, y: 787 },
                { x: 32, y: 829 },
                { x: 30, y: 875 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 142, y: 586 },
                { x: 265, y: 586 },
                { x: 388, y: 586 },
                { x: 510, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ই"),
    },
  ],
  // চ (ca). Down the stem from under the headline, round the bottom, up the right side and back over
  // the top to the stem (counterclockwise); then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:চ",
    {
      script: "bengali",
      glyph: "চ",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and come down the stem",
              path: [
                { x: 138, y: 572 },
                { x: 138, y: 525 },
                { x: 138, y: 479 },
                { x: 134, y: 433 },
                { x: 134, y: 386 },
                { x: 134, y: 340 },
                { x: 134, y: 293 },
                { x: 134, y: 247 },
                { x: 134, y: 200 },
              ],
            },
            {
              label: "round the bottom and climb the right side",
              path: [
                { x: 134, y: 200 },
                { x: 136, y: 155 },
                { x: 148, y: 111 },
                { x: 177, y: 77 },
                { x: 220, y: 64 },
                { x: 265, y: 65 },
                { x: 308, y: 79 },
                { x: 345, y: 105 },
                { x: 377, y: 137 },
                { x: 403, y: 174 },
                { x: 424, y: 215 },
                { x: 437, y: 258 },
                { x: 438, y: 303 },
                { x: 434, y: 348 },
              ],
            },
            {
              label: "turn left over the top and come back to the stem",
              path: [
                { x: 434, y: 348 },
                { x: 400, y: 379 },
                { x: 356, y: 393 },
                { x: 310, y: 399 },
                { x: 264, y: 409 },
                { x: 223, y: 430 },
                { x: 188, y: 460 },
                { x: 146, y: 480 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 144, y: 586 },
                { x: 268, y: 586 },
                { x: 391, y: 586 },
                { x: 515, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("চ"),
    },
  ],
  // ছ (cha). Down the stem, round the small loop, over and round the big bowl to the left tip, then
  // back along the base and down the tail to the bottom right; then, BY CONVENTION, the headline last.
  [
    "bengali:ছ",
    {
      script: "bengali",
      glyph: "ছ",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and come down the stem",
              path: [
                { x: 130, y: 569 },
                { x: 130, y: 522 },
                { x: 130, y: 475 },
                { x: 130, y: 427 },
                { x: 130, y: 380 },
                { x: 130, y: 333 },
              ],
            },
            {
              label: "round the small loop and climb to its top",
              path: [
                { x: 130, y: 333 },
                { x: 143, y: 289 },
                { x: 177, y: 258 },
                { x: 223, y: 254 },
                { x: 266, y: 271 },
                { x: 299, y: 303 },
                { x: 324, y: 342 },
                { x: 334, y: 387 },
                { x: 326, y: 433 },
              ],
            },
            {
              label: "run right and round the big bowl down to the left tip",
              path: [
                { x: 326, y: 433 },
                { x: 370, y: 445 },
                { x: 415, y: 440 },
                { x: 455, y: 418 },
                { x: 485, y: 383 },
                { x: 498, y: 339 },
                { x: 501, y: 293 },
                { x: 491, y: 248 },
                { x: 468, y: 208 },
                { x: 436, y: 175 },
                { x: 399, y: 148 },
                { x: 356, y: 131 },
                { x: 315, y: 110 },
                { x: 269, y: 108 },
                { x: 224, y: 114 },
                { x: 178, y: 121 },
              ],
            },
            {
              label: "sweep back right along the base and down the tail",
              path: [
                { x: 178, y: 121 },
                { x: 224, y: 114 },
                { x: 270, y: 108 },
                { x: 315, y: 97 },
                { x: 352, y: 69 },
                { x: 396, y: 52 },
                { x: 439, y: 34 },
                { x: 481, y: 15 },
                { x: 522, y: -8 },
                { x: 562, y: -31 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 160, y: 586 },
                { x: 300, y: 586 },
                { x: 440, y: 586 },
                { x: 580, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ছ"),
    },
  ],
  // জ (ja). The left part first (from under the headline round the inner loop, clockwise, down the
  // right side, round the bottom and up the left arm), then a lift and the right part down to the
  // foot of the right leg; then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:জ",
    {
      script: "bengali",
      glyph: "জ",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and curve down round the inner loop",
              path: [
                { x: 370, y: 569 },
                { x: 361, y: 525 },
                { x: 329, y: 492 },
                { x: 301, y: 457 },
                { x: 281, y: 416 },
                { x: 274, y: 372 },
                { x: 283, y: 328 },
                { x: 313, y: 294 },
                { x: 354, y: 277 },
                { x: 400, y: 277 },
                { x: 443, y: 291 },
                { x: 481, y: 316 },
                { x: 522, y: 333 },
              ],
            },
            {
              label: "come down the right side and round the bottom",
              path: [
                { x: 522, y: 333 },
                { x: 545, y: 295 },
                { x: 552, y: 250 },
                { x: 550, y: 205 },
                { x: 539, y: 162 },
                { x: 513, y: 125 },
                { x: 479, y: 96 },
                { x: 437, y: 79 },
                { x: 392, y: 73 },
                { x: 347, y: 73 },
                { x: 303, y: 82 },
                { x: 262, y: 101 },
              ],
            },
            {
              label: "climb the left arm to its tip",
              path: [
                { x: 262, y: 101 },
                { x: 228, y: 127 },
                { x: 198, y: 157 },
                { x: 172, y: 191 },
                { x: 150, y: 228 },
                { x: 131, y: 266 },
                { x: 115, y: 306 },
                { x: 101, y: 347 },
                { x: 89, y: 388 },
                { x: 78, y: 429 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then from the headline run down to the right and down the right leg",
              path: [
                { x: 470, y: 569 },
                { x: 491, y: 531 },
                { x: 521, y: 498 },
                { x: 555, y: 471 },
                { x: 594, y: 452 },
                { x: 636, y: 440 },
                { x: 680, y: 437 },
                { x: 722, y: 428 },
                { x: 738, y: 390 },
                { x: 725, y: 348 },
                { x: 714, y: 306 },
                { x: 710, y: 262 },
                { x: 710, y: 218 },
                { x: 710, y: 174 },
                { x: 713, y: 131 },
                { x: 720, y: 88 },
                { x: 730, y: 45 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 222, y: 586 },
                { x: 425, y: 586 },
                { x: 628, y: 586 },
                { x: 830, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("জ"),
    },
  ],
  // ড (ḍa). Down the stem from under the headline, right and up to the point, then clockwise down the
  // right side, round the bottom and up the left arm; then, BY CONVENTION, the headline last.
  [
    "bengali:ড",
    {
      script: "bengali",
      glyph: "ড",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and come down the stem",
              path: [
                { x: 318, y: 572 },
                { x: 318, y: 530 },
                { x: 318, y: 489 },
                { x: 318, y: 447 },
                { x: 318, y: 406 },
                { x: 318, y: 364 },
              ],
            },
            {
              label: "turn right and up to the point",
              path: [
                { x: 318, y: 364 },
                { x: 334, y: 321 },
                { x: 368, y: 288 },
                { x: 414, y: 282 },
                { x: 457, y: 300 },
                { x: 493, y: 331 },
                { x: 526, y: 364 },
                { x: 566, y: 388 },
              ],
            },
            {
              label: "come down the right side and round the bottom",
              path: [
                { x: 566, y: 388 },
                { x: 593, y: 354 },
                { x: 609, y: 313 },
                { x: 616, y: 270 },
                { x: 617, y: 227 },
                { x: 609, y: 184 },
                { x: 590, y: 145 },
                { x: 560, y: 114 },
                { x: 525, y: 87 },
                { x: 485, y: 70 },
                { x: 442, y: 64 },
                { x: 398, y: 64 },
                { x: 355, y: 68 },
                { x: 313, y: 81 },
                { x: 274, y: 100 },
              ],
            },
            {
              label: "climb the left arm to its tip",
              path: [
                { x: 274, y: 100 },
                { x: 239, y: 127 },
                { x: 208, y: 158 },
                { x: 181, y: 193 },
                { x: 158, y: 231 },
                { x: 138, y: 270 },
                { x: 120, y: 311 },
                { x: 105, y: 352 },
                { x: 91, y: 394 },
                { x: 78, y: 436 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 188, y: 586 },
                { x: 355, y: 586 },
                { x: 522, y: 586 },
                { x: 690, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ড"),
    },
  ],
  // ত (ta). One body stroke from inside the curl, clockwise over the top, down the right side, round
  // the bottom and up the left arm; then, BY CONVENTION, a lift and the headline (printed apart) last.
  [
    "bengali:ত",
    {
      script: "bengali",
      glyph: "ত",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn clockwise over the top",
              path: [
                { x: 338, y: 382 },
                { x: 374, y: 402 },
                { x: 398, y: 437 },
                { x: 435, y: 457 },
                { x: 479, y: 457 },
                { x: 520, y: 446 },
                { x: 555, y: 421 },
                { x: 582, y: 387 },
                { x: 600, y: 348 },
                { x: 610, y: 306 },
              ],
            },
            {
              label: "come down the right side and round the bottom",
              path: [
                { x: 610, y: 306 },
                { x: 610, y: 263 },
                { x: 604, y: 220 },
                { x: 588, y: 180 },
                { x: 560, y: 148 },
                { x: 528, y: 118 },
                { x: 489, y: 99 },
                { x: 447, y: 91 },
                { x: 403, y: 90 },
                { x: 360, y: 92 },
                { x: 318, y: 102 },
              ],
            },
            {
              label: "climb the left arm to its tip",
              path: [
                { x: 318, y: 102 },
                { x: 276, y: 121 },
                { x: 239, y: 150 },
                { x: 206, y: 182 },
                { x: 176, y: 218 },
                { x: 152, y: 258 },
                { x: 131, y: 299 },
                { x: 112, y: 342 },
                { x: 97, y: 386 },
                { x: 82, y: 430 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 188, y: 586 },
                { x: 355, y: 586 },
                { x: 522, y: 586 },
                { x: 690, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ত"),
    },
  ],
  // দ (da). Down the left stem from under the headline, up to the right to the shoulder, and down the
  // right side to its foot; then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:দ",
    {
      script: "bengali",
      glyph: "দ",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and come down the left stem",
              path: [
                { x: 118, y: 570 },
                { x: 118, y: 524 },
                { x: 118, y: 477 },
                { x: 118, y: 431 },
                { x: 118, y: 385 },
                { x: 118, y: 339 },
                { x: 121, y: 292 },
                { x: 138, y: 250 },
              ],
            },
            {
              label: "turn and run up to the right to the shoulder",
              path: [
                { x: 138, y: 250 },
                { x: 179, y: 267 },
                { x: 212, y: 296 },
                { x: 243, y: 327 },
                { x: 276, y: 357 },
                { x: 311, y: 384 },
                { x: 347, y: 408 },
                { x: 387, y: 425 },
                { x: 430, y: 418 },
              ],
            },
            {
              label: "come down the right side to its foot",
              path: [
                { x: 430, y: 418 },
                { x: 426, y: 375 },
                { x: 413, y: 333 },
                { x: 404, y: 290 },
                { x: 399, y: 247 },
                { x: 398, y: 203 },
                { x: 399, y: 160 },
                { x: 404, y: 116 },
                { x: 412, y: 73 },
                { x: 418, y: 30 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 148, y: 586 },
                { x: 275, y: 586 },
                { x: 402, y: 586 },
                { x: 530, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("দ"),
    },
  ],
  // ন (na). From inside the curl up over the arch to the stem, down to its foot and back up the stem;
  // then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:ন",
    {
      script: "bengali",
      glyph: "ন",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and arch up and over to the right",
              path: [
                { x: 150, y: 264 },
                { x: 142, y: 308 },
                { x: 152, y: 349 },
                { x: 184, y: 379 },
                { x: 228, y: 388 },
                { x: 271, y: 378 },
                { x: 312, y: 360 },
                { x: 350, y: 336 },
                { x: 384, y: 306 },
                { x: 417, y: 276 },
                { x: 458, y: 260 },
              ],
            },
            {
              label: "come down to the foot of the stem",
              path: [
                { x: 458, y: 260 },
                { x: 466, y: 216 },
                { x: 466, y: 171 },
                { x: 466, y: 126 },
                { x: 466, y: 81 },
                { x: 466, y: 36 },
              ],
            },
            {
              label: "climb the stem to the headline",
              path: [
                { x: 466, y: 36 },
                { x: 466, y: 81 },
                { x: 466, y: 126 },
                { x: 466, y: 171 },
                { x: 466, y: 216 },
                { x: 463, y: 261 },
                { x: 470, y: 306 },
                { x: 470, y: 351 },
                { x: 468, y: 396 },
                { x: 466, y: 441 },
                { x: 466, y: 486 },
                { x: 466, y: 531 },
                { x: 466, y: 576 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 159, y: 586 },
                { x: 298, y: 586 },
                { x: 436, y: 586 },
                { x: 575, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ন"),
    },
  ],
  // ফ (pha). The zigzag from under the headline down to the foot of the stem, up the stem, over the
  // arch and down into the loop; then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:ফ",
    {
      script: "bengali",
      glyph: "ফ",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and zigzag down: out to the right point, back to the left point",
              path: [
                { x: 102, y: 572 },
                { x: 121, y: 534 },
                { x: 153, y: 505 },
                { x: 192, y: 485 },
                { x: 230, y: 464 },
                { x: 264, y: 438 },
                { x: 292, y: 404 },
                { x: 266, y: 373 },
                { x: 226, y: 355 },
                { x: 188, y: 335 },
                { x: 151, y: 312 },
                { x: 118, y: 284 },
              ],
            },
            {
              label: "run down to the right to the foot of the stem",
              path: [
                { x: 118, y: 284 },
                { x: 145, y: 246 },
                { x: 185, y: 222 },
                { x: 229, y: 205 },
                { x: 273, y: 186 },
                { x: 314, y: 162 },
                { x: 353, y: 135 },
                { x: 389, y: 104 },
                { x: 423, y: 72 },
                { x: 466, y: 52 },
              ],
            },
            {
              label: "climb the stem",
              path: [
                { x: 466, y: 52 },
                { x: 478, y: 96 },
                { x: 478, y: 143 },
                { x: 478, y: 189 },
                { x: 478, y: 235 },
                { x: 478, y: 281 },
                { x: 478, y: 327 },
                { x: 478, y: 373 },
                { x: 478, y: 419 },
                { x: 486, y: 464 },
              ],
            },
            {
              label: "arch over to the right and curl down into the loop",
              path: [
                { x: 486, y: 464 },
                { x: 533, y: 468 },
                { x: 579, y: 459 },
                { x: 623, y: 445 },
                { x: 665, y: 423 },
                { x: 699, y: 391 },
                { x: 730, y: 356 },
                { x: 742, y: 312 },
                { x: 732, y: 267 },
                { x: 698, y: 235 },
                { x: 654, y: 236 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 216, y: 586 },
                { x: 412, y: 586 },
                { x: 609, y: 586 },
                { x: 805, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ফ"),
    },
  ],
  // ভ (bha). One body stroke from inside the curl right to the point, clockwise down the right side,
  // round the bottom and up the left arm; then, BY CONVENTION, the headline (printed apart) last.
  [
    "bengali:ভ",
    {
      script: "bengali",
      glyph: "ভ",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and run right to the point",
              path: [
                { x: 306, y: 410 },
                { x: 326, y: 372 },
                { x: 356, y: 341 },
                { x: 397, y: 330 },
                { x: 438, y: 342 },
                { x: 473, y: 369 },
                { x: 503, y: 399 },
                { x: 534, y: 430 },
              ],
            },
            {
              label: "come down the right side and round the bottom",
              path: [
                { x: 534, y: 430 },
                { x: 564, y: 398 },
                { x: 583, y: 359 },
                { x: 593, y: 316 },
                { x: 594, y: 272 },
                { x: 592, y: 228 },
                { x: 579, y: 186 },
                { x: 554, y: 150 },
                { x: 522, y: 120 },
                { x: 483, y: 99 },
                { x: 440, y: 90 },
                { x: 396, y: 90 },
                { x: 352, y: 94 },
                { x: 310, y: 106 },
              ],
            },
            {
              label: "climb the left arm to its tip",
              path: [
                { x: 310, y: 106 },
                { x: 269, y: 126 },
                { x: 234, y: 154 },
                { x: 202, y: 187 },
                { x: 173, y: 222 },
                { x: 150, y: 261 },
                { x: 129, y: 302 },
                { x: 112, y: 344 },
                { x: 96, y: 387 },
                { x: 82, y: 430 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 188, y: 586 },
                { x: 355, y: 586 },
                { x: 522, y: 586 },
                { x: 690, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ভ"),
    },
  ],
  // ম (ma). From under the headline at the left, down the curve, round the loop clockwise, out
  // to the stem, down to its foot and back up; then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:ম",
    {
      script: "bengali",
      glyph: "ম",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline at the left and curve down to the loop",
              path: [
                { x: 90, y: 568 },
                { x: 114, y: 529 },
                { x: 152, y: 503 },
                { x: 190, y: 475 },
                { x: 219, y: 438 },
                { x: 238, y: 395 },
                { x: 247, y: 349 },
                { x: 248, y: 303 },
                { x: 222, y: 264 },
              ],
            },
            {
              label: "round the loop clockwise",
              path: [
                { x: 222, y: 264 },
                { x: 262, y: 262 },
                { x: 255, y: 215 },
                { x: 222, y: 182 },
                { x: 182, y: 182 },
                { x: 155, y: 212 },
                { x: 155, y: 255 },
                { x: 180, y: 288 },
                { x: 220, y: 300 },
                { x: 262, y: 282 },
              ],
            },
            {
              label: "run out to the right to the stem and down to its foot",
              path: [
                { x: 262, y: 282 },
                { x: 306, y: 274 },
                { x: 347, y: 254 },
                { x: 384, y: 226 },
                { x: 418, y: 193 },
                { x: 458, y: 171 },
                { x: 476, y: 129 },
                { x: 478, y: 83 },
                { x: 478, y: 36 },
              ],
            },
            {
              label: "climb the stem to the headline",
              path: [
                { x: 478, y: 36 },
                { x: 478, y: 81 },
                { x: 476, y: 126 },
                { x: 469, y: 171 },
                { x: 478, y: 215 },
                { x: 478, y: 260 },
                { x: 478, y: 305 },
                { x: 478, y: 350 },
                { x: 478, y: 395 },
                { x: 478, y: 441 },
                { x: 478, y: 486 },
                { x: 478, y: 531 },
                { x: 478, y: 576 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 161, y: 586 },
                { x: 302, y: 586 },
                { x: 444, y: 586 },
                { x: 585, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ম"),
    },
  ],
  // য (ya). The zigzag from under the headline down to the foot of the stem, then up the stem; then,
  // BY CONVENTION, a lift and the headline last.
  [
    "bengali:য",
    {
      script: "bengali",
      glyph: "য",
      strokes: [
        {
          segments: [
            {
              label: "start under the headline and zigzag down: out to the right point, back to the left point",
              path: [
                { x: 110, y: 572 },
                { x: 129, y: 534 },
                { x: 161, y: 505 },
                { x: 200, y: 486 },
                { x: 238, y: 464 },
                { x: 272, y: 438 },
                { x: 300, y: 404 },
                { x: 274, y: 373 },
                { x: 235, y: 355 },
                { x: 197, y: 334 },
                { x: 159, y: 312 },
                { x: 126, y: 284 },
              ],
            },
            {
              label: "run down to the right to the foot of the stem",
              path: [
                { x: 126, y: 284 },
                { x: 149, y: 249 },
                { x: 185, y: 225 },
                { x: 225, y: 211 },
                { x: 265, y: 194 },
                { x: 303, y: 175 },
                { x: 340, y: 153 },
                { x: 375, y: 128 },
                { x: 407, y: 99 },
                { x: 439, y: 70 },
                { x: 478, y: 52 },
              ],
            },
            {
              label: "climb the stem to the headline",
              path: [
                { x: 478, y: 52 },
                { x: 493, y: 93 },
                { x: 494, y: 137 },
                { x: 494, y: 181 },
                { x: 490, y: 224 },
                { x: 490, y: 268 },
                { x: 490, y: 312 },
                { x: 490, y: 356 },
                { x: 490, y: 400 },
                { x: 490, y: 444 },
                { x: 490, y: 488 },
                { x: 490, y: 532 },
                { x: 490, y: 576 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 165, y: 586 },
                { x: 310, y: 586 },
                { x: 455, y: 586 },
                { x: 600, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("য"),
    },
  ],
  // ল (la). From inside the curl round the left side and over both arches to the stem, down to its
  // foot and back up; then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:ল",
    {
      script: "bengali",
      glyph: "ল",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and round the left side up over the first arch",
              path: [
                { x: 250, y: 215 },
                { x: 215, y: 180 },
                { x: 170, y: 182 },
                { x: 120, y: 225 },
                { x: 92, y: 300 },
                { x: 110, y: 365 },
                { x: 160, y: 405 },
                { x: 215, y: 415 },
                { x: 275, y: 408 },
                { x: 320, y: 382 },
                { x: 350, y: 345 },
              ],
            },
            {
              label: "climb over the second arch and down to the stem",
              path: [
                { x: 350, y: 345 },
                { x: 382, y: 376 },
                { x: 418, y: 403 },
                { x: 462, y: 408 },
                { x: 506, y: 401 },
                { x: 544, y: 377 },
                { x: 586, y: 360 },
              ],
            },
            {
              label: "come down to the foot of the stem",
              path: [
                { x: 586, y: 360 },
                { x: 598, y: 315 },
                { x: 598, y: 269 },
                { x: 598, y: 222 },
                { x: 598, y: 176 },
                { x: 598, y: 129 },
                { x: 598, y: 83 },
                { x: 598, y: 36 },
              ],
            },
            {
              label: "climb the stem to the headline",
              path: [
                { x: 598, y: 36 },
                { x: 598, y: 81 },
                { x: 598, y: 126 },
                { x: 598, y: 171 },
                { x: 598, y: 217 },
                { x: 598, y: 262 },
                { x: 598, y: 307 },
                { x: 591, y: 351 },
                { x: 597, y: 396 },
                { x: 598, y: 441 },
                { x: 598, y: 486 },
                { x: 598, y: 531 },
                { x: 598, y: 576 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 191, y: 586 },
                { x: 362, y: 586 },
                { x: 534, y: 586 },
                { x: 705, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("ল"),
    },
  ],
  // হ (ha). From inside the curl clockwise over the top, down the right side to the left tip, then
  // back along the base and down the tail; then, BY CONVENTION, a lift and the headline last.
  [
    "bengali:হ",
    {
      script: "bengali",
      glyph: "হ",
      strokes: [
        {
          segments: [
            {
              label: "start inside the curl and turn clockwise over the top",
              path: [
                { x: 150, y: 375 },
                { x: 160, y: 425 },
                { x: 200, y: 460 },
                { x: 250, y: 468 },
                { x: 320, y: 462 },
                { x: 380, y: 442 },
              ],
            },
            {
              label: "come down the right side and round to the left tip",
              path: [
                { x: 380, y: 442 },
                { x: 396, y: 402 },
                { x: 416, y: 363 },
                { x: 422, y: 319 },
                { x: 418, y: 275 },
                { x: 397, y: 236 },
                { x: 365, y: 206 },
                { x: 325, y: 187 },
                { x: 283, y: 175 },
                { x: 241, y: 160 },
                { x: 197, y: 159 },
                { x: 154, y: 167 },
                { x: 110, y: 175 },
              ],
            },
            {
              label: "sweep back right along the base and down the tail",
              path: [
                { x: 110, y: 175 },
                { x: 152, y: 167 },
                { x: 194, y: 160 },
                { x: 235, y: 148 },
                { x: 265, y: 117 },
                { x: 303, y: 98 },
                { x: 341, y: 79 },
                { x: 379, y: 58 },
                { x: 415, y: 36 },
                { x: 451, y: 12 },
                { x: 486, y: -13 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the headline last, from left to right",
              path: [
                { x: 20, y: 586 },
                { x: 142, y: 586 },
                { x: 265, y: 586 },
                { x: 388, y: 586 },
                { x: 510, y: 586 },
              ],
            },
          ],
        },
      ],
      source: bengaliLetterSource("হ"),
    },
  ],
];
