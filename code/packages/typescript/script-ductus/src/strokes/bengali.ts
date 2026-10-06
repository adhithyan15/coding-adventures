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
// the last stroke more often (20%) than the first (7%). So the bar is never
// assumed: a letter is authored only where one placement wins a majority and
// that placement covers the printed bar. ব and র draw it first; খ and থ, whose
// printed bar is only a flag beside the stem, finish with a short move right
// into it; এ ও ঞ ঃ ঁ have none.
//
// Letters whose traces split (ন ক ম ল য ত and others) are deliberately absent.
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

/** The same lookup for a sign (ঃ, ঁ), which lives in `marks`. */
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
];
