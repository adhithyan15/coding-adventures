// Authored japanese ductus records. This is the stable source-ownership boundary.

import type { DuctusEntry } from "./registry.ts";
import type { StrokeSource } from "../strokes.ts";
import { SCRIPTS, type ScriptData } from "../scriptdata.ts";

const canonicalScript = (id: string): ScriptData => {
  const inventory = SCRIPTS.find((candidate) => candidate.script === id);
  if (inventory === undefined)
    throw new Error(`Script Ductus has no ${id} inventory`);
  return inventory;
};

const japanese = canonicalScript("japanese");

const strokeSource = (glyph: string): StrokeSource =>
  japanese.letters.find((letter) => letter.glyph === glyph)!.strokeOrderSource!;

export const entries: DuctusEntry[] = [
  // Sirgazil's 23-frame animation writes hiragana し in one uninterrupted
  // motion: descend from the top, turn around the broad lower curve, and sweep
  // upward to the right. This path keeps that zero-lift order while fitting the
  // heavier bundled Noto Sans JP print outline.
  [
    "japanese:し",
    {
      script: "japanese",
      glyph: "し",
      strokes: [
        {
          segments: [
            {
              label: "descend nearly straight from the top",
              path: [
                { x: 290, y: 750 },
                { x: 290, y: 650 },
                { x: 290, y: 500 },
                { x: 290, y: 350 },
                { x: 290, y: 180 },
              ],
            },
            {
              label: "turn around the broad lower curve and sweep upward right",
              path: [
                { x: 290, y: 180 },
                { x: 300, y: 110 },
                { x: 340, y: 50 },
                { x: 410, y: 5 },
                { x: 490, y: -10 },
                { x: 590, y: 5 },
                { x: 690, y: 55 },
                { x: 770, y: 120 },
                { x: 850, y: 190 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("し"),
    },
  ],
  // Sirgazil's 20-frame animation writes hiragana く in one uninterrupted
  // motion: sweep down-left from the upper right into the sharp central turn,
  // then continue down-right to the lower tip. This path preserves that
  // zero-lift order while fitting the bundled Noto Sans JP print outline.
  [
    "japanese:く",
    {
      script: "japanese",
      glyph: "く",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep down-left from the upper right into the central turn",
              path: [
                { x: 667, y: 770 },
                { x: 600, y: 700 },
                { x: 500, y: 610 },
                { x: 400, y: 525 },
                { x: 310, y: 450 },
                { x: 250, y: 390 },
              ],
            },
            {
              label: "continue down-right to the lower tip",
              path: [
                { x: 250, y: 390 },
                { x: 300, y: 330 },
                { x: 390, y: 250 },
                { x: 490, y: 165 },
                { x: 590, y: 75 },
                { x: 690, y: -30 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("く"),
    },
  ],
  // Sirgazil's 31-frame animation writes hiragana た in four pen-down runs:
  // the upper horizontal, crossing left-falling stem, short right horizontal,
  // and lower-right bowl. These medians preserve that three-lift order while
  // fitting the bundled Noto Sans JP print outline.
  [
    "japanese:た",
    {
      script: "japanese",
      glyph: "た",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper horizontal from left to right",
              path: [
                { x: 110, y: 588 },
                { x: 250, y: 582 },
                { x: 400, y: 594 },
                { x: 590, y: 630 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "descend through the crossing stem and curve left at the foot",
              path: [
                { x: 395, y: 790 },
                { x: 390, y: 710 },
                { x: 370, y: 600 },
                { x: 340, y: 450 },
                { x: 300, y: 300 },
                { x: 250, y: 150 },
                { x: 160, y: -20 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the short right horizontal from left to right",
              path: [
                { x: 540, y: 445 },
                { x: 720, y: 455 },
                { x: 890, y: 445 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "descend into the lower-right bowl and sweep right along its base",
              path: [
                { x: 520, y: 240 },
                { x: 510, y: 160 },
                { x: 530, y: 90 },
                { x: 600, y: 40 },
                { x: 720, y: 15 },
                { x: 900, y: 35 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("た"),
    },
  ],
  // Sirgazil's 35-frame animation writes ね in two pen-down runs: the short
  // left vertical first, then the crossing hooked sweep and lower-right loop.
  // These medians preserve that one-lift order while fitting the bundled Noto
  // Sans JP print outline.
  [
    "japanese:ね",
    {
      script: "japanese",
      glyph: "ね",
      strokes: [
        {
          segments: [
            {
              label: "descend through the short left vertical",
              path: [
                { x: 333, y: 775 },
                { x: 331, y: 690 },
                { x: 326, y: 590 },
                { x: 320, y: 480 },
                { x: 314, y: 370 },
                { x: 307, y: 260 },
                { x: 305, y: 150 },
                { x: 305, y: 30 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep left from the upper right across the vertical",
              path: [
                { x: 350, y: 600 },
                { x: 300, y: 590 },
                { x: 250, y: 582 },
                { x: 200, y: 575 },
                { x: 145, y: 568 },
                { x: 100, y: 565 },
              ],
            },
            {
              label: "hook down along the diagonal and return to the crossing",
              path: [
                { x: 100, y: 565 },
                { x: 170, y: 555 },
                { x: 245, y: 505 },
                { x: 290, y: 440 },
                { x: 260, y: 360 },
                { x: 210, y: 285 },
                { x: 155, y: 205 },
                { x: 90, y: 115 },
                { x: 145, y: 190 },
                { x: 205, y: 275 },
                { x: 265, y: 360 },
                { x: 320, y: 425 },
              ],
            },
            {
              label: "finish clockwise around the lower-right loop",
              path: [
                { x: 320, y: 425 },
                { x: 390, y: 490 },
                { x: 470, y: 545 },
                { x: 560, y: 585 },
                { x: 650, y: 615 },
                { x: 720, y: 605 },
                { x: 780, y: 550 },
                { x: 820, y: 470 },
                { x: 835, y: 380 },
                { x: 835, y: 285 },
                { x: 815, y: 205 },
                { x: 770, y: 145 },
                { x: 705, y: 105 },
                { x: 635, y: 95 },
                { x: 570, y: 115 },
                { x: 525, y: 155 },
                { x: 535, y: 205 },
                { x: 585, y: 235 },
                { x: 650, y: 240 },
                { x: 725, y: 220 },
                { x: 800, y: 185 },
                { x: 875, y: 125 },
                { x: 940, y: 65 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ね"),
    },
  ],
  // Sirgazil's 29-frame animation writes み in two pen-down runs: the top bar,
  // diagonal, and lower-left loop first, then the high-right curve and outward
  // sweep. These medians preserve that one-lift order in the bundled Noto Sans
  // JP print outline.
  [
    "japanese:み",
    {
      script: "japanese",
      glyph: "み",
      strokes: [
        {
          segments: [
            {
              label: "draw the top bar from left to right",
              path: [
                { x: 235, y: 695 },
                { x: 320, y: 695 },
                { x: 410, y: 700 },
                { x: 500, y: 705 },
                { x: 575, y: 712 },
              ],
            },
            {
              label: "descend diagonally into the lower-left loop",
              path: [
                { x: 575, y: 712 },
                { x: 535, y: 625 },
                { x: 490, y: 525 },
                { x: 445, y: 425 },
                { x: 400, y: 325 },
                { x: 355, y: 225 },
                { x: 310, y: 145 },
                { x: 260, y: 95 },
                { x: 205, y: 75 },
              ],
            },
            {
              label:
                "continue around the loop and sweep out through the middle",
              path: [
                { x: 205, y: 75 },
                { x: 145, y: 75 },
                { x: 105, y: 115 },
                { x: 105, y: 175 },
                { x: 120, y: 235 },
                { x: 165, y: 295 },
                { x: 230, y: 345 },
                { x: 310, y: 385 },
                { x: 405, y: 405 },
                { x: 500, y: 405 },
                { x: 600, y: 380 },
                { x: 700, y: 340 },
                { x: 800, y: 295 },
                { x: 900, y: 235 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "begin high on the right and curve down to the left",
              path: [
                { x: 805, y: 500 },
                { x: 805, y: 420 },
                { x: 790, y: 330 },
                { x: 765, y: 240 },
                { x: 730, y: 155 },
                { x: 685, y: 85 },
                { x: 625, y: 25 },
                { x: 545, y: -25 },
              ],
            },
            {
              label: "turn upward at the finish",
              path: [
                { x: 545, y: -25 },
                { x: 610, y: 35 },
                { x: 670, y: 110 },
                { x: 720, y: 185 },
                { x: 765, y: 245 },
                { x: 815, y: 260 },
                { x: 875, y: 235 },
                { x: 925, y: 205 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("み"),
    },
  ],
  // Sirgazil's 33-frame animation writes せ in three pen-down runs: the long
  // horizontal, the left stem and base curve, then the right stem and hook.
  // These medians preserve that two-lift order in the bundled Noto Sans JP
  // print outline.
  [
    "japanese:せ",
    {
      script: "japanese",
      glyph: "せ",
      strokes: [
        {
          segments: [
            {
              label: "draw the long crossing horizontal from left to right",
              path: [
                { x: 70, y: 460 },
                { x: 190, y: 468 },
                { x: 320, y: 478 },
                { x: 460, y: 490 },
                { x: 600, y: 503 },
                { x: 740, y: 515 },
                { x: 890, y: 530 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the left crossing",
              path: [
                { x: 300, y: 710 },
                { x: 300, y: 620 },
                { x: 300, y: 520 },
                { x: 300, y: 420 },
                { x: 300, y: 310 },
                { x: 300, y: 205 },
                { x: 305, y: 125 },
              ],
            },
            {
              label: "curve right along the base",
              path: [
                { x: 305, y: 125 },
                { x: 325, y: 70 },
                { x: 375, y: 40 },
                { x: 455, y: 25 },
                { x: 560, y: 22 },
                { x: 675, y: 25 },
                { x: 790, y: 40 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the right crossing",
              path: [
                { x: 698, y: 735 },
                { x: 698, y: 650 },
                { x: 698, y: 560 },
                { x: 697, y: 470 },
                { x: 696, y: 380 },
                { x: 692, y: 305 },
              ],
            },
            {
              label: "hook left at the finish",
              path: [
                { x: 692, y: 305 },
                { x: 680, y: 270 },
                { x: 650, y: 255 },
                { x: 610, y: 255 },
                { x: 565, y: 260 },
                { x: 525, y: 268 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("せ"),
    },
  ],
  // Sirgazil's 28-frame animation writes て in one uninterrupted run: the
  // high bar, returning diagonal, and broad lower curve. These medians preserve
  // that zero-lift order in the bundled Noto Sans JP print outline.
  [
    "japanese:て",
    {
      script: "japanese",
      glyph: "て",
      strokes: [
        {
          segments: [
            {
              label: "draw the high horizontal from left to right",
              path: [
                { x: 110, y: 620 },
                { x: 220, y: 630 },
                { x: 340, y: 642 },
                { x: 470, y: 655 },
                { x: 600, y: 668 },
                { x: 730, y: 680 },
                { x: 845, y: 688 },
              ],
            },
            {
              label: "turn back down and left through the diagonal",
              path: [
                { x: 845, y: 688 },
                { x: 760, y: 675 },
                { x: 675, y: 645 },
                { x: 600, y: 600 },
                { x: 535, y: 540 },
                { x: 485, y: 470 },
                { x: 450, y: 390 },
                { x: 430, y: 305 },
              ],
            },
            {
              label:
                "round the broad lower curve and sweep right to the finish",
              path: [
                { x: 430, y: 305 },
                { x: 430, y: 230 },
                { x: 450, y: 165 },
                { x: 490, y: 110 },
                { x: 550, y: 70 },
                { x: 620, y: 42 },
                { x: 700, y: 22 },
                { x: 770, y: 12 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("て"),
    },
  ],
  // Sirgazil's 32-frame animation writes な in four pen-down runs: horizontal,
  // crossing stem, upper-right diagonal, then the lower stem, loop, and sweep.
  [
    "japanese:な",
    {
      script: "japanese",
      glyph: "な",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper-left horizontal from left to right",
              path: [
                { x: 120, y: 590 },
                { x: 210, y: 589 },
                { x: 300, y: 592 },
                { x: 390, y: 600 },
                { x: 480, y: 615 },
                { x: 550, y: 635 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the crossing left-falling stem",
              path: [
                { x: 405, y: 775 },
                { x: 395, y: 700 },
                { x: 370, y: 620 },
                { x: 340, y: 535 },
                { x: 305, y: 450 },
                { x: 265, y: 365 },
                { x: 220, y: 285 },
                { x: 175, y: 205 },
                { x: 135, y: 165 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the short upper-right diagonal down and right",
              path: [
                { x: 680, y: 620 },
                { x: 735, y: 592 },
                { x: 790, y: 560 },
                { x: 845, y: 525 },
                { x: 900, y: 490 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the lower-right stem",
              path: [
                { x: 648, y: 460 },
                { x: 649, y: 385 },
                { x: 651, y: 310 },
                { x: 653, y: 235 },
                { x: 655, y: 165 },
                { x: 655, y: 110 },
              ],
            },
            {
              label: "turn around the loop and sweep right to the finish",
              path: [
                { x: 655, y: 110 },
                { x: 640, y: 55 },
                { x: 595, y: 20 },
                { x: 530, y: -5 },
                { x: 460, y: 0 },
                { x: 400, y: 35 },
                { x: 360, y: 85 },
                { x: 365, y: 135 },
                { x: 405, y: 175 },
                { x: 470, y: 205 },
                { x: 545, y: 210 },
                { x: 625, y: 185 },
                { x: 705, y: 150 },
                { x: 790, y: 105 },
                { x: 870, y: 55 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("な"),
    },
  ],
  // Sirgazil's つ animation supplies the one-run movement. Unicode identifies
  // U+3063 as small tsu; these medians preserve that movement while fitting it
  // explicitly to the bundled smaller Noto Sans JP glyph.
  [
    "japanese:っ",
    {
      script: "japanese",
      glyph: "っ",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper left and sweep right across the high shoulder",
              path: [
                { x: 180, y: 360 },
                { x: 280, y: 390 },
                { x: 380, y: 425 },
                { x: 480, y: 450 },
                { x: 600, y: 470 },
                { x: 700, y: 430 },
                { x: 780, y: 350 },
              ],
            },
            {
              label:
                "round down the right side and finish by sweeping left along the lower curve",
              path: [
                { x: 780, y: 350 },
                { x: 810, y: 280 },
                { x: 780, y: 200 },
                { x: 700, y: 110 },
                { x: 600, y: 60 },
                { x: 500, y: 30 },
                { x: 390, y: 15 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("っ"),
    },
  ],
  // Sirgazil's 28-frame animation writes も in three pen-down runs: descend
  // through the stem and broad lower bowl, then lift for each left-to-right
  // horizontal. These medians preserve that order while fitting the bundled
  // Noto Sans JP print outline.
  [
    "japanese:も",
    {
      script: "japanese",
      glyph: "も",
      strokes: [
        {
          segments: [
            {
              label:
                "descend and turn around the broad lower bowl to the rising right tip",
              path: [
                { x: 399, y: 772 },
                { x: 395, y: 690 },
                { x: 385, y: 610 },
                { x: 374, y: 520 },
                { x: 362, y: 430 },
                { x: 350, y: 335 },
                { x: 340, y: 245 },
                { x: 335, y: 170 },
                { x: 350, y: 90 },
                { x: 405, y: 20 },
                { x: 500, y: -8 },
                { x: 610, y: -5 },
                { x: 710, y: 35 },
                { x: 785, y: 105 },
                { x: 830, y: 195 },
                { x: 810, y: 280 },
                { x: 735, y: 385 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the upper horizontal from left to right across the stem",
              path: [
                { x: 120, y: 615 },
                { x: 215, y: 595 },
                { x: 315, y: 580 },
                { x: 415, y: 578 },
                { x: 520, y: 582 },
                { x: 617, y: 590 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the lower horizontal from left to right across the stem",
              path: [
                { x: 97, y: 366 },
                { x: 190, y: 345 },
                { x: 290, y: 330 },
                { x: 400, y: 325 },
                { x: 510, y: 329 },
                { x: 610, y: 336 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("も"),
    },
  ],
  // Sirgazil's 30-frame animation writes わ in two pen-down runs: the long
  // left vertical first, then the crossing sweep, down-left hook, central
  // return, and broad right loop. These medians preserve that one-lift order
  // while fitting the bundled Noto Sans JP print outline.
  [
    "japanese:わ",
    {
      script: "japanese",
      glyph: "わ",
      strokes: [
        {
          segments: [
            {
              label: "descend through the long left vertical",
              path: [
                { x: 340, y: 780 },
                { x: 338, y: 690 },
                { x: 334, y: 590 },
                { x: 330, y: 480 },
                { x: 326, y: 365 },
                { x: 322, y: 250 },
                { x: 320, y: 135 },
                { x: 320, y: 25 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right from the upper left across the vertical",
              path: [
                { x: 95, y: 565 },
                { x: 160, y: 570 },
                { x: 225, y: 578 },
                { x: 285, y: 586 },
                { x: 330, y: 594 },
                { x: 360, y: 600 },
              ],
            },
            {
              label:
                "hook down and left, then return through the central crossing",
              path: [
                { x: 360, y: 600 },
                { x: 330, y: 525 },
                { x: 290, y: 440 },
                { x: 245, y: 355 },
                { x: 195, y: 275 },
                { x: 145, y: 200 },
                { x: 95, y: 130 },
                { x: 145, y: 200 },
                { x: 195, y: 275 },
                { x: 245, y: 350 },
                { x: 285, y: 410 },
                { x: 315, y: 400 },
              ],
            },
            {
              label: "continue clockwise around the broad right loop",
              path: [
                { x: 315, y: 400 },
                { x: 395, y: 460 },
                { x: 490, y: 510 },
                { x: 590, y: 545 },
                { x: 685, y: 560 },
                { x: 770, y: 535 },
                { x: 840, y: 485 },
                { x: 880, y: 415 },
                { x: 888, y: 340 },
                { x: 875, y: 270 },
                { x: 840, y: 210 },
                { x: 785, y: 155 },
                { x: 715, y: 110 },
                { x: 635, y: 75 },
                { x: 550, y: 35 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("わ"),
    },
  ],
  // Sirgazil's 30-frame animation writes ゆ in two pen-down runs: the left
  // stem and broad clockwise loop first, then the central descending curve.
  // These medians preserve that one-lift order while fitting the bundled Noto
  // Sans JP print outline.
  [
    "japanese:ゆ",
    {
      script: "japanese",
      glyph: "ゆ",
      strokes: [
        {
          segments: [
            {
              label:
                "descend through the left stem and turn up across the high shoulder",
              path: [
                { x: 200, y: 710 },
                { x: 195, y: 635 },
                { x: 187, y: 555 },
                { x: 180, y: 475 },
                { x: 175, y: 395 },
                { x: 178, y: 305 },
                { x: 190, y: 220 },
                { x: 200, y: 145 },
                { x: 205, y: 230 },
                { x: 215, y: 315 },
                { x: 235, y: 390 },
                { x: 265, y: 465 },
                { x: 330, y: 535 },
                { x: 410, y: 585 },
              ],
            },
            {
              label: "continue clockwise around the broad loop",
              path: [
                { x: 410, y: 585 },
                { x: 500, y: 620 },
                { x: 600, y: 625 },
                { x: 700, y: 600 },
                { x: 785, y: 550 },
                { x: 845, y: 480 },
                { x: 870, y: 395 },
                { x: 860, y: 315 },
                { x: 820, y: 245 },
                { x: 755, y: 190 },
                { x: 675, y: 155 },
                { x: 595, y: 140 },
              ],
            },
            {
              label: "curve left to the inner finish",
              path: [
                { x: 595, y: 140 },
                { x: 530, y: 155 },
                { x: 470, y: 185 },
                { x: 420, y: 225 },
                { x: 375, y: 275 },
                { x: 345, y: 315 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the center of the loop",
              path: [
                { x: 550, y: 790 },
                { x: 560, y: 700 },
                { x: 570, y: 610 },
                { x: 580, y: 520 },
                { x: 585, y: 430 },
                { x: 578, y: 335 },
                { x: 558, y: 245 },
                { x: 525, y: 155 },
              ],
            },
            {
              label: "curve down and left to the finish",
              path: [
                { x: 525, y: 155 },
                { x: 490, y: 80 },
                { x: 445, y: 20 },
                { x: 390, y: -45 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ゆ"),
    },
  ],
  // Sirgazil's corrected 26-frame animation writes よ with a short
  // left-to-right upper bar, then one continuous stem and clockwise lower
  // loop. These medians preserve that one-lift order in the bundled Noto Sans
  // JP subset.
  [
    "japanese:よ",
    {
      script: "japanese",
      glyph: "よ",
      strokes: [
        {
          segments: [
            {
              label: "draw the short upper horizontal from left to right",
              path: [
                { x: 500, y: 548 }, { x: 610, y: 550 }, { x: 720, y: 554 }, { x: 825, y: 562 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the upper bar and turn left",
              path: [
                { x: 535, y: 780 }, { x: 530, y: 665 }, { x: 520, y: 545 }, { x: 500, y: 420 },
                { x: 470, y: 315 }, { x: 430, y: 255 }, { x: 390, y: 238 },
              ],
            },
            {
              label: "continue clockwise around the broad lower loop to the rightward finish",
              path: [
                { x: 390, y: 238 }, { x: 300, y: 230 }, { x: 220, y: 205 }, { x: 170, y: 160 },
                { x: 165, y: 110 }, { x: 215, y: 35 }, { x: 300, y: 0 }, { x: 395, y: 0 },
                { x: 500, y: 40 }, { x: 580, y: 175 }, { x: 650, y: 190 }, { x: 720, y: 130 },
                { x: 780, y: 60 }, { x: 815, y: 40 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("よ"),
    },
  ],
  // Sirgazil's 32-frame animation writes め in two runs: the short left
  // descending curve, then the crossing diagonal and continuous paired loop.
  // These medians preserve that one-lift order in the bundled Noto Sans JP
  // subset.
  [
    "japanese:め",
    {
      script: "japanese",
      glyph: "め",
      strokes: [
        {
          segments: [
            {
              label: "descend from the upper left and curve down and right",
              path: [
                { x: 220, y: 690 }, { x: 235, y: 610 }, { x: 265, y: 520 },
                { x: 305, y: 425 }, { x: 350, y: 335 }, { x: 405, y: 240 },
                { x: 470, y: 135 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend diagonally left through the first stroke",
              path: [
                { x: 625, y: 745 }, { x: 610, y: 665 }, { x: 585, y: 575 },
                { x: 555, y: 485 }, { x: 515, y: 390 }, { x: 470, y: 300 },
                { x: 420, y: 220 }, { x: 365, y: 155 },
              ],
            },
            {
              label: "loop around the lower left and sweep upward across the top",
              path: [
                { x: 365, y: 155 }, { x: 305, y: 100 }, { x: 245, y: 70 },
                { x: 190, y: 80 }, { x: 145, y: 125 }, { x: 120, y: 195 },
                { x: 125, y: 275 }, { x: 155, y: 350 }, { x: 205, y: 420 },
                { x: 275, y: 480 }, { x: 355, y: 530 }, { x: 445, y: 575 },
                { x: 545, y: 595 }, { x: 640, y: 580 },
              ],
            },
            {
              label: "continue clockwise around the broad right curve to the lower finish",
              path: [
                { x: 640, y: 580 }, { x: 725, y: 545 }, { x: 790, y: 490 },
                { x: 835, y: 420 }, { x: 855, y: 340 }, { x: 850, y: 255 },
                { x: 820, y: 180 }, { x: 770, y: 115 }, { x: 705, y: 70 },
                { x: 630, y: 35 }, { x: 550, y: 5 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("め"),
    },
  ],
  // Sirgazil's 24-frame animation writes つ in one uninterrupted run: sweep
  // right across the high arch, turn down around the outer right side, and
  // return left along the broad lower curve. These medians preserve that
  // zero-lift order in the bundled Noto Sans JP subset.
  [
    "japanese:つ",
    {
      script: "japanese",
      glyph: "つ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right from the upper left across the high arch",
              path: [
                { x: 85, y: 480 }, { x: 175, y: 505 }, { x: 270, y: 540 },
                { x: 380, y: 570 }, { x: 495, y: 595 }, { x: 610, y: 610 },
                { x: 700, y: 590 }, { x: 770, y: 545 }, { x: 820, y: 480 },
              ],
            },
            {
              label: "turn down around the right side and return left along the broad lower curve",
              path: [
                { x: 820, y: 480 }, { x: 860, y: 405 }, { x: 862, y: 330 },
                { x: 835, y: 260 }, { x: 785, y: 205 }, { x: 720, y: 160 },
                { x: 645, y: 125 }, { x: 560, y: 90 }, { x: 465, y: 65 },
                { x: 365, y: 55 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("つ"),
    },
  ],
  // Sirgazil's 26-frame animation writes hiragana ろ in a single pen-down run:
  // the start marker never leaves the upper-left origin across all 26 frames.
  // This path is the bundled Noto Sans JP subset's own medial line, so the
  // shoulder, the diagonal and the belly are the font's geometry rather than a
  // second drawing. The diagonal is traced to its foot because it has to be:
  // stopping where the belly departs leaves a tenth of the letter's ink
  // untraced, which the coverage check rejects.
  [
    "japanese:ろ",
    {
      script: "japanese",
      glyph: "ろ",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper left and draw the short high shoulder to the right",
              path: [
                { x: 252, y: 671 },
                { x: 323, y: 695 },
                { x: 403, y: 695 },
                { x: 484, y: 695 },
                { x: 563, y: 699 },
                { x: 640, y: 691 },
              ],
            },
            {
              label:
                "turn down at the corner and descend the long diagonal to the lower left",
              path: [
                { x: 640, y: 691 },
                { x: 566, y: 585 },
                { x: 463, y: 498 },
                { x: 380, y: 403 },
                { x: 272, y: 327 },
                { x: 168, y: 243 },
              ],
            },
            {
              label:
                "swing right into the broad clockwise belly and finish with a short tail at the bottom",
              path: [
                { x: 168, y: 243 },
                { x: 250, y: 309 },
                { x: 334, y: 373 },
                { x: 426, y: 407 },
                { x: 531, y: 419 },
                { x: 637, y: 418 },
                { x: 731, y: 379 },
                { x: 795, y: 296 },
                { x: 804, y: 193 },
                { x: 762, y: 101 },
                { x: 678, y: 39 },
                { x: 581, y: 8 },
                { x: 475, y: -2 },
                { x: 368, y: -1 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ろ"),
    },
  ],
  // Small ゅ takes ゆ's two-run movement, exactly as small っ takes つ's. The
  // path is ゆ's own verified pen path mapped through the two glyphs' bounding
  // boxes and then snapped to the SMALL glyph's medial line — no point moved
  // more than 33 units to land on it — so the order is ゆ's citation and the
  // geometry is ゅ's own outline.
  [
    "japanese:ゅ",
    {
      script: "japanese",
      glyph: "ゅ",
      strokes: [
        {
          segments: [
            {
              label:
                "descend through the left stem and turn up across the high shoulder",
              path: [
                { x: 259, y: 543 },
                { x: 247, y: 486 },
                { x: 238, y: 420 },
                { x: 235, y: 357 },
                { x: 232, y: 291 },
                { x: 241, y: 216 },
                { x: 247, y: 147 },
                { x: 250, y: 93 },
                { x: 244, y: 156 },
                { x: 241, y: 225 },
                { x: 289, y: 282 },
                { x: 319, y: 336 },
                { x: 373, y: 396 },
                { x: 436, y: 441 },
              ],
            },
            {
              label:
                "continue clockwise around the broad loop",
              path: [
                { x: 436, y: 441 },
                { x: 508, y: 471 },
                { x: 595, y: 486 },
                { x: 685, y: 474 },
                { x: 766, y: 429 },
                { x: 808, y: 360 },
                { x: 817, y: 291 },
                { x: 811, y: 231 },
                { x: 784, y: 168 },
                { x: 727, y: 117 },
                { x: 658, y: 90 },
                { x: 589, y: 87 },
              ],
            },
            {
              label:
                "curve left to the inner finish",
              path: [
                { x: 589, y: 87 },
                { x: 535, y: 93 },
                { x: 484, y: 114 },
                { x: 439, y: 147 },
                { x: 400, y: 192 },
                { x: 403, y: 225 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "descend through the center of the loop",
              path: [
                { x: 562, y: 582 },
                { x: 568, y: 540 },
                { x: 580, y: 465 },
                { x: 583, y: 393 },
                { x: 583, y: 318 },
                { x: 580, y: 240 },
                { x: 568, y: 168 },
                { x: 532, y: 96 },
              ],
            },
            {
              label:
                "curve down and left to the finish",
              path: [
                { x: 532, y: 96 },
                { x: 508, y: 24 },
                { x: 469, y: -21 },
                { x: 415, y: -42 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ゅ"),
    },
  ],
  // The counter tranche adds six independently written hiragana. Each path
  // follows the cited Sirgazil frame order and is fitted to the bundled Noto
  // Sans JP subset, so the prose claim and the rendered filmstrip stay one
  // checkable object.
  [
    "japanese:の",
    {
      script: "japanese",
      glyph: "の",
      strokes: [
        {
          segments: [
            {
              label: "descend from high centre-right toward the lower left",
              path: [
                { x: 520, y: 680 }, { x: 510, y: 600 }, { x: 490, y: 515 },
                { x: 465, y: 425 }, { x: 435, y: 335 }, { x: 400, y: 245 },
                { x: 360, y: 170 }, { x: 315, y: 120 }, { x: 270, y: 95 },
              ],
            },
            {
              label: "round the bottom and rise around the broad left curve",
              path: [
                { x: 270, y: 95 }, { x: 225, y: 90 }, { x: 185, y: 115 },
                { x: 150, y: 165 }, { x: 130, y: 230 }, { x: 128, y: 305 },
                { x: 145, y: 385 }, { x: 180, y: 460 }, { x: 235, y: 525 },
                { x: 305, y: 585 }, { x: 375, y: 635 }, { x: 455, y: 665 },
                { x: 520, y: 680 },
              ],
            },
            {
              label: "cross near the top and swing right and down to the finish",
              path: [
                { x: 520, y: 680 }, { x: 575, y: 665 }, { x: 665, y: 640 },
                { x: 740, y: 595 }, { x: 800, y: 530 }, { x: 845, y: 450 },
                { x: 865, y: 360 }, { x: 855, y: 270 }, { x: 820, y: 190 },
                { x: 765, y: 125 }, { x: 695, y: 80 }, { x: 615, y: 55 },
                { x: 525, y: 40 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("の"),
    },
  ],
  [
    "japanese:ひ",
    {
      script: "japanese",
      glyph: "ひ",
      strokes: [
        {
          segments: [
            {
              label: "draw the short high shoulder rightward and turn down",
              path: [
                { x: 120, y: 645 }, { x: 205, y: 650 }, { x: 295, y: 660 },
                { x: 385, y: 675 }, { x: 465, y: 695 }, { x: 430, y: 660 },
                { x: 390, y: 615 },
              ],
            },
            {
              label: "descend and swing left around the broad bottom",
              path: [
                { x: 390, y: 615 }, { x: 335, y: 545 }, { x: 285, y: 455 },
                { x: 245, y: 355 }, { x: 215, y: 255 }, { x: 205, y: 165 },
                { x: 225, y: 90 }, { x: 275, y: 35 }, { x: 350, y: 5 },
                { x: 435, y: -5 }, { x: 520, y: 20 },
              ],
            },
            {
              label: "rise up the right side and finish with the outward flick",
              path: [
                { x: 520, y: 20 }, { x: 590, y: 65 }, { x: 640, y: 135 },
                { x: 675, y: 225 }, { x: 700, y: 330 }, { x: 710, y: 440 },
                { x: 720, y: 545 }, { x: 710, y: 635 }, { x: 690, y: 700 },
                { x: 720, y: 650 }, { x: 750, y: 575 }, { x: 785, y: 495 },
                { x: 825, y: 420 }, { x: 870, y: 350 }, { x: 915, y: 290 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ひ"),
    },
  ],
  [
    "japanese:ふ",
    {
      script: "japanese",
      glyph: "ふ",
      strokes: [
        {
          segments: [
            {
              label: "draw the short top tick down and to the right",
              path: [
                { x: 395, y: 735 }, { x: 445, y: 710 }, { x: 500, y: 680 },
                { x: 555, y: 650 }, { x: 610, y: 615 }, { x: 560, y: 565 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve down and round before sweeping away to the lower left",
              path: [
                { x: 425, y: 450 }, { x: 475, y: 405 }, { x: 520, y: 355 },
                { x: 560, y: 300 }, { x: 590, y: 240 }, { x: 610, y: 175 },
                { x: 610, y: 110 }, { x: 585, y: 60 }, { x: 535, y: 30 },
                { x: 470, y: 15 }, { x: 400, y: 18 }, { x: 330, y: 30 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate lower-left mark down and left",
              path: [
                { x: 300, y: 260 }, { x: 255, y: 220 }, { x: 205, y: 175 },
                { x: 150, y: 130 }, { x: 95, y: 90 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate lower-right mark down and right",
              path: [
                { x: 755, y: 350 }, { x: 795, y: 300 }, { x: 835, y: 240 },
                { x: 875, y: 175 }, { x: 910, y: 110 }, { x: 930, y: 65 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ふ"),
    },
  ],
  [
    "japanese:ほ",
    {
      script: "japanese",
      glyph: "ほ",
      strokes: [
        {
          segments: [
            {
              label: "descend through the left vertical and hook left at the foot",
              path: [
                { x: 205, y: 745 }, { x: 195, y: 660 }, { x: 180, y: 565 },
                { x: 165, y: 465 }, { x: 155, y: 365 }, { x: 155, y: 270 },
                { x: 165, y: 185 }, { x: 190, y: 120 }, { x: 225, y: 85 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the upper right horizontal from left to right",
              path: [
                { x: 430, y: 680 }, { x: 515, y: 680 }, { x: 610, y: 680 },
                { x: 705, y: 680 }, { x: 795, y: 685 }, { x: 865, y: 695 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the lower right horizontal from left to right",
              path: [
                { x: 420, y: 450 }, { x: 510, y: 445 }, { x: 605, y: 445 },
                { x: 700, y: 445 }, { x: 795, y: 450 }, { x: 885, y: 465 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through both bars and turn around the lower loop",
              path: [
                { x: 690, y: 680 }, { x: 690, y: 590 }, { x: 690, y: 500 },
                { x: 690, y: 410 }, { x: 690, y: 320 }, { x: 685, y: 235 },
                { x: 680, y: 165 }, { x: 690, y: 100 }, { x: 665, y: 50 },
                { x: 610, y: 20 }, { x: 545, y: 10 }, { x: 485, y: 25 },
                { x: 440, y: 60 }, { x: 420, y: 110 }, { x: 430, y: 155 },
                { x: 465, y: 195 }, { x: 520, y: 220 }, { x: 585, y: 225 },
                { x: 650, y: 215 },
              ],
            },
            {
              label: "sweep right from the loop to the low finish",
              path: [
                { x: 650, y: 215 }, { x: 735, y: 195 }, { x: 805, y: 165 },
                { x: 865, y: 125 }, { x: 915, y: 80 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ほ"),
    },
  ],
  [
    "japanese:む",
    {
      script: "japanese",
      glyph: "む",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper-left horizontal from left to right",
              path: [
                { x: 100, y: 620 }, { x: 180, y: 620 }, { x: 265, y: 620 },
                { x: 350, y: 620 }, { x: 435, y: 630 }, { x: 520, y: 650 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "descend through the horizontal into the foot loop",
              path: [
                { x: 330, y: 780 }, { x: 330, y: 700 }, { x: 330, y: 620 },
                { x: 330, y: 535 }, { x: 330, y: 450 }, { x: 335, y: 365 },
                { x: 340, y: 285 }, { x: 330, y: 220 }, { x: 300, y: 180 },
                { x: 255, y: 165 }, { x: 205, y: 180 }, { x: 165, y: 220 },
                { x: 145, y: 280 }, { x: 155, y: 345 }, { x: 185, y: 395 },
                { x: 230, y: 425 }, { x: 280, y: 425 }, { x: 320, y: 395 },
                { x: 340, y: 345 },
              ],
            },
            {
              label: "leave the loop and sweep broadly to the right",
              path: [
                { x: 340, y: 345 }, { x: 330, y: 265 }, { x: 325, y: 190 },
                { x: 330, y: 110 }, { x: 330, y: 40 }, { x: 360, y: 10 },
                { x: 420, y: 0 }, { x: 500, y: 0 }, { x: 580, y: 0 },
                { x: 650, y: 10 }, { x: 710, y: 35 }, { x: 750, y: 80 },
                { x: 770, y: 140 }, { x: 785, y: 210 }, { x: 785, y: 245 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "add the separate upper-right mark down and right",
              path: [
                { x: 685, y: 660 }, { x: 730, y: 625 }, { x: 775, y: 585 },
                { x: 820, y: 540 }, { x: 865, y: 495 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("む"),
    },
  ],
  [
    "japanese:や",
    {
      script: "japanese",
      glyph: "や",
      strokes: [
        {
          segments: [
            {
              label: "sweep right from the left and curl into the hook",
              path: [
                { x: 75, y: 390 }, { x: 155, y: 415 }, { x: 245, y: 455 },
                { x: 340, y: 500 }, { x: 440, y: 545 }, { x: 540, y: 580 },
                { x: 635, y: 605 }, { x: 710, y: 610 }, { x: 775, y: 595 },
                { x: 825, y: 560 }, { x: 855, y: 510 }, { x: 870, y: 455 },
                { x: 860, y: 400 }, { x: 830, y: 350 }, { x: 785, y: 315 },
                { x: 725, y: 295 }, { x: 660, y: 295 }, { x: 600, y: 315 },
                { x: 555, y: 335 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "add the separate short tick at the top",
              path: [
                { x: 445, y: 770 }, { x: 485, y: 740 }, { x: 530, y: 705 },
                { x: 575, y: 665 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the long descender across the sweep to the lower right",
              path: [
                { x: 200, y: 700 }, { x: 235, y: 635 }, { x: 270, y: 565 },
                { x: 305, y: 490 }, { x: 340, y: 410 }, { x: 375, y: 325 },
                { x: 405, y: 240 }, { x: 435, y: 155 }, { x: 465, y: 75 },
                { x: 490, y: 5 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("や"),
    },
  ],
  // Small ゃ and ょ take や's and よ's movements, exactly as small ゅ takes ゆ's.
  // Each path is the full-size glyph's own verified pen path mapped through the
  // two glyphs' bounding boxes and then snapped to the SMALL glyph's medial line
  // (no point lies more than 34 units from the mapped path), so the order is the
  // full-size sign's citation and the geometry is the small glyph's own outline.
  // The captions are the full-size sign's, verbatim.
  [
    "japanese:ゃ",
    {
      script: "japanese",
      glyph: "ゃ",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep right from the left and curl into the hook",
              path: [
                { x: 129, y: 314 },
                { x: 189, y: 306 },
                { x: 247, y: 331 },
                { x: 305, y: 355 },
                { x: 359, y: 387 },
                { x: 418, y: 408 },
                { x: 477, y: 431 },
                { x: 537, y: 449 },
                { x: 597, y: 465 },
                { x: 660, y: 469 },
                { x: 722, y: 457 },
                { x: 772, y: 422 },
                { x: 796, y: 364 },
                { x: 794, y: 302 },
                { x: 765, y: 247 },
                { x: 711, y: 216 },
                { x: 649, y: 206 },
                { x: 587, y: 213 },
                { x: 533, y: 242 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "add the separate short tick at the top",
              path: [
                { x: 441, y: 594 },
                { x: 481, y: 581 },
                { x: 514, y: 553 },
                { x: 553, y: 542 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the long descender across the sweep to the lower right",
              path: [
                { x: 221, y: 526 },
                { x: 274, y: 486 },
                { x: 305, y: 424 },
                { x: 330, y: 360 },
                { x: 364, y: 300 },
                { x: 391, y: 236 },
                { x: 417, y: 172 },
                { x: 440, y: 106 },
                { x: 462, y: 41 },
                { x: 481, y: -26 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ゃ"),
    },
  ],
  [
    "japanese:ょ",
    {
      script: "japanese",
      glyph: "ょ",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short upper horizontal from left to right",
              path: [
                { x: 499, y: 432 },
                { x: 585, y: 435 },
                { x: 671, y: 439 },
                { x: 755, y: 452 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "descend through the upper bar and turn left",
              path: [
                { x: 527, y: 612 },
                { x: 491, y: 537 },
                { x: 504, y: 453 },
                { x: 491, y: 377 },
                { x: 495, y: 290 },
                { x: 482, y: 207 },
                { x: 407, y: 176 },
              ],
            },
            {
              label:
                "continue clockwise around the broad lower loop to the rightward finish",
              path: [
                { x: 407, y: 176 },
                { x: 336, y: 169 },
                { x: 268, y: 144 },
                { x: 229, y: 87 },
                { x: 246, y: 19 },
                { x: 307, y: -18 },
                { x: 378, y: -28 },
                { x: 449, y: -18 },
                { x: 497, y: 32 },
                { x: 526, y: 94 },
                { x: 582, y: 136 },
                { x: 647, y: 105 },
                { x: 706, y: 65 },
                { x: 747, y: 8 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ょ"),
    },
  ],
  // を is fitted from KanjiVG's three directed stroke paths for U+3092. KanjiVG
  // supplies only the ORDER and DIRECTION: its paths were scaled onto the bundled
  // Noto Sans JP outline and every sample was snapped to that outline's skeleton,
  // so the coordinates below are the print glyph's medial line. Stroke 2 is one
  // run with a sharp turn at its foot; the turn is the boundary between its two
  // labelled segments.
  [
    "japanese:を",
    {
      script: "japanese",
      glyph: "を",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short upper bar from left to right",
              path: [
                { x: 139, y: 651 },
                { x: 221, y: 639 },
                { x: 304, y: 635 },
                { x: 384, y: 655 },
                { x: 463, y: 645 },
                { x: 546, y: 647 },
                { x: 630, y: 654 },
                { x: 713, y: 664 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "start above the bar and cut down-left through it",
              path: [
                { x: 435, y: 775 },
                { x: 405, y: 719 },
                { x: 388, y: 658 },
                { x: 356, y: 603 },
                { x: 332, y: 543 },
                { x: 300, y: 488 },
                { x: 270, y: 431 },
                { x: 230, y: 382 },
                { x: 185, y: 336 },
                { x: 130, y: 303 },
              ],
            },
            {
              label:
                "turn up and right into the arch, then down the stem",
              path: [
                { x: 130, y: 303 },
                { x: 186, y: 336 },
                { x: 231, y: 383 },
                { x: 282, y: 423 },
                { x: 345, y: 436 },
                { x: 409, y: 448 },
                { x: 474, y: 445 },
                { x: 525, y: 407 },
                { x: 537, y: 345 },
                { x: 546, y: 282 },
                { x: 555, y: 218 },
                { x: 555, y: 153 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "start at the right and sweep down-left across the middle",
              path: [
                { x: 847, y: 503 },
                { x: 796, y: 450 },
                { x: 725, y: 420 },
                { x: 655, y: 390 },
                { x: 583, y: 364 },
                { x: 520, y: 321 },
                { x: 452, y: 286 },
                { x: 390, y: 241 },
                { x: 340, y: 183 },
                { x: 323, y: 110 },
              ],
            },
            {
              label:
                "round the lower left and finish along the base to the right",
              path: [
                { x: 323, y: 110 },
                { x: 332, y: 62 },
                { x: 366, y: 27 },
                { x: 410, y: 6 },
                { x: 458, y: -3 },
                { x: 506, y: -7 },
                { x: 555, y: -9 },
                { x: 604, y: -8 },
                { x: 653, y: -5 },
                { x: 702, y: -1 },
                { x: 750, y: 5 },
                { x: 794, y: 25 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("を"),
    },
  ],
  // そ, れ and る are fitted from KanjiVG's directed stroke paths for U+305D,
  // U+308C and U+308B, the way を is. KanjiVG supplies only the ORDER and
  // DIRECTION. Each path runs along the bundled Noto Sans JP outline's medial
  // line between turning points read off that outline, so the coordinates are
  // the print glyph's own. Every sharp turn inside a stroke is a boundary
  // between two labelled segments, and each join is exact. Where the print
  // glyph has no separate ink for a return (そ doubling back along its middle
  // bar, れ climbing back up its diagonal), the path retraces the ink it came
  // down.
  [
    "japanese:そ",
    {
      script: "japanese",
      glyph: "そ",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short top bar from left to right",
              path: [
                { x: 302, y: 705 },
                { x: 372, y: 709 },
                { x: 442, y: 713 },
                { x: 512, y: 715 },
                { x: 582, y: 717 },
                { x: 650, y: 705 },
              ],
            },
            {
              label:
                "turn and run down-left to the bar",
              path: [
                { x: 650, y: 705 },
                { x: 613, y: 648 },
                { x: 559, y: 602 },
                { x: 506, y: 557 },
                { x: 452, y: 512 },
                { x: 399, y: 466 },
                { x: 350, y: 417 },
                { x: 289, y: 385 },
                { x: 219, y: 375 },
                { x: 150, y: 365 },
              ],
            },
            {
              label:
                "swing back right along the long bar",
              path: [
                { x: 150, y: 365 },
                { x: 218, y: 373 },
                { x: 286, y: 383 },
                { x: 353, y: 397 },
                { x: 422, y: 397 },
                { x: 491, y: 397 },
                { x: 560, y: 402 },
                { x: 628, y: 413 },
                { x: 696, y: 417 },
                { x: 765, y: 421 },
                { x: 834, y: 421 },
              ],
            },
            {
              label:
                "double back and drop into the curve",
              path: [
                { x: 834, y: 421 },
                { x: 765, y: 421 },
                { x: 696, y: 419 },
                { x: 628, y: 414 },
                { x: 560, y: 402 },
                { x: 501, y: 373 },
                { x: 457, y: 320 },
                { x: 416, y: 266 },
                { x: 394, y: 201 },
              ],
            },
            {
              label:
                "round the lower left and finish on the base",
              path: [
                { x: 394, y: 201 },
                { x: 399, y: 131 },
                { x: 437, y: 74 },
                { x: 492, y: 30 },
                { x: 559, y: 9 },
                { x: 628, y: -3 },
                { x: 698, y: -3 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("そ"),
    },
  ],
  [
    "japanese:れ",
    {
      script: "japanese",
      glyph: "れ",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the vertical from top to bottom",
              path: [
                { x: 334, y: 750 },
                { x: 330, y: 681 },
                { x: 326, y: 613 },
                { x: 320, y: 544 },
                { x: 314, y: 476 },
                { x: 306, y: 409 },
                { x: 299, y: 341 },
                { x: 302, y: 273 },
                { x: 302, y: 204 },
                { x: 302, y: 135 },
                { x: 302, y: 66 },
                { x: 306, y: -2 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the short bar from left to right",
              path: [
                { x: 122, y: 570 },
                { x: 192, y: 575 },
                { x: 262, y: 585 },
                { x: 330, y: 598 },
              ],
            },
            {
              label:
                "turn and cut down to the lower left",
              path: [
                { x: 330, y: 598 },
                { x: 318, y: 525 },
                { x: 314, y: 451 },
                { x: 289, y: 383 },
                { x: 238, y: 330 },
                { x: 193, y: 270 },
                { x: 150, y: 210 },
                { x: 106, y: 150 },
              ],
            },
            {
              label:
                "climb back up and rise right over the arch",
              path: [
                { x: 106, y: 150 },
                { x: 148, y: 208 },
                { x: 190, y: 267 },
                { x: 233, y: 325 },
                { x: 283, y: 376 },
                { x: 333, y: 428 },
                { x: 390, y: 472 },
                { x: 446, y: 518 },
                { x: 505, y: 559 },
                { x: 568, y: 593 },
                { x: 638, y: 610 },
              ],
            },
            {
              label:
                "come down and flick up to the right",
              path: [
                { x: 638, y: 610 },
                { x: 708, y: 597 },
                { x: 750, y: 540 },
                { x: 754, y: 468 },
                { x: 748, y: 396 },
                { x: 738, y: 325 },
                { x: 729, y: 253 },
                { x: 722, y: 181 },
                { x: 723, y: 109 },
                { x: 760, y: 50 },
                { x: 831, y: 42 },
                { x: 898, y: 65 },
                { x: 954, y: 110 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("れ"),
    },
  ],
  [
    "japanese:る",
    {
      script: "japanese",
      glyph: "る",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short top bar from left to right",
              path: [
                { x: 276, y: 697 },
                { x: 353, y: 697 },
                { x: 429, y: 701 },
                { x: 505, y: 705 },
                { x: 582, y: 705 },
                { x: 656, y: 693 },
              ],
            },
            {
              label:
                "turn and run the long diagonal down-left",
              path: [
                { x: 656, y: 693 },
                { x: 614, y: 635 },
                { x: 560, y: 586 },
                { x: 504, y: 539 },
                { x: 448, y: 492 },
                { x: 396, y: 441 },
                { x: 343, y: 390 },
                { x: 283, y: 348 },
                { x: 226, y: 302 },
                { x: 172, y: 253 },
              ],
            },
            {
              label:
                "swing back right and round the big bowl",
              path: [
                { x: 172, y: 253 },
                { x: 227, y: 299 },
                { x: 282, y: 345 },
                { x: 341, y: 387 },
                { x: 406, y: 413 },
                { x: 477, y: 418 },
                { x: 549, y: 425 },
                { x: 620, y: 421 },
                { x: 689, y: 401 },
                { x: 748, y: 361 },
                { x: 795, y: 307 },
                { x: 813, y: 238 },
                { x: 810, y: 167 },
                { x: 777, y: 103 },
                { x: 727, y: 52 },
                { x: 663, y: 19 },
                { x: 595, y: 2 },
                { x: 523, y: -7 },
                { x: 452, y: -3 },
              ],
            },
            {
              label:
                "curl into the small loop at the base",
              path: [
                { x: 452, y: -3 },
                { x: 388, y: 17 },
                { x: 341, y: 64 },
                { x: 336, y: 130 },
                { x: 381, y: 180 },
                { x: 444, y: 197 },
                { x: 511, y: 188 },
                { x: 566, y: 151 },
                { x: 602, y: 94 },
                { x: 620, y: 29 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("る"),
    },
  ],
  // き, け, ぬ and へ are fitted from KanjiVG's directed stroke paths for
  // U+304D, U+3051, U+306C and U+3078, the way そ, れ and る are. KanjiVG
  // supplies only the ORDER and DIRECTION. Each path runs along the bundled
  // Noto Sans JP outline's medial line between turning points read off that
  // outline, so the coordinates are the print glyph's own. Every sharp turn
  // inside a stroke is a boundary between two labelled segments, and each join
  // is exact. Where the print glyph has no separate ink for a return (け
  // climbing back from the foot of its left stroke into the flick), the path
  // retraces the ink it came down.
  [
    "japanese:き",
    {
      script: "japanese",
      glyph: "き",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the upper bar from left to right",
              path: [
                { x: 216, y: 648 },
                { x: 283, y: 640 },
                { x: 350, y: 637 },
                { x: 418, y: 636 },
                { x: 485, y: 636 },
                { x: 553, y: 640 },
                { x: 620, y: 649 },
                { x: 686, y: 661 },
                { x: 752, y: 676 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the lower bar from left to right",
              path: [
                { x: 200, y: 440 },
                { x: 269, y: 436 },
                { x: 338, y: 434 },
                { x: 407, y: 432 },
                { x: 476, y: 433 },
                { x: 545, y: 437 },
                { x: 612, y: 450 },
                { x: 681, y: 452 },
                { x: 749, y: 463 },
                { x: 816, y: 480 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "run the long diagonal down to the right",
              path: [
                { x: 524, y: 756 },
                { x: 543, y: 689 },
                { x: 564, y: 622 },
                { x: 582, y: 555 },
                { x: 606, y: 490 },
                { x: 634, y: 427 },
                { x: 663, y: 363 },
                { x: 698, y: 303 },
                { x: 728, y: 240 },
              ],
            },
            {
              label:
                "turn sharply and hook back to the left",
              path: [
                { x: 728, y: 240 },
                { x: 644, y: 236 },
                { x: 560, y: 248 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "curve down and along the base",
              path: [
                { x: 244, y: 232 },
                { x: 228, y: 164 },
                { x: 236, y: 94 },
                { x: 279, y: 39 },
                { x: 342, y: 9 },
                { x: 411, y: -5 },
                { x: 481, y: -8 },
                { x: 552, y: -8 },
                { x: 622, y: -5 },
                { x: 692, y: 4 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("き"),
    },
  ],
  [
    "japanese:け",
    {
      script: "japanese",
      glyph: "け",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the left vertical down",
              path: [
                { x: 203, y: 729 },
                { x: 190, y: 660 },
                { x: 179, y: 590 },
                { x: 170, y: 520 },
                { x: 163, y: 450 },
                { x: 159, y: 379 },
                { x: 159, y: 309 },
                { x: 160, y: 238 },
                { x: 171, y: 169 },
                { x: 195, y: 103 },
                { x: 199, y: 33 },
              ],
            },
            {
              label:
                "turn and flick up to the right",
              path: [
                { x: 199, y: 33 },
                { x: 199, y: 99 },
                { x: 230, y: 156 },
                { x: 253, y: 218 },
                { x: 275, y: 281 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the bar from left to right",
              path: [
                { x: 435, y: 529 },
                { x: 510, y: 525 },
                { x: 585, y: 525 },
                { x: 659, y: 525 },
                { x: 734, y: 532 },
                { x: 809, y: 536 },
                { x: 883, y: 545 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "run down and sweep to the lower left",
              path: [
                { x: 715, y: 741 },
                { x: 719, y: 674 },
                { x: 719, y: 607 },
                { x: 723, y: 540 },
                { x: 723, y: 472 },
                { x: 723, y: 405 },
                { x: 720, y: 338 },
                { x: 716, y: 271 },
                { x: 705, y: 205 },
                { x: 684, y: 141 },
                { x: 650, y: 83 },
                { x: 604, y: 34 },
                { x: 551, y: -7 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("け"),
    },
  ],
  [
    "japanese:ぬ",
    {
      script: "japanese",
      glyph: "ぬ",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short diagonal",
              path: [
                { x: 179, y: 642 },
                { x: 200, y: 575 },
                { x: 225, y: 510 },
                { x: 239, y: 442 },
                { x: 273, y: 382 },
                { x: 302, y: 318 },
                { x: 335, y: 256 },
                { x: 379, y: 202 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "cut down to the lower left",
              path: [
                { x: 543, y: 742 },
                { x: 538, y: 672 },
                { x: 526, y: 604 },
                { x: 519, y: 535 },
                { x: 506, y: 467 },
                { x: 487, y: 399 },
                { x: 464, y: 333 },
                { x: 435, y: 269 },
                { x: 395, y: 213 },
                { x: 360, y: 153 },
                { x: 312, y: 103 },
                { x: 255, y: 62 },
              ],
            },
            {
              label:
                "loop up and over the arch",
              path: [
                { x: 255, y: 62 },
                { x: 187, y: 58 },
                { x: 132, y: 93 },
                { x: 112, y: 157 },
                { x: 114, y: 225 },
                { x: 131, y: 291 },
                { x: 158, y: 354 },
                { x: 196, y: 411 },
                { x: 243, y: 461 },
                { x: 291, y: 509 },
                { x: 346, y: 548 },
                { x: 407, y: 579 },
                { x: 472, y: 599 },
                { x: 538, y: 613 },
                { x: 606, y: 612 },
                { x: 673, y: 599 },
                { x: 735, y: 570 },
              ],
            },
            {
              label:
                "come down the right side",
              path: [
                { x: 735, y: 570 },
                { x: 784, y: 525 },
                { x: 821, y: 471 },
                { x: 841, y: 408 },
                { x: 843, y: 342 },
                { x: 843, y: 276 },
                { x: 832, y: 211 },
                { x: 823, y: 146 },
              ],
            },
            {
              label:
                "tie a small loop and flick out",
              path: [
                { x: 823, y: 146 },
                { x: 782, y: 92 },
                { x: 733, y: 45 },
                { x: 668, y: 23 },
                { x: 600, y: 25 },
                { x: 545, y: 64 },
                { x: 527, y: 128 },
                { x: 559, y: 186 },
                { x: 619, y: 216 },
                { x: 687, y: 215 },
                { x: 752, y: 197 },
                { x: 809, y: 160 },
                { x: 866, y: 122 },
                { x: 915, y: 74 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ぬ"),
    },
  ],
  [
    "japanese:へ",
    {
      script: "japanese",
      glyph: "へ",
      strokes: [
        {
          segments: [
            {
              label:
                "rise up to the peak",
              path: [
                { x: 120, y: 269 },
                { x: 168, y: 320 },
                { x: 213, y: 375 },
                { x: 257, y: 430 },
                { x: 300, y: 486 },
                { x: 348, y: 537 },
                { x: 400, y: 585 },
              ],
            },
            {
              label:
                "run down to the right",
              path: [
                { x: 400, y: 585 },
                { x: 464, y: 564 },
                { x: 514, y: 515 },
                { x: 559, y: 462 },
                { x: 604, y: 408 },
                { x: 649, y: 355 },
                { x: 695, y: 302 },
                { x: 743, y: 251 },
                { x: 791, y: 200 },
                { x: 839, y: 150 },
                { x: 892, y: 105 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("へ"),
    },
  ],
  // ら has been written since chapter 8 (JA-W08-ra) but had no inventory row,
  // so it had no stroke-order source and no ductus. It is fitted here, with
  // chapter 133's four signs, from KanjiVG's directed paths for U+3089 in
  // the same way: KanjiVG supplies the order and direction, and the bundled
  // outline's medial line supplies the coordinates.
  [
    "japanese:ら",
    {
      script: "japanese",
      glyph: "ら",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the top stroke to the right",
              path: [
                { x: 362, y: 737 },
                { x: 424, y: 722 },
                { x: 486, y: 708 },
                { x: 549, y: 696 },
                { x: 611, y: 684 },
                { x: 674, y: 673 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "run down the left side",
              path: [
                { x: 266, y: 569 },
                { x: 259, y: 504 },
                { x: 250, y: 439 },
                { x: 242, y: 374 },
                { x: 238, y: 309 },
                { x: 230, y: 245 },
              ],
            },
            {
              label:
                "turn up and round the open bowl",
              path: [
                { x: 230, y: 245 },
                { x: 278, y: 289 },
                { x: 334, y: 328 },
                { x: 393, y: 363 },
                { x: 457, y: 388 },
                { x: 524, y: 403 },
                { x: 592, y: 407 },
                { x: 660, y: 401 },
                { x: 724, y: 376 },
                { x: 774, y: 329 },
                { x: 805, y: 269 },
                { x: 806, y: 200 },
                { x: 784, y: 136 },
                { x: 738, y: 85 },
                { x: 682, y: 46 },
                { x: 617, y: 22 },
                { x: 550, y: 7 },
                { x: 482, y: 1 },
                { x: 413, y: 1 },
                { x: 346, y: 9 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("ら"),
    },
  ],
  // あ, い, う, え, お and か have been written since chapters 1, 3 and 10
  // (JA-W03-a, JA-W01-i, JA-W03-u, JA-W01-e, JA-W10-o, JA-W03-ka), but their
  // rows had no cited stroke-order source, so they had no ductus and their
  // writing lessons printed no filmstrip. They are fitted from KanjiVG's
  // directed paths for U+3042, U+3044, U+3046, U+3048, U+304A and U+304B the
  // way き, け, ぬ, へ and ら are: KanjiVG supplies only the ORDER and
  // DIRECTION, and the bundled outline's medial line supplies the coordinates.
  // Every sharp turn inside a stroke is a segment boundary and each join is
  // exact. え's hump branches off its diagonal in the print glyph, so where
  // KanjiVG's handwriting climbs back beside the diagonal, the path retraces
  // the diagonal's own ink up to that branch.
  [
    "japanese:あ",
    {
      script: "japanese",
      glyph: "あ",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the bar from left to right",
              path: [
                { x: 181, y: 642 },
                { x: 254, y: 635 },
                { x: 327, y: 634 },
                { x: 401, y: 635 },
                { x: 474, y: 638 },
                { x: 547, y: 643 },
                { x: 620, y: 650 },
                { x: 692, y: 659 },
                { x: 765, y: 670 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the vertical down",
              path: [
                { x: 425, y: 754 },
                { x: 414, y: 683 },
                { x: 405, y: 613 },
                { x: 400, y: 542 },
                { x: 397, y: 470 },
                { x: 385, y: 401 },
                { x: 390, y: 329 },
                { x: 397, y: 258 },
                { x: 407, y: 188 },
                { x: 429, y: 121 },
                { x: 437, y: 50 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "cut down to the lower left",
              path: [
                { x: 677, y: 522 },
                { x: 652, y: 454 },
                { x: 627, y: 385 },
                { x: 594, y: 320 },
                { x: 554, y: 258 },
                { x: 508, y: 201 },
                { x: 453, y: 153 },
                { x: 396, y: 109 },
                { x: 336, y: 67 },
                { x: 266, y: 46 },
                { x: 193, y: 50 },
              ],
            },
            {
              label:
                "loop up across the vertical",
              path: [
                { x: 193, y: 50 },
                { x: 154, y: 105 },
                { x: 153, y: 174 },
                { x: 174, y: 240 },
                { x: 211, y: 298 },
                { x: 259, y: 347 },
                { x: 313, y: 390 },
                { x: 376, y: 417 },
                { x: 437, y: 447 },
                { x: 504, y: 463 },
                { x: 572, y: 470 },
                { x: 641, y: 466 },
              ],
            },
            {
              label:
                "round the right side and finish low",
              path: [
                { x: 641, y: 466 },
                { x: 709, y: 457 },
                { x: 771, y: 427 },
                { x: 821, y: 381 },
                { x: 855, y: 321 },
                { x: 865, y: 254 },
                { x: 856, y: 186 },
                { x: 826, y: 124 },
                { x: 778, y: 75 },
                { x: 723, y: 34 },
                { x: 660, y: 7 },
                { x: 593, y: -10 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("あ"),
    },
  ],
  [
    "japanese:い",
    {
      script: "japanese",
      glyph: "い",
      strokes: [
        {
          segments: [
            {
              label:
                "come down the left side",
              path: [
                { x: 174, y: 659 },
                { x: 174, y: 592 },
                { x: 174, y: 526 },
                { x: 174, y: 459 },
                { x: 178, y: 393 },
                { x: 185, y: 327 },
                { x: 196, y: 261 },
                { x: 213, y: 197 },
                { x: 237, y: 135 },
                { x: 278, y: 83 },
                { x: 330, y: 43 },
              ],
            },
            {
              label:
                "curve round and flick up to the right",
              path: [
                { x: 330, y: 43 },
                { x: 394, y: 52 },
                { x: 442, y: 97 },
                { x: 476, y: 153 },
                { x: 498, y: 215 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the short stroke down",
              path: [
                { x: 726, y: 627 },
                { x: 759, y: 570 },
                { x: 789, y: 511 },
                { x: 813, y: 450 },
                { x: 834, y: 388 },
                { x: 851, y: 324 },
                { x: 865, y: 260 },
                { x: 874, y: 195 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("い"),
    },
  ],
  [
    "japanese:う",
    {
      script: "japanese",
      glyph: "う",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short top stroke to the right",
              path: [
                { x: 330, y: 740 },
                { x: 400, y: 728 },
                { x: 470, y: 720 },
                { x: 541, y: 712 },
                { x: 611, y: 706 },
                { x: 682, y: 700 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "rise to the right along the top",
              path: [
                { x: 214, y: 456 },
                { x: 285, y: 469 },
                { x: 355, y: 488 },
                { x: 426, y: 502 },
                { x: 498, y: 512 },
                { x: 571, y: 512 },
                { x: 642, y: 500 },
              ],
            },
            {
              label:
                "curve down and sweep to the lower left",
              path: [
                { x: 642, y: 500 },
                { x: 704, y: 464 },
                { x: 747, y: 407 },
                { x: 758, y: 336 },
                { x: 755, y: 265 },
                { x: 730, y: 198 },
                { x: 684, y: 142 },
                { x: 631, y: 94 },
                { x: 568, y: 58 },
                { x: 502, y: 31 },
                { x: 433, y: 10 },
                { x: 362, y: 0 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("う"),
    },
  ],
  [
    "japanese:え",
    {
      script: "japanese",
      glyph: "え",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the short top stroke to the right",
              path: [
                { x: 340, y: 746 },
                { x: 405, y: 736 },
                { x: 469, y: 727 },
                { x: 534, y: 718 },
                { x: 599, y: 710 },
                { x: 664, y: 702 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the bar to the right",
              path: [
                { x: 224, y: 478 },
                { x: 296, y: 482 },
                { x: 369, y: 487 },
                { x: 442, y: 493 },
                { x: 514, y: 498 },
                { x: 587, y: 498 },
                { x: 656, y: 506 },
              ],
            },
            {
              label:
                "cut down to the lower left",
              path: [
                { x: 656, y: 506 },
                { x: 610, y: 456 },
                { x: 563, y: 405 },
                { x: 515, y: 357 },
                { x: 467, y: 308 },
                { x: 420, y: 258 },
                { x: 372, y: 210 },
                { x: 323, y: 161 },
                { x: 275, y: 113 },
                { x: 228, y: 62 },
                { x: 180, y: 14 },
              ],
            },
            {
              label:
                "climb back up the diagonal",
              path: [
                { x: 180, y: 14 },
                { x: 230, y: 66 },
                { x: 280, y: 118 },
                { x: 331, y: 169 },
                { x: 382, y: 219 },
                { x: 440, y: 262 },
              ],
            },
            {
              label:
                "turn over the hump and down",
              path: [
                { x: 440, y: 262 },
                { x: 503, y: 245 },
                { x: 554, y: 207 },
                { x: 576, y: 146 },
                { x: 594, y: 83 },
                { x: 624, y: 26 },
              ],
            },
            {
              label:
                "run along the base to the right",
              path: [
                { x: 624, y: 26 },
                { x: 682, y: 4 },
                { x: 744, y: 2 },
                { x: 807, y: 5 },
                { x: 868, y: 14 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("え"),
    },
  ],
  [
    "japanese:お",
    {
      script: "japanese",
      glyph: "お",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the bar from left to right",
              path: [
                { x: 157, y: 580 },
                { x: 227, y: 576 },
                { x: 297, y: 576 },
                { x: 366, y: 582 },
                { x: 436, y: 589 },
                { x: 505, y: 600 },
                { x: 573, y: 616 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "run the long stroke down",
              path: [
                { x: 365, y: 760 },
                { x: 365, y: 692 },
                { x: 365, y: 624 },
                { x: 361, y: 557 },
                { x: 361, y: 489 },
                { x: 361, y: 421 },
                { x: 361, y: 353 },
                { x: 361, y: 285 },
                { x: 361, y: 217 },
                { x: 361, y: 149 },
                { x: 361, y: 82 },
                { x: 337, y: 20 },
              ],
            },
            {
              label:
                "turn and loop up across it",
              path: [
                { x: 337, y: 20 },
                { x: 269, y: 20 },
                { x: 207, y: 45 },
                { x: 159, y: 93 },
                { x: 150, y: 159 },
                { x: 185, y: 216 },
                { x: 234, y: 263 },
                { x: 292, y: 299 },
                { x: 354, y: 324 },
                { x: 416, y: 349 },
                { x: 482, y: 367 },
                { x: 549, y: 378 },
                { x: 617, y: 380 },
              ],
            },
            {
              label:
                "round the right side and finish low",
              path: [
                { x: 617, y: 380 },
                { x: 684, y: 374 },
                { x: 747, y: 349 },
                { x: 798, y: 305 },
                { x: 824, y: 243 },
                { x: 823, y: 176 },
                { x: 795, y: 115 },
                { x: 746, y: 68 },
                { x: 686, y: 38 },
                { x: 620, y: 21 },
                { x: 553, y: 12 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the dot down to the right",
              path: [
                { x: 729, y: 640 },
                { x: 789, y: 609 },
                { x: 846, y: 572 },
                { x: 901, y: 532 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("お"),
    },
  ],
  [
    "japanese:か",
    {
      script: "japanese",
      glyph: "か",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the bar to the right",
              path: [
                { x: 122, y: 517 },
                { x: 196, y: 525 },
                { x: 270, y: 529 },
                { x: 344, y: 534 },
                { x: 417, y: 545 },
                { x: 491, y: 549 },
                { x: 565, y: 540 },
                { x: 622, y: 493 },
              ],
            },
            {
              label:
                "turn down the right side",
              path: [
                { x: 622, y: 493 },
                { x: 635, y: 424 },
                { x: 634, y: 354 },
                { x: 630, y: 284 },
                { x: 622, y: 214 },
                { x: 608, y: 144 },
                { x: 584, y: 78 },
                { x: 538, y: 25 },
              ],
            },
            {
              label:
                "hook back to the left",
              path: [
                { x: 538, y: 25 },
                { x: 457, y: 13 },
                { x: 378, y: 29 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the long stroke down to the left",
              path: [
                { x: 390, y: 749 },
                { x: 379, y: 679 },
                { x: 366, y: 610 },
                { x: 353, y: 541 },
                { x: 336, y: 473 },
                { x: 316, y: 405 },
                { x: 295, y: 338 },
                { x: 272, y: 271 },
                { x: 247, y: 205 },
                { x: 219, y: 140 },
                { x: 188, y: 76 },
                { x: 150, y: 17 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "draw the dot down to the right",
              path: [
                { x: 766, y: 629 },
                { x: 801, y: 573 },
                { x: 832, y: 515 },
                { x: 862, y: 456 },
                { x: 888, y: 395 },
                { x: 910, y: 333 },
              ],
            },
          ],
        },
      ],
      source: strokeSource("か"),
    },
  ],
];
