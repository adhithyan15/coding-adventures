// Authored kannada ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import kannada from "../../../../../learning/human-languages/data/scripts/kannada.json";

const kannadaIndependentVowelSource = (glyph: string): StrokeSource => {
  const letter = kannada.independentVowels.find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Kannada ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

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
];
