// Authored kannada ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import kannada from "../../../../../learning/human-languages/data/scripts/kannada.json";

const kannadaLetterSource = (glyph: string): StrokeSource => {
  const letter = [...kannada.letters, ...kannada.independentVowels].find(
    (candidate) => candidate.glyph === glyph,
  );
  const mark = kannada.marks.find((candidate) => candidate.mark === glyph);
  const owner = letter ?? mark;
  if (
    !owner ||
    !("strokeOrderSource" in owner) ||
    !owner.strokeOrderSource
  ) {
    throw new Error(`Kannada ${glyph} has no verified source`);
  }
  return owner.strokeOrderSource;
};

const kannadaIndependentVowelSource = kannadaLetterSource;

export const entries: DuctusEntry[] = [
  // Gopala Krishna A's 35-frame animation keeps the pencil down throughout:
  // the compact left loop flows into the broad bowl, rises through the right
  // loop, and returns left along the inner bar. These four movements preserve
  // that one-run order on the bundled Noto Sans Kannada outline.
  [
    "kannada:ಅ",
    {
      script: "kannada",
      glyph: "ಅ",
      strokes: [
        {
          segments: [
            {
              label: "turn clockwise around the compact left loop",
              path: [
                { x: 110, y: 400 },
                { x: 125, y: 460 },
                { x: 180, y: 525 },
                { x: 242, y: 528 },
                { x: 310, y: 490 },
                { x: 345, y: 430 },
                { x: 335, y: 380 },
                { x: 285, y: 330 },
                { x: 220, y: 312 },
                { x: 160, y: 330 },
                { x: 115, y: 370 },
              ],
            },
            {
              label: "sweep around the broad lower bowl",
              path: [
                { x: 115, y: 370 },
                { x: 75, y: 300 },
                { x: 90, y: 210 },
                { x: 150, y: 115 },
                { x: 270, y: 55 },
                { x: 420, y: 28 },
                { x: 550, y: 45 },
                { x: 670, y: 100 },
                { x: 750, y: 200 },
                { x: 785, y: 320 },
              ],
            },
            {
              label: "turn counterclockwise around the rounded right loop",
              path: [
                { x: 785, y: 320 },
                { x: 770, y: 410 },
                { x: 720, y: 500 },
                { x: 640, y: 525 },
                { x: 570, y: 480 },
                { x: 535, y: 420 },
                { x: 555, y: 365 },
                { x: 610, y: 325 },
                { x: 680, y: 280 },
              ],
            },
            {
              label: "return left along the inward horizontal bar",
              path: [
                { x: 680, y: 280 },
                { x: 600, y: 280 },
                { x: 500, y: 280 },
                { x: 420, y: 280 },
                { x: 375, y: 280 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಅ"),
    },
  ],
  // Gopala Krishna A's 35-frame animation writes independent vowel ಆ in two
  // runs. The first joins the compact left loop to the broad lower bowl. After
  // one lift, the second circles the right loop and returns left along the
  // inner bar. These four medians fit that order to the bundled Noto Sans
  // Kannada outline.
  [
    "kannada:ಆ",
    {
      script: "kannada",
      glyph: "ಆ",
      strokes: [
        {
          segments: [
            {
              label: "turn clockwise around the compact left loop",
              path: [
                { x: 110, y: 400 },
                { x: 125, y: 460 },
                { x: 180, y: 525 },
                { x: 242, y: 528 },
                { x: 310, y: 490 },
                { x: 345, y: 430 },
                { x: 335, y: 380 },
                { x: 285, y: 330 },
                { x: 220, y: 312 },
                { x: 160, y: 330 },
                { x: 115, y: 370 },
              ],
            },
            {
              label:
                "sweep around the broad lower bowl and finish at the upper right",
              path: [
                { x: 115, y: 370 },
                { x: 75, y: 300 },
                { x: 90, y: 210 },
                { x: 150, y: 115 },
                { x: 270, y: 55 },
                { x: 420, y: 28 },
                { x: 550, y: 45 },
                { x: 670, y: 100 },
                { x: 750, y: 200 },
                { x: 785, y: 320 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then turn clockwise around the rounded right loop",
              path: [
                { x: 535, y: 420 },
                { x: 570, y: 480 },
                { x: 640, y: 525 },
                { x: 720, y: 500 },
                { x: 770, y: 410 },
                { x: 785, y: 320 },
                { x: 750, y: 290 },
                { x: 680, y: 280 },
              ],
            },
            {
              label: "return left along the inward horizontal bar",
              path: [
                { x: 680, y: 280 },
                { x: 600, y: 280 },
                { x: 500, y: 280 },
                { x: 420, y: 280 },
                { x: 375, y: 280 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಆ"),
    },
  ],
  // Yogesh's 98-frame animation writes independent vowel ಇ in one pen-down
  // run. The middle stem is deliberately retraced: the first arch descends it,
  // the second movement climbs it again before flowing through the right arch,
  // outer descent, lower loop, and exit.
  [
    "kannada:ಇ",
    {
      script: "kannada",
      glyph: "ಇ",
      strokes: [
        {
          segments: [
            {
              label:
                "climb the left upright, turn over the first arch, and descend the middle stem",
              path: [
                { x: 82, y: 365 },
                { x: 78, y: 430 },
                { x: 120, y: 510 },
                { x: 180, y: 525 },
                { x: 245, y: 505 },
                { x: 315, y: 450 },
                { x: 335, y: 365 },
              ],
            },
            {
              label:
                "retrace the middle stem upward and turn over the second arch",
              path: [
                { x: 335, y: 365 },
                { x: 345, y: 445 },
                { x: 395, y: 510 },
                { x: 480, y: 525 },
                { x: 555, y: 485 },
                { x: 620, y: 395 },
                { x: 632, y: 310 },
              ],
            },
            {
              label:
                "descend through the broad outer curve and turn left along the base",
              path: [
                { x: 632, y: 310 },
                { x: 620, y: 225 },
                { x: 585, y: 140 },
                { x: 520, y: 70 },
                { x: 430, y: 28 },
                { x: 330, y: 20 },
                { x: 240, y: 45 },
                { x: 175, y: 95 },
                { x: 165, y: 140 },
              ],
            },
            {
              label: "close the lower loop and sweep out to the right",
              path: [
                { x: 165, y: 140 },
                { x: 205, y: 200 },
                { x: 270, y: 240 },
                { x: 325, y: 245 },
                { x: 390, y: 220 },
                { x: 470, y: 165 },
                { x: 540, y: 90 },
                { x: 610, y: 20 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಇ"),
    },
  ],
  // Gopala Krishna A's 44-frame animation writes independent long-i ಈ in
  // two runs. The rounded body flows through a retraced upper bar and high
  // curl; after one lift, the crossbar continues into the right loop and hook.
  [
    "kannada:ಈ",
    {
      script: "kannada",
      glyph: "ಈ",
      strokes: [
        {
          segments: [
            {
              label: "draw the broad rounded body and return to its upper-right join",
              path: [
                { x: 492, y: 507 },
                { x: 419, y: 511 },
                { x: 355, y: 513 },
                { x: 291, y: 511 },
                { x: 222, y: 501 },
                { x: 144, y: 392 },
                { x: 119, y: 333 },
                { x: 115, y: 206 },
                { x: 138, y: 144 },
                { x: 177, y: 92 },
                { x: 230, y: 54 },
                { x: 292, y: 33 },
                { x: 355, y: 28 },
                { x: 418, y: 33 },
                { x: 480, y: 53 },
                { x: 533, y: 92 },
                { x: 572, y: 144 },
                { x: 594, y: 206 },
                { x: 568, y: 393 },
                { x: 528, y: 443 },
                { x: 492, y: 507 },
              ],
            },
            {
              label: "sweep the upper bar left, retrace it right, and curl upward",
              path: [
                { x: 492, y: 507 },
                { x: 450, y: 500 },
                { x: 300, y: 500 },
                { x: 150, y: 500 },
                { x: 90, y: 500 },
                { x: 220, y: 500 },
                { x: 400, y: 500 },
                { x: 540, y: 500 },
                { x: 600, y: 540 },
                { x: 625, y: 600 },
                { x: 625, y: 680 },
                { x: 600, y: 740 },
                { x: 560, y: 760 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the horizontal crossbar from left to right",
              path: [
                { x: 30, y: 285 },
                { x: 220, y: 285 },
                { x: 420, y: 285 },
                { x: 620, y: 285 },
                { x: 830, y: 285 },
              ],
            },
            {
              label: "turn around the small right loop and descend into the lower hook",
              path: [
                { x: 830, y: 285 },
                { x: 900, y: 285 },
                { x: 950, y: 310 },
                { x: 970, y: 400 },
                { x: 930, y: 480 },
                { x: 850, y: 480 },
                { x: 760, y: 430 },
                { x: 754, y: 400 },
                { x: 760, y: 370 },
                { x: 776, y: 340 },
                { x: 850, y: 310 },
                { x: 880, y: 260 },
                { x: 884, y: 220 },
                { x: 912, y: 180 },
                { x: 924, y: 140 },
                { x: 917, y: 100 },
                { x: 875, y: 60 },
                { x: 862, y: 30 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಈ"),
    },
  ],
  // Gopala Krishna A's 35-frame animation writes independent vowel ಉ in one
  // run: compact upper-left loop, broad lower-left bowl, tall middle arch,
  // lower-right bowl, and the open upper terminal. These four medians fit that
  // zero-lift order to the bundled Noto Sans Kannada outline.
  [
    "kannada:ಉ",
    {
      script: "kannada",
      glyph: "ಉ",
      strokes: [
        {
          segments: [
            {
              label: "turn counterclockwise around the compact upper-left loop",
              path: [
                { x: 85, y: 375 },
                { x: 140, y: 335 },
                { x: 200, y: 313 },
                { x: 260, y: 315 },
                { x: 315, y: 345 },
                { x: 345, y: 390 },
                { x: 345, y: 440 },
                { x: 280, y: 500 },
                { x: 220, y: 530 },
                { x: 155, y: 525 },
                { x: 105, y: 470 },
                { x: 75, y: 410 },
                { x: 85, y: 375 },
              ],
            },
            {
              label:
                "descend through the left shoulder and sweep around the broad lower-left bowl",
              path: [
                { x: 85, y: 375 },
                { x: 75, y: 315 },
                { x: 80, y: 255 },
                { x: 100, y: 190 },
                { x: 140, y: 125 },
                { x: 205, y: 75 },
                { x: 275, y: 35 },
                { x: 345, y: 28 },
                { x: 410, y: 55 },
                { x: 465, y: 105 },
                { x: 505, y: 170 },
                { x: 518, y: 220 },
              ],
            },
            {
              label:
                "climb over the tall middle arch and descend into the lower-right bowl",
              path: [
                { x: 518, y: 220 },
                { x: 510, y: 300 },
                { x: 510, y: 375 },
                { x: 525, y: 440 },
                { x: 570, y: 500 },
                { x: 635, y: 525 },
                { x: 700, y: 515 },
                { x: 750, y: 475 },
                { x: 780, y: 410 },
                { x: 783, y: 335 },
                { x: 783, y: 260 },
                { x: 790, y: 190 },
                { x: 815, y: 125 },
                { x: 860, y: 75 },
                { x: 915, y: 35 },
                { x: 970, y: 28 },
              ],
            },
            {
              label:
                "sweep around the outer-right curve and finish at the open upper terminal",
              path: [
                { x: 970, y: 28 },
                { x: 1025, y: 35 },
                { x: 1080, y: 70 },
                { x: 1120, y: 125 },
                { x: 1145, y: 190 },
                { x: 1145, y: 250 },
                { x: 1125, y: 320 },
                { x: 1090, y: 390 },
                { x: 1045, y: 450 },
                { x: 975, y: 530 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಉ"),
    },
  ],
  // Gopala Krishna A's 34-frame animation writes independent vowel ಊ in one
  // run: compact upper-left spiral, broad lower-left bowl, two joined tall
  // arches, and a small lower-right spiral. These four medians fit that
  // zero-lift order to the bundled Noto Sans Kannada outline.
  [
    "kannada:ಊ",
    {
      script: "kannada",
      glyph: "ಊ",
      strokes: [
        {
          segments: [
            {
              label: "turn counterclockwise around the compact upper-left spiral",
              path: [
                { x: 65, y: 330 },
                { x: 70, y: 385 },
                { x: 95, y: 445 },
                { x: 140, y: 500 },
                { x: 205, y: 535 },
                { x: 275, y: 535 },
                { x: 330, y: 500 },
                { x: 345, y: 445 },
                { x: 340, y: 390 },
                { x: 315, y: 345 },
                { x: 280, y: 320 },
              ],
            },
            {
              label:
                "descend through the left shoulder and sweep around the broad lower-left bowl",
              path: [
                { x: 280, y: 320 },
                { x: 220, y: 305 },
                { x: 145, y: 300 },
                { x: 80, y: 285 },
                { x: 75, y: 220 },
                { x: 90, y: 155 },
                { x: 125, y: 95 },
                { x: 185, y: 50 },
                { x: 260, y: 25 },
                { x: 345, y: 30 },
                { x: 420, y: 65 },
                { x: 475, y: 125 },
                { x: 515, y: 200 },
              ],
            },
            {
              label:
                "climb over the first tall arch, descend through the middle trough, and climb over the second arch",
              path: [
                { x: 515, y: 200 },
                { x: 515, y: 280 },
                { x: 520, y: 365 },
                { x: 545, y: 445 },
                { x: 595, y: 505 },
                { x: 660, y: 530 },
                { x: 725, y: 505 },
                { x: 765, y: 445 },
                { x: 780, y: 365 },
                { x: 780, y: 280 },
                { x: 790, y: 195 },
                { x: 825, y: 115 },
                { x: 885, y: 55 },
                { x: 960, y: 30 },
                { x: 1025, y: 55 },
                { x: 1065, y: 115 },
                { x: 1065, y: 200 },
                { x: 1065, y: 285 },
                { x: 1080, y: 370 },
                { x: 1120, y: 450 },
                { x: 1180, y: 510 },
                { x: 1250, y: 535 },
                { x: 1320, y: 525 },
                { x: 1380, y: 490 },
              ],
            },
            {
              label:
                "descend the outer-right curve and curl around the small lower-right spiral",
              path: [
                { x: 1380, y: 490 },
                { x: 1430, y: 440 },
                { x: 1480, y: 370 },
                { x: 1510, y: 290 },
                { x: 1515, y: 205 },
                { x: 1500, y: 130 },
                { x: 1460, y: 70 },
                { x: 1400, y: 35 },
                { x: 1335, y: 30 },
                { x: 1270, y: 55 },
                { x: 1220, y: 105 },
                { x: 1205, y: 170 },
                { x: 1225, y: 225 },
                { x: 1270, y: 255 },
                { x: 1315, y: 245 },
                { x: 1340, y: 210 },
                { x: 1320, y: 175 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಊ"),
    },
  ],
  // Gopala Krishna A's 30-frame animation writes independent vowel ಎ in one
  // run: compact left loop, joined lower curves, rising right side, then the
  // tall outer arch finishing left. These four medians fit that zero-lift
  // order to the bundled Noto Sans Kannada outline.
  [
    "kannada:ಎ",
    {
      script: "kannada",
      glyph: "ಎ",
      strokes: [
        {
          segments: [
            {
              label: "turn clockwise around the compact left loop",
              path: [
                { x: 220, y: 185 },
                { x: 240, y: 190 },
                { x: 260, y: 210 },
                { x: 260, y: 235 },
                { x: 245, y: 260 },
                { x: 210, y: 285 },
                { x: 170, y: 295 },
                { x: 135, y: 290 },
                { x: 90, y: 270 },
                { x: 65, y: 225 },
                { x: 67, y: 165 },
                { x: 100, y: 105 },
                { x: 160, y: 55 },
                { x: 230, y: 28 },
                { x: 300, y: 35 },
                { x: 350, y: 85 },
                { x: 370, y: 150 },
                { x: 370, y: 180 },
              ],
            },
            {
              label: "sweep through the joined lower-left curve",
              path: [
                { x: 370, y: 180 },
                { x: 390, y: 145 },
                { x: 430, y: 95 },
                { x: 475, y: 55 },
                { x: 525, y: 30 },
                { x: 575, y: 28 },
              ],
            },
            {
              label:
                "turn around the rounded lower-right bowl and climb its right side",
              path: [
                { x: 575, y: 28 },
                { x: 630, y: 48 },
                { x: 680, y: 95 },
                { x: 710, y: 155 },
                { x: 710, y: 220 },
                { x: 690, y: 290 },
                { x: 650, y: 355 },
                { x: 590, y: 415 },
                { x: 515, y: 460 },
              ],
            },
            {
              label: "carry the tall outer arch over and finish to the left",
              path: [
                { x: 515, y: 460 },
                { x: 440, y: 500 },
                { x: 360, y: 525 },
                { x: 290, y: 540 },
                { x: 240, y: 540 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಎ"),
    },
  ],
  // Gopala Krishna A's 31-frame animation writes independent vowel ಏ in two
  // runs. The first carries the same compact loop and joined lower body into
  // the tall outer arch; after one lift, the second draws the small upper loop
  // from left to right. These medians fit that order to Noto Sans Kannada.
  [
    "kannada:ಏ",
    {
      script: "kannada",
      glyph: "ಏ",
      strokes: [
        {
          segments: [
            {
              label: "turn clockwise around the compact left loop",
              path: [
                { x: 220, y: 185 },
                { x: 240, y: 190 },
                { x: 260, y: 210 },
                { x: 260, y: 235 },
                { x: 245, y: 260 },
                { x: 210, y: 285 },
                { x: 170, y: 295 },
                { x: 135, y: 290 },
                { x: 90, y: 270 },
                { x: 65, y: 225 },
                { x: 67, y: 165 },
                { x: 100, y: 105 },
                { x: 160, y: 55 },
                { x: 230, y: 28 },
                { x: 300, y: 35 },
                { x: 350, y: 85 },
                { x: 370, y: 150 },
                { x: 370, y: 180 },
              ],
            },
            {
              label:
                "sweep through the joined lower curves and climb the right side",
              path: [
                { x: 370, y: 180 },
                { x: 410, y: 115 },
                { x: 475, y: 55 },
                { x: 550, y: 30 },
                { x: 625, y: 55 },
                { x: 685, y: 125 },
                { x: 710, y: 220 },
                { x: 680, y: 320 },
                { x: 610, y: 410 },
                { x: 515, y: 460 },
              ],
            },
            {
              label:
                "carry the tall outer arch over and finish at the upper left",
              path: [
                { x: 515, y: 460 },
                { x: 465, y: 480 },
                { x: 420, y: 491 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the small upper loop from left to right",
              path: [
                { x: 110, y: 565 },
                { x: 112, y: 520 },
                { x: 130, y: 480 },
                { x: 165, y: 445 },
                { x: 215, y: 420 },
                { x: 267, y: 420 },
                { x: 315, y: 430 },
                { x: 350, y: 470 },
                { x: 365, y: 525 },
                { x: 370, y: 570 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಏ"),
    },
  ],
  // Gopala Krishna A's 30-frame animation writes independent vowel ಒ in one
  // run: upper-left loop, curved descent, joined lower bowls, and the open
  // right terminal. These four medians fit that order to Noto Sans Kannada.
  [
    "kannada:ಒ",
    {
      script: "kannada",
      glyph: "ಒ",
      strokes: [
        {
          segments: [
            {
              label: "turn counterclockwise around the compact upper-left loop",
              path: [
                { x: 125, y: 365 },
                { x: 105, y: 400 },
                { x: 105, y: 445 },
                { x: 130, y: 490 },
                { x: 180, y: 525 },
                { x: 235, y: 515 },
                { x: 285, y: 480 },
                { x: 310, y: 435 },
                { x: 305, y: 400 },
              ],
            },
            {
              label:
                "descend through the curved middle into the lower-left bowl",
              path: [
                { x: 305, y: 400 },
                { x: 280, y: 350 },
                { x: 235, y: 300 },
                { x: 185, y: 260 },
                { x: 135, y: 220 },
                { x: 95, y: 175 },
                { x: 90, y: 125 },
                { x: 120, y: 75 },
                { x: 180, y: 35 },
                { x: 250, y: 30 },
                { x: 315, y: 70 },
                { x: 370, y: 145 },
              ],
            },
            {
              label: "sweep through the join and around the lower-right bowl",
              path: [
                { x: 370, y: 145 },
                { x: 405, y: 95 },
                { x: 460, y: 55 },
                { x: 525, y: 30 },
                { x: 595, y: 35 },
                { x: 655, y: 75 },
                { x: 700, y: 130 },
                { x: 705, y: 180 },
              ],
            },
            {
              label: "climb the right side and curl left at the open terminal",
              path: [
                { x: 705, y: 180 },
                { x: 695, y: 225 },
                { x: 670, y: 260 },
                { x: 640, y: 285 },
                { x: 610, y: 295 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಒ"),
    },
  ],
  // Gopala Krishna A's 35-frame animation writes independent vowel ಓ in two
  // runs. The first matches the loop, joined lower bowls, and open terminal of
  // ಒ; after one lift, the second adds the small upper flourish. These medians
  // fit that order to Noto Sans Kannada.
  [
    "kannada:ಓ",
    {
      script: "kannada",
      glyph: "ಓ",
      strokes: [
        {
          segments: [
            {
              label: "turn counterclockwise around the compact upper-left loop",
              path: [
                { x: 125, y: 365 },
                { x: 105, y: 400 },
                { x: 105, y: 445 },
                { x: 130, y: 490 },
                { x: 180, y: 525 },
                { x: 235, y: 515 },
                { x: 285, y: 480 },
                { x: 310, y: 435 },
                { x: 305, y: 400 },
              ],
            },
            {
              label: "descend through the curved middle into the lower-left bowl",
              path: [
                { x: 305, y: 400 },
                { x: 280, y: 350 },
                { x: 235, y: 300 },
                { x: 185, y: 260 },
                { x: 135, y: 220 },
                { x: 95, y: 175 },
                { x: 90, y: 125 },
                { x: 120, y: 75 },
                { x: 180, y: 35 },
                { x: 250, y: 30 },
                { x: 315, y: 70 },
                { x: 370, y: 145 },
              ],
            },
            {
              label: "sweep through the join and around the lower-right bowl",
              path: [
                { x: 370, y: 145 },
                { x: 405, y: 95 },
                { x: 460, y: 55 },
                { x: 525, y: 30 },
                { x: 595, y: 35 },
                { x: 655, y: 75 },
                { x: 700, y: 130 },
                { x: 705, y: 180 },
              ],
            },
            {
              label: "climb the right side and curl left at the open terminal",
              path: [
                { x: 705, y: 180 },
                { x: 695, y: 225 },
                { x: 670, y: 260 },
                { x: 640, y: 285 },
                { x: 610, y: 295 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep left and curl upward through the small upper flourish",
              path: [
                { x: 270, y: 550 },
                { x: 220, y: 555 },
                { x: 160, y: 570 },
                { x: 120, y: 600 },
                { x: 105, y: 640 },
                { x: 105, y: 680 },
                { x: 130, y: 710 },
                { x: 180, y: 720 },
                { x: 230, y: 720 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಓ"),
    },
  ],
  // Gopala Krishna A's 28-frame animation writes independent vowel ಐ in one
  // run: compact left spiral and lower bowl, broad right loop, then the high
  // arch finishing at the open upper-left terminal. These three medians fit
  // that zero-lift order to the bundled
  // Noto Sans Kannada outline.
  [
    "kannada:ಐ",
    {
      script: "kannada",
      glyph: "ಐ",
      strokes: [
        {
          segments: [
            {
              label:
                "turn clockwise through the compact left spiral and around its lower bowl",
              path: [
                { x: 220, y: 185 },
                { x: 240, y: 190 },
                { x: 260, y: 210 },
                { x: 260, y: 235 },
                { x: 245, y: 260 },
                { x: 210, y: 285 },
                { x: 170, y: 295 },
                { x: 135, y: 290 },
                { x: 90, y: 270 },
                { x: 65, y: 225 },
                { x: 67, y: 165 },
                { x: 100, y: 105 },
                { x: 160, y: 55 },
                { x: 230, y: 30 },
                { x: 300, y: 35 },
                { x: 340, y: 70 },
                { x: 360, y: 110 },
                { x: 375, y: 195 },
              ],
            },
            {
              label:
                "sweep through the join and around the broad right loop",
              path: [
                { x: 375, y: 195 },
                { x: 405, y: 120 },
                { x: 465, y: 70 },
                { x: 535, y: 45 },
                { x: 610, y: 55 },
                { x: 675, y: 105 },
                { x: 720, y: 180 },
                { x: 735, y: 270 },
                { x: 720, y: 365 },
                { x: 680, y: 445 },
                { x: 615, y: 500 },
                { x: 545, y: 525 },
                { x: 485, y: 510 },
                { x: 440, y: 475 },
                { x: 417, y: 420 },
              ],
            },
            {
              label:
                "carry the high arch leftward and finish at the open upper-left terminal",
              path: [
                { x: 417, y: 420 },
                { x: 400, y: 430 },
                { x: 385, y: 440 },
                { x: 365, y: 445 },
                { x: 350, y: 455 },
                { x: 337, y: 470 },
                { x: 320, y: 490 },
                { x: 300, y: 510 },
                { x: 280, y: 520 },
                { x: 240, y: 525 },
                { x: 210, y: 525 },
                { x: 150, y: 490 },
                { x: 100, y: 430 },
                { x: 75, y: 370 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಐ"),
    },
  ],
  // Gopala Krishna A's 59-frame animation writes independent vowel ಋ in
  // three runs. The first joins the upper-left spiral, lower-left spiral, and
  // rounded middle bowl. After one lift, the second draws the inward bar and
  // high hook. After another lift, the third circles the open right bowl.
  // These seven medians fit that attested order to Noto Sans Kannada.
  [
    "kannada:ಋ",
    {
      script: "kannada",
      glyph: "ಋ",
      strokes: [
        {
          segments: [
            {
              label: "turn clockwise around the compact upper-left spiral",
              path: [
                { x: 245, y: 440 },
                { x: 240, y: 485 },
                { x: 210, y: 525 },
                { x: 155, y: 540 },
                { x: 100, y: 525 },
                { x: 65, y: 490 },
                { x: 60, y: 445 },
                { x: 75, y: 415 },
                { x: 115, y: 390 },
                { x: 150, y: 365 },
                { x: 155, y: 370 },
                { x: 205, y: 375 },
                { x: 225, y: 395 },
                { x: 245, y: 440 },
              ],
            },
            {
              label:
                "descend through the outer curve and curl around the lower-left spiral",
              path: [
                { x: 245, y: 440 },
                { x: 295, y: 405 },
                { x: 295, y: 445 },
                { x: 300, y: 450 },
                { x: 335, y: 395 },
                { x: 355, y: 320 },
                { x: 355, y: 315 },
                { x: 370, y: 240 },
                { x: 375, y: 155 },
                { x: 350, y: 100 },
                { x: 300, y: 70 },
                { x: 250, y: 50 },
                { x: 190, y: 40 },
                { x: 120, y: 50 },
                { x: 70, y: 90 },
                { x: 60, y: 140 },
                { x: 90, y: 180 },
                { x: 145, y: 195 },
                { x: 200, y: 185 },
                { x: 230, y: 160 },
                { x: 235, y: 150 },
                { x: 240, y: 110 },
              ],
            },
            {
              label:
                "sweep through the join and around the rounded middle bowl",
              path: [
                { x: 240, y: 110 },
                { x: 275, y: 75 },
                { x: 355, y: 80 },
                { x: 385, y: 75 },
                { x: 445, y: 55 },
                { x: 535, y: 70 },
                { x: 625, y: 60 },
                { x: 650, y: 85 },
                { x: 675, y: 105 },
                { x: 700, y: 145 },
                { x: 705, y: 175 },
                { x: 710, y: 235 },
                { x: 685, y: 330 },
                { x: 680, y: 335 },
                { x: 660, y: 445 },
                { x: 625, y: 535 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the inward bar from left to right",
              path: [
                { x: 455, y: 510 },
                { x: 500, y: 515 },
                { x: 555, y: 515 },
                { x: 610, y: 525 },
                { x: 650, y: 555 },
              ],
            },
            {
              label: "curl upward into the high hook",
              path: [
                { x: 650, y: 555 },
                { x: 680, y: 570 },
                { x: 695, y: 590 },
                { x: 695, y: 595 },
                { x: 700, y: 615 },
                { x: 700, y: 660 },
                { x: 685, y: 725 },
                { x: 655, y: 760 },
                { x: 640, y: 750 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then sweep rightward around the lower bowl",
              path: [
                { x: 690, y: 105 },
                { x: 735, y: 65 },
                { x: 755, y: 75 },
                { x: 810, y: 45 },
                { x: 900, y: 55 },
                { x: 990, y: 70 },
                { x: 995, y: 75 },
                { x: 1020, y: 85 },
                { x: 1050, y: 115 },
                { x: 1065, y: 145 },
              ],
            },
            {
              label:
                "climb the outer side and finish at the open upper terminal",
              path: [
                { x: 1065, y: 145 },
                { x: 1075, y: 215 },
                { x: 1065, y: 295 },
                { x: 1060, y: 305 },
                { x: 1035, y: 390 },
                { x: 1005, y: 490 },
                { x: 960, y: 535 },
                { x: 925, y: 560 },
                { x: 910, y: 540 },
              ],
            },
          ],
        },
      ],
      source: kannadaIndependentVowelSource("ಋ"),
    },
  ],
  // The source animation writes ಅಃ, so the standalone U+0C83 record owns only
  // its final two pen-down runs: the upper closed loop, then the lower closed
  // loop after one lift. These medians fit those runs to Noto Sans Kannada.
  [
    "kannada:ಃ",
    {
      script: "kannada",
      glyph: "ಃ",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper dot as a closed loop",
              path: [
                { x: 153, y: 28 },
                { x: 210, y: 48 },
                { x: 235, y: 105 },
                { x: 210, y: 165 },
                { x: 153, y: 195 },
                { x: 92, y: 165 },
                { x: 68, y: 105 },
                { x: 92, y: 48 },
                { x: 153, y: 28 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower dot as a closed loop",
              path: [
                { x: 153, y: 290 },
                { x: 210, y: 312 },
                { x: 235, y: 382 },
                { x: 210, y: 450 },
                { x: 153, y: 478 },
                { x: 92, y: 450 },
                { x: 68, y: 382 },
                { x: 92, y: 312 },
                { x: 153, y: 290 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ಃ"),
    },
  ],
  // Base consonants. Each bare consonant row (role "syllable", inherent a)
  // cites the same Gopala Krishna A series as the vowels above. The frames
  // were read from an identical mirror copy of each Commons GIF; the slug for
  // dental ತ and ದ is "tha" and "dha" in that series ("ta" and "da" animate
  // retroflex ಟ and ಡ), so the file names below were checked against the
  // drawn glyph rather than assumed from the slug.
  // Gopala Krishna A's 34-frame Kannada-alphabet-na.gif writes ನ in two runs.
  // The body starts at the lower tail, climbs around the left bowl, slants down
  // and climbs the right side; after one lift, the top bar runs rightward into
  // the hook. The medians follow the bundled Noto Sans Kannada outline's
  // skeleton, where the right side meets the bar instead of arching over.
  [
    "kannada:ನ",
    {
      script: "kannada",
      glyph: "ನ",
      strokes: [
        {
          segments: [
            {
              label: "rise from the tail around the left bowl",
              path: [
                { x: 118, y: 14 },
                { x: 103, y: 38 },
                { x: 90, y: 68 },
                { x: 80, y: 102 },
                { x: 77, y: 138 },
                { x: 80, y: 173 },
                { x: 92, y: 207 },
                { x: 112, y: 234 },
                { x: 142, y: 251 },
                { x: 177, y: 258 },
                { x: 213, y: 256 },
                { x: 248, y: 248 },
              ],
            },
            {
              label: "slant down into the right bowl",
              path: [
                { x: 248, y: 248 },
                { x: 273, y: 222 },
                { x: 295, y: 194 },
                { x: 316, y: 164 },
                { x: 335, y: 134 },
                { x: 355, y: 105 },
                { x: 378, y: 78 },
                { x: 405, y: 55 },
                { x: 437, y: 40 },
                { x: 470, y: 29 },
              ],
            },
            {
              label: "climb the right side to the top bar",
              path: [
                { x: 470, y: 29 },
                { x: 509, y: 34 },
                { x: 545, y: 44 },
                { x: 576, y: 63 },
                { x: 598, y: 92 },
                { x: 613, y: 126 },
                { x: 621, y: 163 },
                { x: 623, y: 202 },
                { x: 620, y: 241 },
                { x: 613, y: 278 },
                { x: 601, y: 314 },
                { x: 585, y: 348 },
                { x: 564, y: 380 },
                { x: 539, y: 410 },
                { x: 514, y: 439 },
                { x: 493, y: 470 },
                { x: 476, y: 503 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the top bar rightward",
              path: [
                { x: 55, y: 516 },
                { x: 94, y: 516 },
                { x: 134, y: 516 },
                { x: 174, y: 516 },
                { x: 213, y: 516 },
                { x: 252, y: 516 },
                { x: 292, y: 516 },
                { x: 332, y: 516 },
                { x: 371, y: 516 },
                { x: 410, y: 516 },
                { x: 450, y: 516 },
              ],
            },
            {
              label: "curl up into the hook",
              path: [
                { x: 450, y: 516 },
                { x: 479, y: 512 },
                { x: 511, y: 514 },
                { x: 546, y: 523 },
                { x: 579, y: 539 },
                { x: 604, y: 563 },
                { x: 618, y: 594 },
                { x: 620, y: 630 },
                { x: 614, y: 666 },
                { x: 601, y: 701 },
                { x: 583, y: 734 },
                { x: 562, y: 766 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ನ"),
    },
  ],
  // Gopala Krishna A's 43-frame Kannada-alphabet-tha.gif (the uploader's
  // slug for dental ತ) writes ತ in two runs: the broad bowl, the arc over the
  // top, the small inner loop and the rise to the bar; then, after one lift,
  // the top bar and hook. Noto merges the arc with the loop's top, so the
  // loop's right side is retraced on the way up.
  [
    "kannada:ತ",
    {
      script: "kannada",
      glyph: "ತ",
      strokes: [
        {
          segments: [
            {
              label: "sweep down and around the broad bowl",
              path: [
                { x: 78, y: 246 },
                { x: 79, y: 214 },
                { x: 81, y: 180 },
                { x: 89, y: 145 },
                { x: 106, y: 112 },
                { x: 130, y: 84 },
                { x: 160, y: 61 },
                { x: 194, y: 46 },
                { x: 231, y: 37 },
                { x: 269, y: 32 },
                { x: 309, y: 30 },
                { x: 348, y: 32 },
                { x: 387, y: 37 },
                { x: 424, y: 46 },
                { x: 458, y: 61 },
                { x: 489, y: 82 },
                { x: 514, y: 110 },
                { x: 531, y: 143 },
                { x: 540, y: 179 },
                { x: 541, y: 216 },
                { x: 533, y: 252 },
                { x: 515, y: 280 },
                { x: 487, y: 301 },
              ],
            },
            {
              label: "turn left over the top into the inner loop",
              path: [
                { x: 487, y: 301 },
                { x: 477, y: 330 },
                { x: 456, y: 349 },
                { x: 425, y: 358 },
                { x: 387, y: 363 },
                { x: 348, y: 364 },
                { x: 311, y: 358 },
                { x: 280, y: 342 },
                { x: 253, y: 319 },
              ],
            },
            {
              label: "close the small loop and rise to the top bar",
              path: [
                { x: 253, y: 319 },
                { x: 255, y: 283 },
                { x: 268, y: 253 },
                { x: 295, y: 233 },
                { x: 330, y: 224 },
                { x: 366, y: 224 },
                { x: 402, y: 233 },
                { x: 434, y: 250 },
                { x: 460, y: 275 },
                { x: 476, y: 305 },
                { x: 482, y: 340 },
                { x: 480, y: 376 },
                { x: 471, y: 413 },
                { x: 458, y: 447 },
                { x: 446, y: 481 },
                { x: 436, y: 514 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the top bar rightward",
              path: [
                { x: 55, y: 516 },
                { x: 96, y: 516 },
                { x: 136, y: 516 },
                { x: 177, y: 516 },
                { x: 217, y: 516 },
                { x: 258, y: 516 },
                { x: 298, y: 516 },
                { x: 339, y: 516 },
                { x: 379, y: 516 },
                { x: 420, y: 516 },
              ],
            },
            {
              label: "curl up into the hook",
              path: [
                { x: 420, y: 516 },
                { x: 453, y: 521 },
                { x: 486, y: 534 },
                { x: 514, y: 556 },
                { x: 531, y: 587 },
                { x: 536, y: 624 },
                { x: 531, y: 662 },
                { x: 519, y: 698 },
                { x: 505, y: 733 },
                { x: 492, y: 768 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ತ"),
    },
  ],
  // Gopala Krishna A's 39-frame Kannada-alphabet-dha.gif (the uploader's
  // slug for dental ದ) writes the whole bowl in one run: left lobe, up into
  // the middle point, right lobe, then back left across the top. After one
  // lift the top bar and hook follow. Noto fuses the bowl's top into the bar,
  // so the closing movement runs along the lower edge of that ink.
  [
    "kannada:ದ",
    {
      script: "kannada",
      glyph: "ದ",
      strokes: [
        {
          segments: [
            {
              label: "go down the left side into the left lobe",
              path: [
                { x: 212, y: 500 },
                { x: 195, y: 469 },
                { x: 174, y: 439 },
                { x: 150, y: 410 },
                { x: 129, y: 380 },
                { x: 111, y: 348 },
                { x: 98, y: 314 },
                { x: 90, y: 278 },
                { x: 87, y: 240 },
                { x: 86, y: 201 },
                { x: 90, y: 163 },
                { x: 98, y: 127 },
                { x: 113, y: 94 },
                { x: 136, y: 66 },
                { x: 165, y: 46 },
                { x: 199, y: 36 },
                { x: 235, y: 34 },
                { x: 271, y: 40 },
                { x: 304, y: 55 },
                { x: 331, y: 80 },
                { x: 353, y: 111 },
                { x: 371, y: 146 },
                { x: 387, y: 182 },
              ],
            },
            {
              label: "drop from the point around the right lobe",
              path: [
                { x: 387, y: 182 },
                { x: 392, y: 154 },
                { x: 397, y: 129 },
                { x: 408, y: 104 },
                { x: 431, y: 79 },
                { x: 460, y: 56 },
                { x: 493, y: 41 },
                { x: 529, y: 34 },
                { x: 566, y: 36 },
                { x: 600, y: 47 },
                { x: 629, y: 67 },
                { x: 651, y: 96 },
                { x: 666, y: 129 },
                { x: 674, y: 166 },
                { x: 677, y: 203 },
                { x: 677, y: 241 },
                { x: 673, y: 279 },
                { x: 664, y: 315 },
                { x: 652, y: 349 },
                { x: 634, y: 382 },
                { x: 612, y: 413 },
                { x: 589, y: 443 },
                { x: 568, y: 473 },
                { x: 550, y: 505 },
              ],
            },
            {
              label: "close the bowl leftward along the top",
              path: [
                { x: 550, y: 505 },
                { x: 508, y: 504 },
                { x: 466, y: 503 },
                { x: 423, y: 502 },
                { x: 381, y: 502 },
                { x: 339, y: 501 },
                { x: 296, y: 500 },
                { x: 254, y: 499 },
                { x: 212, y: 498 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the top bar rightward",
              path: [
                { x: 55, y: 516 },
                { x: 95, y: 516 },
                { x: 134, y: 516 },
                { x: 174, y: 516 },
                { x: 213, y: 516 },
                { x: 253, y: 516 },
                { x: 292, y: 516 },
                { x: 332, y: 516 },
                { x: 372, y: 516 },
                { x: 411, y: 516 },
                { x: 451, y: 516 },
                { x: 490, y: 516 },
                { x: 530, y: 516 },
              ],
            },
            {
              label: "curl up into the hook",
              path: [
                { x: 530, y: 516 },
                { x: 558, y: 515 },
                { x: 591, y: 520 },
                { x: 624, y: 533 },
                { x: 652, y: 556 },
                { x: 669, y: 587 },
                { x: 674, y: 624 },
                { x: 670, y: 661 },
                { x: 658, y: 698 },
                { x: 639, y: 732 },
                { x: 618, y: 766 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ದ"),
    },
  ],
  // Gopala Krishna A's 36-frame Kannada-alphabet-ra.gif closes the round
  // bowl counterclockwise from its upper left, then lifts once for the top bar
  // and hook. Noto fuses the bowl's top into the bar, so the closing movement
  // runs along the lower edge of that ink.
  [
    "kannada:ರ",
    {
      script: "kannada",
      glyph: "ರ",
      strokes: [
        {
          segments: [
            {
              label: "go down the left side and round the base",
              path: [
                { x: 184, y: 500 },
                { x: 169, y: 469 },
                { x: 149, y: 437 },
                { x: 128, y: 406 },
                { x: 110, y: 374 },
                { x: 97, y: 339 },
                { x: 90, y: 302 },
                { x: 87, y: 264 },
                { x: 89, y: 226 },
                { x: 96, y: 189 },
                { x: 109, y: 154 },
                { x: 128, y: 122 },
                { x: 153, y: 94 },
                { x: 182, y: 70 },
                { x: 215, y: 52 },
                { x: 250, y: 40 },
                { x: 286, y: 33 },
                { x: 325, y: 31 },
                { x: 363, y: 33 },
                { x: 400, y: 39 },
                { x: 436, y: 49 },
              ],
            },
            {
              label: "climb the right side and close leftward",
              path: [
                { x: 436, y: 49 },
                { x: 468, y: 68 },
                { x: 498, y: 92 },
                { x: 522, y: 120 },
                { x: 541, y: 152 },
                { x: 554, y: 187 },
                { x: 562, y: 224 },
                { x: 565, y: 262 },
                { x: 563, y: 300 },
                { x: 555, y: 337 },
                { x: 543, y: 373 },
                { x: 525, y: 405 },
                { x: 503, y: 436 },
                { x: 480, y: 465 },
                { x: 454, y: 488 },
                { x: 422, y: 500 },
                { x: 385, y: 502 },
                { x: 345, y: 502 },
                { x: 305, y: 501 },
                { x: 264, y: 499 },
                { x: 224, y: 498 },
                { x: 184, y: 497 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the top bar rightward",
              path: [
                { x: 55, y: 516 },
                { x: 94, y: 516 },
                { x: 132, y: 516 },
                { x: 170, y: 516 },
                { x: 209, y: 516 },
                { x: 248, y: 516 },
                { x: 286, y: 516 },
                { x: 324, y: 516 },
                { x: 363, y: 516 },
                { x: 402, y: 516 },
                { x: 440, y: 516 },
              ],
            },
            {
              label: "curl up into the hook",
              path: [
                { x: 440, y: 516 },
                { x: 468, y: 518 },
                { x: 499, y: 526 },
                { x: 527, y: 543 },
                { x: 549, y: 569 },
                { x: 560, y: 602 },
                { x: 561, y: 638 },
                { x: 554, y: 673 },
                { x: 540, y: 706 },
                { x: 523, y: 737 },
                { x: 503, y: 768 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ರ"),
    },
  ],
  // Gopala Krishna A's 44-frame Kannada-alphabet-ka.gif writes ಕ in four
  // runs: the round bowl, the lower bar, the short link rising from it, and
  // the upper bar with its hook, so three lifts. The animation's link bulges
  // right; Noto draws it as a straight waist, which the median follows.
  [
    "kannada:ಕ",
    {
      script: "kannada",
      glyph: "ಕ",
      strokes: [
        {
          segments: [
            {
              label: "go down the left side and round the base",
              path: [
                { x: 166, y: 332 },
                { x: 148, y: 299 },
                { x: 131, y: 265 },
                { x: 116, y: 231 },
                { x: 107, y: 195 },
                { x: 106, y: 157 },
                { x: 115, y: 122 },
                { x: 133, y: 90 },
                { x: 160, y: 65 },
                { x: 193, y: 47 },
                { x: 229, y: 36 },
                { x: 267, y: 31 },
                { x: 306, y: 31 },
                { x: 345, y: 35 },
                { x: 382, y: 43 },
              ],
            },
            {
              label: "climb the right side and close leftward",
              path: [
                { x: 382, y: 43 },
                { x: 413, y: 63 },
                { x: 440, y: 89 },
                { x: 458, y: 121 },
                { x: 467, y: 157 },
                { x: 466, y: 195 },
                { x: 458, y: 231 },
                { x: 443, y: 265 },
                { x: 423, y: 296 },
                { x: 399, y: 319 },
                { x: 367, y: 330 },
                { x: 328, y: 332 },
                { x: 288, y: 331 },
                { x: 247, y: 330 },
                { x: 207, y: 329 },
                { x: 166, y: 328 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower bar rightward",
              path: [
                { x: 55, y: 344 },
                { x: 95, y: 344 },
                { x: 134, y: 344 },
                { x: 174, y: 344 },
                { x: 214, y: 344 },
                { x: 254, y: 344 },
                { x: 294, y: 344 },
                { x: 333, y: 344 },
                { x: 373, y: 344 },
                { x: 413, y: 344 },
                { x: 452, y: 344 },
                { x: 492, y: 344 },
                { x: 532, y: 344 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the short link upward",
              path: [
                { x: 304, y: 352 },
                { x: 304, y: 388 },
                { x: 303, y: 426 },
                { x: 302, y: 467 },
                { x: 301, y: 508 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the upper bar rightward",
              path: [
                { x: 55, y: 516 },
                { x: 97, y: 516 },
                { x: 139, y: 516 },
                { x: 181, y: 516 },
                { x: 222, y: 516 },
                { x: 264, y: 516 },
                { x: 306, y: 516 },
                { x: 348, y: 516 },
                { x: 390, y: 516 },
              ],
            },
            {
              label: "curl up into the hook",
              path: [
                { x: 390, y: 516 },
                { x: 425, y: 525 },
                { x: 457, y: 540 },
                { x: 480, y: 564 },
                { x: 493, y: 597 },
                { x: 496, y: 632 },
                { x: 489, y: 669 },
                { x: 477, y: 703 },
                { x: 461, y: 736 },
                { x: 443, y: 768 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ಕ"),
    },
  ],
  // Gopala Krishna A's 37-frame Kannada-alphabet-ga.gif climbs the left leg
  // from its foot, arches over and comes down the right leg in one run, then
  // lifts once for the top bar and hook. Noto fuses the arch's top into the
  // bar, so the arch runs along the lower edge of that ink.
  [
    "kannada:ಗ",
    {
      script: "kannada",
      glyph: "ಗ",
      strokes: [
        {
          segments: [
            {
              label: "climb the left leg into the arch",
              path: [
                { x: 91, y: 14 },
                { x: 92, y: 54 },
                { x: 94, y: 94 },
                { x: 96, y: 135 },
                { x: 98, y: 175 },
                { x: 101, y: 215 },
                { x: 106, y: 255 },
                { x: 113, y: 293 },
                { x: 124, y: 330 },
                { x: 138, y: 365 },
                { x: 156, y: 399 },
                { x: 176, y: 432 },
                { x: 196, y: 465 },
                { x: 213, y: 499 },
              ],
            },
            {
              label: "arch over and go down the right leg",
              path: [
                { x: 213, y: 499 },
                { x: 253, y: 498 },
                { x: 293, y: 498 },
                { x: 333, y: 498 },
                { x: 372, y: 498 },
                { x: 407, y: 493 },
                { x: 434, y: 478 },
                { x: 455, y: 453 },
                { x: 476, y: 423 },
                { x: 496, y: 391 },
                { x: 513, y: 358 },
                { x: 526, y: 323 },
                { x: 535, y: 287 },
                { x: 542, y: 250 },
                { x: 546, y: 211 },
                { x: 548, y: 172 },
                { x: 550, y: 133 },
                { x: 551, y: 94 },
                { x: 551, y: 54 },
                { x: 549, y: 14 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the top bar rightward",
              path: [
                { x: 55, y: 516 },
                { x: 95, y: 516 },
                { x: 135, y: 516 },
                { x: 175, y: 516 },
                { x: 215, y: 516 },
                { x: 255, y: 516 },
                { x: 295, y: 516 },
                { x: 335, y: 516 },
                { x: 375, y: 516 },
                { x: 415, y: 516 },
              ],
            },
            {
              label: "curl up into the hook",
              path: [
                { x: 415, y: 516 },
                { x: 447, y: 518 },
                { x: 479, y: 525 },
                { x: 510, y: 540 },
                { x: 534, y: 565 },
                { x: 547, y: 597 },
                { x: 549, y: 632 },
                { x: 543, y: 668 },
                { x: 531, y: 701 },
                { x: 516, y: 734 },
                { x: 500, y: 766 },
              ],
            },
          ],
        },
      ],
      source: kannadaLetterSource("ಗ"),
    },
  ],
];
