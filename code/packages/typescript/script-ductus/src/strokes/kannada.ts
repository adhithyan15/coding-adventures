// Authored kannada ductus records. This is the stable source-ownership boundary.

import type { LetterDuctus, Point, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import kannada from "../../../../../learning/human-languages/data/scripts/kannada.json";

const kannadaIndependentVowelSource = (glyph: string): StrokeSource => {
  const letter = kannada.independentVowels.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Kannada ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // Gopala Krishna A's 35-frame animation keeps the pencil down throughout:
  // the compact left loop flows into the broad bowl, rises through the right
  // loop, and returns left along the inner bar. These four movements preserve
  // that one-run order on the bundled Noto Sans Kannada outline.
    ["kannada:ಅ", {
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
  }],
  // Gopala Krishna A's 35-frame animation writes independent vowel ಆ in two
  // runs. The first joins the compact left loop to the broad lower bowl. After
  // one lift, the second circles the right loop and returns left along the
  // inner bar. These four medians fit that order to the bundled Noto Sans
  // Kannada outline.
    ["kannada:ಆ", {
    script: "kannada",
    glyph: "ಆ",
    strokes: [
      {
        segments: [
          {
            label: "turn clockwise around the compact left loop",
            path: [
              { x: 110, y: 400 }, { x: 125, y: 460 }, { x: 180, y: 525 },
              { x: 242, y: 528 }, { x: 310, y: 490 }, { x: 345, y: 430 },
              { x: 335, y: 380 }, { x: 285, y: 330 }, { x: 220, y: 312 },
              { x: 160, y: 330 }, { x: 115, y: 370 },
            ],
          },
          {
            label: "sweep around the broad lower bowl and finish at the upper right",
            path: [
              { x: 115, y: 370 }, { x: 75, y: 300 }, { x: 90, y: 210 },
              { x: 150, y: 115 }, { x: 270, y: 55 }, { x: 420, y: 28 },
              { x: 550, y: 45 }, { x: 670, y: 100 }, { x: 750, y: 200 },
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
              { x: 535, y: 420 }, { x: 570, y: 480 }, { x: 640, y: 525 },
              { x: 720, y: 500 }, { x: 770, y: 410 }, { x: 785, y: 320 },
              { x: 750, y: 290 }, { x: 680, y: 280 },
            ],
          },
          {
            label: "return left along the inward horizontal bar",
            path: [
              { x: 680, y: 280 }, { x: 600, y: 280 }, { x: 500, y: 280 },
              { x: 420, y: 280 }, { x: 375, y: 280 },
            ],
          },
        ],
      },
    ],
    source: kannadaIndependentVowelSource("ಆ"),
  }],
  // Yogesh's 98-frame animation writes independent vowel ಇ in one pen-down
  // run. The middle stem is deliberately retraced: the first arch descends it,
  // the second movement climbs it again before flowing through the right arch,
  // outer descent, lower loop, and exit.
    ["kannada:ಇ", {
    script: "kannada",
    glyph: "ಇ",
    strokes: [
      {
        segments: [
          {
            label: "climb the left upright, turn over the first arch, and descend the middle stem",
            path: [
              { x: 82, y: 365 }, { x: 78, y: 430 }, { x: 120, y: 510 },
              { x: 180, y: 525 }, { x: 245, y: 505 }, { x: 315, y: 450 },
              { x: 335, y: 365 },
            ],
          },
          {
            label: "retrace the middle stem upward and turn over the second arch",
            path: [
              { x: 335, y: 365 }, { x: 345, y: 445 }, { x: 395, y: 510 },
              { x: 480, y: 525 }, { x: 555, y: 485 }, { x: 620, y: 395 },
              { x: 632, y: 310 },
            ],
          },
          {
            label: "descend through the broad outer curve and turn left along the base",
            path: [
              { x: 632, y: 310 }, { x: 620, y: 225 }, { x: 585, y: 140 },
              { x: 520, y: 70 }, { x: 430, y: 28 }, { x: 330, y: 20 },
              { x: 240, y: 45 }, { x: 175, y: 95 }, { x: 165, y: 140 },
            ],
          },
          {
            label: "close the lower loop and sweep out to the right",
            path: [
              { x: 165, y: 140 }, { x: 205, y: 200 }, { x: 270, y: 240 },
              { x: 325, y: 245 }, { x: 390, y: 220 }, { x: 470, y: 165 },
              { x: 540, y: 90 }, { x: 610, y: 20 },
            ],
          },
        ],
      },
    ],
    source: kannadaIndependentVowelSource("ಇ"),
  }],
  // Gopala Krishna A's 35-frame animation writes independent vowel ಉ in one
  // run: compact upper-left loop, broad lower-left bowl, tall middle arch,
  // lower-right bowl, and the open upper terminal. These four medians fit that
  // zero-lift order to the bundled Noto Sans Kannada outline.
    ["kannada:ಉ", {
    script: "kannada",
    glyph: "ಉ",
    strokes: [
      {
        segments: [
          {
            label: "turn counterclockwise around the compact upper-left loop",
            path: [
              { x: 85, y: 375 }, { x: 140, y: 335 }, { x: 200, y: 313 },
              { x: 260, y: 315 }, { x: 315, y: 345 }, { x: 345, y: 390 },
              { x: 345, y: 440 }, { x: 280, y: 500 }, { x: 220, y: 530 },
              { x: 155, y: 525 }, { x: 105, y: 470 }, { x: 75, y: 410 },
              { x: 85, y: 375 },
            ],
          },
          {
            label: "descend through the left shoulder and sweep around the broad lower-left bowl",
            path: [
              { x: 85, y: 375 }, { x: 75, y: 315 }, { x: 80, y: 255 },
              { x: 100, y: 190 }, { x: 140, y: 125 }, { x: 205, y: 75 },
              { x: 275, y: 35 }, { x: 345, y: 28 }, { x: 410, y: 55 },
              { x: 465, y: 105 }, { x: 505, y: 170 }, { x: 518, y: 220 },
            ],
          },
          {
            label: "climb over the tall middle arch and descend into the lower-right bowl",
            path: [
              { x: 518, y: 220 }, { x: 510, y: 300 }, { x: 510, y: 375 },
              { x: 525, y: 440 }, { x: 570, y: 500 }, { x: 635, y: 525 },
              { x: 700, y: 515 }, { x: 750, y: 475 }, { x: 780, y: 410 },
              { x: 783, y: 335 }, { x: 783, y: 260 }, { x: 790, y: 190 },
              { x: 815, y: 125 }, { x: 860, y: 75 }, { x: 915, y: 35 },
              { x: 970, y: 28 },
            ],
          },
          {
            label: "sweep around the outer-right curve and finish at the open upper terminal",
            path: [
              { x: 970, y: 28 }, { x: 1025, y: 35 }, { x: 1080, y: 70 },
              { x: 1120, y: 125 }, { x: 1145, y: 190 }, { x: 1145, y: 250 },
              { x: 1125, y: 320 }, { x: 1090, y: 390 }, { x: 1045, y: 450 },
              { x: 975, y: 530 },
            ],
          },
        ],
      },
    ],
    source: kannadaIndependentVowelSource("ಉ"),
  }],
  // Gopala Krishna A's 30-frame animation writes independent vowel ಎ in one
  // run: compact left loop, joined lower curves, rising right side, then the
  // tall outer arch finishing left. These four medians fit that zero-lift
  // order to the bundled Noto Sans Kannada outline.
    ["kannada:ಎ", {
    script: "kannada",
    glyph: "ಎ",
    strokes: [
      {
        segments: [
          {
            label: "turn clockwise around the compact left loop",
            path: [
              { x: 220, y: 185 }, { x: 240, y: 190 }, { x: 260, y: 210 },
              { x: 260, y: 235 }, { x: 245, y: 260 }, { x: 210, y: 285 },
              { x: 170, y: 295 }, { x: 135, y: 290 }, { x: 90, y: 270 }, { x: 65, y: 225 },
              { x: 67, y: 165 }, { x: 100, y: 105 }, { x: 160, y: 55 },
              { x: 230, y: 28 }, { x: 300, y: 35 }, { x: 350, y: 85 },
              { x: 370, y: 150 }, { x: 370, y: 180 },
            ],
          },
          {
            label: "sweep through the joined lower-left curve",
            path: [
              { x: 370, y: 180 }, { x: 390, y: 145 }, { x: 430, y: 95 },
              { x: 475, y: 55 }, { x: 525, y: 30 }, { x: 575, y: 28 },
            ],
          },
          {
            label: "turn around the rounded lower-right bowl and climb its right side",
            path: [
              { x: 575, y: 28 }, { x: 630, y: 48 }, { x: 680, y: 95 },
              { x: 710, y: 155 }, { x: 710, y: 220 }, { x: 690, y: 290 },
              { x: 650, y: 355 }, { x: 590, y: 415 }, { x: 515, y: 460 },
            ],
          },
          {
            label: "carry the tall outer arch over and finish to the left",
            path: [
              { x: 515, y: 460 }, { x: 440, y: 500 }, { x: 360, y: 525 },
              { x: 290, y: 540 }, { x: 240, y: 540 },
            ],
          },
        ],
      },
    ],
    source: kannadaIndependentVowelSource("ಎ"),
  }],
  // Gopala Krishna A's 31-frame animation writes independent vowel ಏ in two
  // runs. The first carries the same compact loop and joined lower body into
  // the tall outer arch; after one lift, the second draws the small upper loop
  // from left to right. These medians fit that order to Noto Sans Kannada.
    ["kannada:ಏ", {
    script: "kannada",
    glyph: "ಏ",
    strokes: [
      {
        segments: [
          {
            label: "turn clockwise around the compact left loop",
            path: [
              { x: 220, y: 185 }, { x: 240, y: 190 }, { x: 260, y: 210 },
              { x: 260, y: 235 }, { x: 245, y: 260 }, { x: 210, y: 285 },
              { x: 170, y: 295 }, { x: 135, y: 290 }, { x: 90, y: 270 }, { x: 65, y: 225 },
              { x: 67, y: 165 }, { x: 100, y: 105 }, { x: 160, y: 55 },
              { x: 230, y: 28 }, { x: 300, y: 35 }, { x: 350, y: 85 },
              { x: 370, y: 150 }, { x: 370, y: 180 },
            ],
          },
          {
            label: "sweep through the joined lower curves and climb the right side",
            path: [
              { x: 370, y: 180 }, { x: 410, y: 115 }, { x: 475, y: 55 },
              { x: 550, y: 30 }, { x: 625, y: 55 }, { x: 685, y: 125 },
              { x: 710, y: 220 }, { x: 680, y: 320 }, { x: 610, y: 410 },
              { x: 515, y: 460 },
            ],
          },
          {
            label: "carry the tall outer arch over and finish at the upper left",
            path: [
              { x: 515, y: 460 }, { x: 465, y: 480 }, { x: 420, y: 491 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "draw the small upper loop from left to right",
            path: [
              { x: 110, y: 565 }, { x: 112, y: 520 }, { x: 130, y: 480 },
              { x: 165, y: 445 }, { x: 215, y: 420 }, { x: 267, y: 420 },
              { x: 315, y: 430 }, { x: 350, y: 470 }, { x: 365, y: 525 },
              { x: 370, y: 570 },
            ],
          },
        ],
      },
    ],
    source: kannadaIndependentVowelSource("ಏ"),
  }],
  // Gopala Krishna A's 30-frame animation writes independent vowel ಒ in one
  // run: upper-left loop, curved descent, joined lower bowls, and the open
  // right terminal. These four medians fit that order to Noto Sans Kannada.
    ["kannada:ಒ", {
    script: "kannada",
    glyph: "ಒ",
    strokes: [
      {
        segments: [
          {
            label: "turn counterclockwise around the compact upper-left loop",
            path: [
              { x: 125, y: 365 }, { x: 105, y: 400 }, { x: 105, y: 445 },
              { x: 130, y: 490 }, { x: 180, y: 525 }, { x: 235, y: 515 },
              { x: 285, y: 480 }, { x: 310, y: 435 }, { x: 305, y: 400 },
            ],
          },
          {
            label: "descend through the curved middle into the lower-left bowl",
            path: [
              { x: 305, y: 400 }, { x: 280, y: 350 }, { x: 235, y: 300 },
              { x: 185, y: 260 }, { x: 135, y: 220 }, { x: 95, y: 175 },
              { x: 90, y: 125 }, { x: 120, y: 75 }, { x: 180, y: 35 },
              { x: 250, y: 30 }, { x: 315, y: 70 }, { x: 370, y: 145 },
            ],
          },
          {
            label: "sweep through the join and around the lower-right bowl",
            path: [
              { x: 370, y: 145 }, { x: 405, y: 95 }, { x: 460, y: 55 },
              { x: 525, y: 30 }, { x: 595, y: 35 }, { x: 655, y: 75 },
              { x: 700, y: 130 }, { x: 705, y: 180 },
            ],
          },
          {
            label: "climb the right side and curl left at the open terminal",
            path: [
              { x: 705, y: 180 }, { x: 695, y: 225 }, { x: 670, y: 260 },
              { x: 640, y: 285 }, { x: 610, y: 295 },
            ],
          },
        ],
      },
    ],
    source: kannadaIndependentVowelSource("ಒ"),
  }],
];
