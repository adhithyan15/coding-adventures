// Authored japanese ductus records. This is the stable source-ownership boundary.

import type { DuctusEntry } from "./registry.ts";
import { SCRIPTS, type ScriptData } from "../scriptdata.ts";

const canonicalScript = (id: string): ScriptData => {
  const inventory = SCRIPTS.find((candidate) => candidate.script === id);
  if (inventory === undefined)
    throw new Error(`Script Ductus has no ${id} inventory`);
  return inventory;
};

const japanese = canonicalScript("japanese");

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
      source: japanese.letters.find((letter) => letter.glyph === "し")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "く")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "た")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "ね")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "み")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "せ")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "て")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "な")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "っ")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "も")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "わ")!
        .strokeOrderSource!,
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
      source: japanese.letters.find((letter) => letter.glyph === "ゆ")!
        .strokeOrderSource!,
    },
  ],
];
