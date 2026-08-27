// Authored gujarati ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import gujarati from "../../../../../learning/human-languages/data/scripts/gujarati.json";

const gujaratiAlphabetSource = (glyph: string): StrokeSource => {
  const letter = gujarati.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Gujarati ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // t30apps animates Gujarati અ as a joined body first, then a separately
  // descending right stem. The fitted medians preserve that one-lift order
  // while following the broader joins and foot of the bundled Noto glyph.
    ["gujarati:અ", {
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
  }],
  // t30apps builds Gujarati આ from the joined અ body, lifts for અ's right
  // stem, then lifts again for the added trailing ā stem. The fitted medians
  // retain that three-run order across the wider bundled Noto glyph.
    ["gujarati:આ", {
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
  }],
  // t30apps animates Gujarati ઇ as one unbroken run through the upper loop,
  // central crossing, broad lower loop, and rising hook. These fitted medians
  // preserve that zero-lift order inside the bundled Noto glyph's wider body.
    ["gujarati:ઇ", {
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
  }],
  // t30apps gives Gujarati ઈ the same unbroken loops as ઇ, then extends the
  // rising hook into a high clockwise curl. The fitted median preserves that
  // zero-lift order across the taller bundled Noto outline.
    ["gujarati:ઈ", {
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
  }],
  // t30apps animates Gujarati ઉ as one unbroken run through its small upper
  // bowl, middle cusp, broad lower bowl, and tall returning outer curve. This
  // fitted median preserves that zero-lift order inside the wider Noto outline.
    ["gujarati:ઉ", {
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
  }],
  // t30apps repeats the complete zero-lift Gujarati ઉ run for ઊ, then carries
  // the same pen across the high shoulder and down the long right-side tail.
  // The fitted median keeps that extension inside the wider Noto outline.
    ["gujarati:ઊ", {
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
  }],
  // t30apps writes Gujarati ઋ as a bent left body, lifts for the central
  // stem, then lifts again for the right loop and descending tail. These
  // medians retain that three-path order inside the bundled Noto outline.
    ["gujarati:ઋ", {
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
  }],
  // t30apps writes Gujarati એ as a joined body, a separately descended right
  // stem, then a separate high arc. These fitted medians preserve that
  // three-path, two-lift order across the wider bundled Noto outline.
    ["gujarati:એ", {
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
  }],
  // t30apps repeats Gujarati એ's body, right stem, and first high arc for ઐ,
  // then adds a fourth, higher arc. These fitted medians preserve that
  // four-path, three-lift order inside the stacked Noto marks.
    ["gujarati:ઐ", {
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
  }],
  // t30apps writes Gujarati ઓ as the complete three-run આ sequence followed
  // by a separate high arc. These fitted medians preserve that four-path,
  // three-lift order across the wider bundled Noto outline.
    ["gujarati:ઓ", {
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
  }],
  // t30apps repeats Gujarati ઓ's body, stems, and first high arc for ઔ, then
  // adds a fifth, higher arc. These fitted medians preserve that five-path,
  // four-lift order inside the bundled Noto glyph's stacked marks.
    ["gujarati:ઔ", {
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
  }],
  // t30apps writes Gujarati ક as a continuous upper-loop-to-lower-body run,
  // then lifts for a rising diagonal cross-stroke. These medians preserve the
  // two-path order while fitting the broader bundled Noto outline.
    ["gujarati:ક", {
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
  }],
  // t30apps writes Gujarati ખ as one joined left-lobe-and-curl run, then
  // lifts for the full-height right spine and its lower foot. These medians
  // preserve the two-path order while fitting the bundled Noto outline.
    ["gujarati:ખ", {
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
  }],
  // t30apps writes Gujarati ગ as one rounded left-body run, then lifts for
  // the full-height right spine and its lower foot. These medians preserve
  // the two-path order while fitting the bundled Noto outline.
    ["gujarati:ગ", {
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
  }],
  // t30apps writes Gujarati ઘ as one joined upper-lobe-to-lower-body run,
  // then lifts for the full-height right spine and its lower foot. These
  // medians preserve the two-path order while fitting the bundled Noto outline.
    ["gujarati:ઘ", {
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
  }],
  // t30apps writes Gujarati ઙ as one long S-like body, then lifts for the
  // compact upper-right dot. These medians preserve the two-path order while
  // fitting the bundled Noto outline.
    ["gujarati:ઙ", {
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
  }],
  // t30apps writes Gujarati ચ as one joined upper-bowl-to-middle-loop-to-lower-
  // body run, then lifts for the full-height right spine and its lower foot.
  // These medians preserve the two-path order while fitting the Noto outline.
    ["gujarati:ચ", {
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
  }],
  // t30apps writes Gujarati છ as one continuous run through the upper-left
  // lobe, lower body, outer-right curve, and upper-right lobe. These connected
  // medians preserve that zero-lift order while fitting the Noto outline.
    ["gujarati:છ", {
    script: "gujarati",
    glyph: "છ",
    strokes: [
      {
        segments: [
          {
            label: "circle the upper-left lobe and turn back through the middle",
            path: [
              { x: 280, y: 560 }, { x: 220, y: 565 }, { x: 160, y: 555 },
              { x: 110, y: 530 }, { x: 80, y: 490 }, { x: 80, y: 450 },
              { x: 100, y: 415 }, { x: 140, y: 385 }, { x: 170, y: 360 },
              { x: 200, y: 345 },
            ],
          },
          {
            label: "continue around the broad lower body and climb the outer right curve",
            path: [
              { x: 200, y: 345 }, { x: 160, y: 315 }, { x: 135, y: 275 },
              { x: 125, y: 225 },
              { x: 135, y: 170 }, { x: 170, y: 115 }, { x: 225, y: 75 },
              { x: 290, y: 45 }, { x: 360, y: 42 }, { x: 425, y: 65 },
              { x: 530, y: 100 }, { x: 575, y: 140 }, { x: 605, y: 180 },
              { x: 625, y: 220 }, { x: 640, y: 260 }, { x: 650, y: 300 },
              { x: 660, y: 350 },
            ],
          },
          {
            label: "circle the upper-right lobe and finish beside the outer curve",
            path: [
              { x: 660, y: 350 }, { x: 630, y: 400 }, { x: 620, y: 455 },
              { x: 590, y: 515 }, { x: 545, y: 555 }, { x: 495, y: 560 },
              { x: 455, y: 535 }, { x: 430, y: 495 }, { x: 425, y: 450 },
              { x: 445, y: 410 }, { x: 480, y: 380 }, { x: 520, y: 360 },
              { x: 565, y: 345 }, { x: 620, y: 340 }, { x: 660, y: 350 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("છ"),
  }],
  // t30apps writes Gujarati જ as one continuous left-loop-to-crossing-to-right-
  // loop-to-exit run. These connected medians preserve that zero-lift order
  // while fitting the bundled Noto outline.
    ["gujarati:જ", {
    script: "gujarati",
    glyph: "જ",
    strokes: [
      {
        segments: [
          {
            label: "circle the upper-left loop",
            path: [
              { x: 300, y: 550 }, { x: 315, y: 510 }, { x: 315, y: 470 },
              { x: 315, y: 430 }, { x: 300, y: 390 }, { x: 275, y: 350 },
              { x: 230, y: 315 }, { x: 180, y: 300 }, { x: 130, y: 320 },
              { x: 90, y: 360 }, { x: 75, y: 400 }, { x: 75, y: 450 },
              { x: 90, y: 500 }, { x: 130, y: 535 }, { x: 190, y: 560 },
              { x: 250, y: 560 }, { x: 300, y: 550 },
            ],
          },
          {
            label: "continue diagonally through the crossing body",
            path: [
              { x: 300, y: 550 }, { x: 320, y: 530 }, { x: 350, y: 520 },
              { x: 385, y: 510 }, { x: 415, y: 500 }, { x: 435, y: 490 },
              { x: 450, y: 480 }, { x: 470, y: 470 }, { x: 490, y: 460 },
              { x: 500, y: 450 }, { x: 520, y: 430 }, { x: 540, y: 410 },
              { x: 560, y: 390 }, { x: 580, y: 370 }, { x: 630, y: 310 },
              { x: 670, y: 240 },
              { x: 680, y: 170 }, { x: 675, y: 110 }, { x: 650, y: 75 },
            ],
          },
          {
            label: "circle the lower-right loop and sweep into the upper-right exit",
            path: [
              { x: 650, y: 75 }, { x: 600, y: 45 }, { x: 545, y: 35 },
              { x: 490, y: 50 }, { x: 450, y: 85 }, { x: 420, y: 130 },
              { x: 415, y: 180 }, { x: 430, y: 230 }, { x: 460, y: 275 },
              { x: 500, y: 320 }, { x: 550, y: 365 }, { x: 610, y: 410 },
              { x: 680, y: 460 }, { x: 750, y: 510 },
              { x: 790, y: 545 },
            ],
          },
        ],
      },
    ],
    source: gujaratiAlphabetSource("જ"),
  }],
  // t30apps writes Gujarati ઝ as a rounded left body, then lifts for the right
  // loop and tail, then lifts again for the short upper stem. These medians
  // preserve the three-path order while fitting the bundled Noto outline.
    ["gujarati:ઝ", {
    script: "gujarati",
    glyph: "ઝ",
    strokes: [
      { segments: [{ label: "circle the rounded left body from upper left to lower left", path: [
        { x: 80, y: 430 }, { x: 140, y: 455 }, { x: 210, y: 455 },
        { x: 270, y: 430 }, { x: 320, y: 380 }, { x: 350, y: 320 },
        { x: 355, y: 250 }, { x: 345, y: 180 }, { x: 320, y: 120 },
        { x: 280, y: 75 }, { x: 230, y: 45 }, { x: 180, y: 50 },
        { x: 130, y: 75 }, { x: 90, y: 110 }, { x: 55, y: 150 },
      ] }] },
      { segments: [{ label: "lift, then circle the right loop and finish through its lower tail", path: [
        { x: 370, y: 320 }, { x: 420, y: 360 }, { x: 470, y: 375 },
        { x: 520, y: 360 }, { x: 570, y: 330 }, { x: 610, y: 285 },
        { x: 620, y: 230 }, { x: 615, y: 170 }, { x: 590, y: 110 },
        { x: 550, y: 65 }, { x: 500, y: 35 },
      ] }] },
      { segments: [{ label: "lift again, then descend the short upper stem", path: [
        { x: 496, y: 560 }, { x: 496, y: 500 }, { x: 496, y: 440 },
        { x: 496, y: 380 }, { x: 496, y: 345 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ઝ"),
  }],
  // t30apps writes Gujarati ઞ as a rounded left body, then lifts for the short
  // rightward shoulder, then lifts again for the tall spine and lower terminal.
  // These medians preserve the three-path order while fitting the Noto outline.
    ["gujarati:ઞ", {
    script: "gujarati",
    glyph: "ઞ",
    strokes: [
      { segments: [{ label: "circle the rounded left body from upper left to lower left", path: [
        { x: 80, y: 500 }, { x: 135, y: 545 }, { x: 195, y: 560 },
        { x: 250, y: 540 }, { x: 295, y: 500 }, { x: 325, y: 445 },
        { x: 335, y: 385 }, { x: 325, y: 325 }, { x: 290, y: 270 },
        { x: 240, y: 225 }, { x: 185, y: 215 }, { x: 130, y: 235 },
        { x: 85, y: 265 }, { x: 55, y: 305 },
      ] }] },
      { segments: [{ label: "lift, then sweep the short rightward shoulder", path: [
        { x: 365, y: 370 }, { x: 405, y: 370 }, { x: 445, y: 375 },
        { x: 480, y: 390 }, { x: 520, y: 415 },
      ] }] },
      { segments: [{ label: "lift again, then descend the tall spine and curl through its terminal", path: [
        { x: 557, y: 560 }, { x: 557, y: 490 }, { x: 557, y: 420 },
        { x: 557, y: 350 }, { x: 557, y: 280 }, { x: 557, y: 210 },
        { x: 557, y: 145 }, { x: 565, y: 90 }, { x: 590, y: 55 },
        { x: 625, y: 40 }, { x: 665, y: 40 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ઞ"),
  }],
  // t30apps writes Gujarati ટ as one unbroken run across the upper turn,
  // diagonally through the middle, and clockwise around the lower bowl. This
  // median preserves that order while fitting the bundled Noto outline.
    ["gujarati:ટ", {
    script: "gujarati",
    glyph: "ટ",
    strokes: [
      { segments: [{ label: "sweep the upper turn, bend down-left, and circle the lower bowl", path: [
        { x: 105, y: 520 }, { x: 155, y: 555 }, { x: 215, y: 565 },
        { x: 275, y: 555 }, { x: 325, y: 530 }, { x: 355, y: 495 },
        { x: 360, y: 455 }, { x: 345, y: 420 }, { x: 315, y: 390 },
        { x: 270, y: 355 }, { x: 220, y: 325 }, { x: 165, y: 290 },
        { x: 115, y: 250 }, { x: 85, y: 210 }, { x: 80, y: 165 },
        { x: 95, y: 120 }, { x: 130, y: 80 }, { x: 180, y: 50 },
        { x: 235, y: 35 }, { x: 295, y: 40 }, { x: 345, y: 55 },
        { x: 390, y: 80 }, { x: 420, y: 110 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ટ"),
  }],
  // t30apps writes Gujarati ઠ as one continuous run: sweep the high shoulder
  // from right to left, descend through the middle into the outer lower bowl,
  // then curl back to the inner terminal. This median fits that order to Noto.
    ["gujarati:ઠ", {
    script: "gujarati",
    glyph: "ઠ",
    strokes: [
      { segments: [{ label: "sweep left across the shoulder, circle the lower bowl, and curl inward", path: [
        { x: 400, y: 565 }, { x: 350, y: 575 }, { x: 300, y: 575 },
        { x: 250, y: 565 }, { x: 205, y: 545 }, { x: 175, y: 515 },
        { x: 165, y: 480 }, { x: 180, y: 445 }, { x: 220, y: 410 },
        { x: 275, y: 375 }, { x: 335, y: 340 }, { x: 390, y: 295 },
        { x: 430, y: 245 }, { x: 455, y: 190 }, { x: 455, y: 135 },
        { x: 440, y: 90 }, { x: 410, y: 55 }, { x: 365, y: 30 },
        { x: 310, y: 20 }, { x: 250, y: 20 }, { x: 190, y: 35 },
        { x: 140, y: 65 }, { x: 105, y: 105 }, { x: 85, y: 150 },
        { x: 85, y: 195 }, { x: 100, y: 235 }, { x: 130, y: 270 },
        { x: 170, y: 295 }, { x: 215, y: 310 }, { x: 260, y: 315 },
        { x: 305, y: 310 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ઠ"),
  }],
  // t30apps writes Gujarati ડ as one unbroken descending sweep: the high
  // shoulder runs right-to-left, turns through the middle, and continues
  // around the lower bowl to its lower-left terminal. The Noto fit follows it.
    ["gujarati:ડ", {
    script: "gujarati",
    glyph: "ડ",
    strokes: [
      { segments: [{ label: "sweep left across the shoulder, descend through the middle, and round the lower bowl", path: [
        { x: 390, y: 565 }, { x: 340, y: 575 }, { x: 285, y: 575 },
        { x: 235, y: 565 }, { x: 195, y: 545 }, { x: 165, y: 515 },
        { x: 150, y: 480 }, { x: 160, y: 445 }, { x: 190, y: 415 },
        { x: 235, y: 385 }, { x: 285, y: 355 }, { x: 335, y: 325 },
        { x: 380, y: 290 }, { x: 415, y: 250 }, { x: 430, y: 205 },
        { x: 430, y: 155 }, { x: 420, y: 110 }, { x: 395, y: 70 },
        { x: 360, y: 40 }, { x: 315, y: 20 }, { x: 265, y: 20 },
        { x: 215, y: 30 }, { x: 170, y: 50 }, { x: 130, y: 80 },
        { x: 95, y: 115 }, { x: 65, y: 155 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ડ"),
  }],
  // t30apps writes Gujarati ઢ as one continuous path: the upper shoulder flows
  // through the middle into the outer lower bowl, then turns directly around
  // the small inner loop. The median keeps that no-lift order inside Noto.
    ["gujarati:ઢ", {
    script: "gujarati",
    glyph: "ઢ",
    strokes: [
      { segments: [{ label: "sweep the upper shoulder, round the outer bowl, and circle the inner loop", path: [
        { x: 125, y: 560 }, { x: 175, y: 575 }, { x: 230, y: 580 },
        { x: 285, y: 575 }, { x: 335, y: 555 }, { x: 375, y: 525 },
        { x: 390, y: 490 }, { x: 380, y: 455 }, { x: 350, y: 420 },
        { x: 305, y: 390 }, { x: 255, y: 360 }, { x: 205, y: 330 },
        { x: 155, y: 300 }, { x: 115, y: 265 }, { x: 85, y: 225 },
        { x: 70, y: 180 }, { x: 75, y: 130 }, { x: 95, y: 85 },
        { x: 130, y: 50 }, { x: 175, y: 25 }, { x: 225, y: 20 },
        { x: 275, y: 30 }, { x: 320, y: 50 }, { x: 360, y: 40 },
        { x: 400, y: 60 }, { x: 430, y: 90 }, { x: 450, y: 125 },
        { x: 450, y: 165 }, { x: 430, y: 195 }, { x: 400, y: 225 },
        { x: 360, y: 240 }, { x: 320, y: 225 }, { x: 290, y: 195 },
        { x: 275, y: 160 }, { x: 275, y: 120 }, { x: 285, y: 85 },
        { x: 300, y: 60 }, { x: 325, y: 45 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ઢ"),
  }],
  // t30apps writes Gujarati ણ in three runs: the left spine and hooked tail,
  // the separate middle bowl, then the tall right spine and foot. These Noto
  // medians retain the observed body-before-bowl-before-spine order.
    ["gujarati:ણ", {
    script: "gujarati",
    glyph: "ણ",
    strokes: [
      { segments: [{ label: "descend the left spine and sweep through the hooked lower tail", path: [
        { x: 115, y: 590 }, { x: 115, y: 520 }, { x: 115, y: 440 },
        { x: 115, y: 360 }, { x: 115, y: 285 }, { x: 120, y: 220 },
        { x: 140, y: 170 }, { x: 180, y: 130 }, { x: 230, y: 100 },
        { x: 285, y: 75 }, { x: 340, y: 45 }, { x: 385, y: 10 },
        { x: 415, y: -30 }, { x: 425, y: -70 }, { x: 410, y: -105 },
      ] }] },
      { segments: [{ label: "lift, then circle the separate middle bowl", path: [
        { x: 280, y: 555 }, { x: 330, y: 575 }, { x: 385, y: 575 },
        { x: 435, y: 555 }, { x: 475, y: 520 }, { x: 500, y: 470 },
        { x: 510, y: 410 }, { x: 505, y: 350 }, { x: 485, y: 305 },
        { x: 450, y: 270 }, { x: 410, y: 250 }, { x: 365, y: 245 },
        { x: 320, y: 260 }, { x: 285, y: 290 }, { x: 260, y: 325 },
      ] }] },
      { segments: [{ label: "lift again, descend the tall right spine, and turn through its foot", path: [
        { x: 690, y: 590 }, { x: 690, y: 520 }, { x: 690, y: 440 },
        { x: 690, y: 360 }, { x: 690, y: 280 }, { x: 690, y: 200 },
        { x: 690, y: 145 }, { x: 700, y: 105 }, { x: 725, y: 75 },
        { x: 760, y: 55 }, { x: 795, y: 45 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ણ"),
  }],
  // t30apps writes Gujarati ત in two runs: the open left body and shoulder,
  // then the separate tall right spine and foot. These Noto medians retain the
  // observed body-before-spine order while fitting the wider printed glyph.
    ["gujarati:ત", {
    script: "gujarati",
    glyph: "ત",
    strokes: [
      { segments: [{ label: "sweep from the lower terminal around the open body and across the upper shoulder", path: [
        { x: 215, y: 45 }, { x: 185, y: 75 }, { x: 150, y: 110 },
        { x: 120, y: 150 }, { x: 95, y: 195 }, { x: 85, y: 245 },
        { x: 88, y: 295 }, { x: 105, y: 335 }, { x: 135, y: 370 },
        { x: 175, y: 392 }, { x: 225, y: 402 }, { x: 280, y: 402 },
        { x: 335, y: 402 }, { x: 380, y: 402 }, { x: 420, y: 402 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 440, y: 585 }, { x: 440, y: 520 }, { x: 440, y: 450 },
        { x: 440, y: 380 }, { x: 440, y: 310 }, { x: 440, y: 240 },
        { x: 440, y: 175 }, { x: 442, y: 125 }, { x: 450, y: 90 },
        { x: 468, y: 62 }, { x: 495, y: 45 }, { x: 530, y: 38 },
        { x: 550, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ત"),
  }],
  // t30apps writes Gujarati થ in two runs: the small loop and broad body through
  // the right shoulder, then the separate tall spine and foot. These Noto
  // medians preserve the observed loop-and-body-before-spine order.
    ["gujarati:થ", {
    script: "gujarati",
    glyph: "થ",
    strokes: [
      { segments: [{ label: "circle the small upper loop, descend, and sweep around the broad body into the right shoulder", path: [
        { x: 270, y: 490 }, { x: 255, y: 530 }, { x: 225, y: 560 },
        { x: 180, y: 570 }, { x: 135, y: 555 }, { x: 105, y: 525 },
        { x: 95, y: 485 }, { x: 108, y: 450 }, { x: 140, y: 425 },
        { x: 108, y: 450 }, { x: 95, y: 485 }, { x: 105, y: 525 },
        { x: 135, y: 555 }, { x: 180, y: 570 }, { x: 225, y: 560 },
        { x: 255, y: 530 }, { x: 270, y: 490 }, { x: 270, y: 450 },
        { x: 270, y: 410 }, { x: 270, y: 390 }, { x: 250, y: 350 },
        { x: 215, y: 315 }, { x: 180, y: 300 }, { x: 145, y: 290 },
        { x: 120, y: 285 }, { x: 85, y: 305 }, { x: 100, y: 260 },
        { x: 130, y: 220 },
        { x: 175, y: 190 }, { x: 230, y: 175 }, { x: 290, y: 178 },
        { x: 345, y: 195 }, { x: 390, y: 225 }, { x: 430, y: 265 },
        { x: 465, y: 295 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 490, y: 585 }, { x: 490, y: 520 }, { x: 490, y: 450 },
        { x: 490, y: 380 }, { x: 490, y: 310 }, { x: 490, y: 240 },
        { x: 490, y: 175 }, { x: 492, y: 125 }, { x: 500, y: 90 },
        { x: 520, y: 62 }, { x: 548, y: 45 }, { x: 580, y: 38 },
        { x: 603, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("થ"),
  }],
  // t30apps writes Gujarati દ in one continuous run: around the upper body,
  // through its narrow middle turn, then around the lower body to the terminal.
  // This Noto median retains that observed upper-before-lower order.
    ["gujarati:દ", {
    script: "gujarati",
    glyph: "દ",
    strokes: [
      { segments: [{ label: "circle the upper body, narrow through the middle, and sweep around the lower body into its terminal", path: [
        { x: 330, y: 555 }, { x: 285, y: 570 }, { x: 235, y: 572 },
        { x: 185, y: 565 }, { x: 140, y: 545 }, { x: 105, y: 515 },
        { x: 85, y: 475 }, { x: 85, y: 435 }, { x: 100, y: 400 },
        { x: 125, y: 370 }, { x: 160, y: 350 }, { x: 205, y: 335 },
        { x: 240, y: 325 }, { x: 290, y: 325 }, { x: 340, y: 325 },
        { x: 375, y: 320 }, { x: 330, y: 320 }, { x: 280, y: 320 },
        { x: 240, y: 325 }, { x: 205, y: 310 }, { x: 170, y: 290 },
        { x: 140, y: 260 }, { x: 120, y: 225 }, { x: 110, y: 185 },
        { x: 115, y: 145 }, { x: 135, y: 105 }, { x: 170, y: 75 },
        { x: 215, y: 50 }, { x: 265, y: 35 }, { x: 315, y: 35 },
        { x: 355, y: 45 }, { x: 385, y: 65 }, { x: 405, y: 90 },
        { x: 415, y: 115 }, { x: 440, y: 120 }, { x: 452, y: 105 },
        { x: 452, y: 85 }, { x: 442, y: 70 }, { x: 420, y: 65 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("દ"),
  }],
  // t30apps writes Gujarati ધ in two runs: the high-entry joined body through
  // its right shoulder, then the separate tall spine and foot. These Noto
  // medians preserve the observed body-before-spine order.
    ["gujarati:ધ", {
    script: "gujarati",
    glyph: "ધ",
    strokes: [
      { segments: [{ label: "descend from the high entry through the turns and sweep around the broad body into the right shoulder", path: [
        { x: 210, y: 620 }, { x: 175, y: 600 }, { x: 140, y: 575 },
        { x: 110, y: 545 }, { x: 90, y: 510 }, { x: 82, y: 475 },
        { x: 88, y: 440 }, { x: 105, y: 410 }, { x: 130, y: 385 },
        { x: 165, y: 365 }, { x: 210, y: 355 }, { x: 260, y: 360 },
        { x: 305, y: 375 }, { x: 275, y: 370 }, { x: 235, y: 355 },
        { x: 195, y: 335 }, { x: 165, y: 305 }, { x: 145, y: 270 },
        { x: 140, y: 230 }, { x: 150, y: 195 }, { x: 175, y: 165 },
        { x: 210, y: 145 }, { x: 255, y: 135 }, { x: 300, y: 140 },
        { x: 345, y: 155 }, { x: 385, y: 180 }, { x: 420, y: 215 },
        { x: 450, y: 245 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 471, y: 585 }, { x: 471, y: 520 }, { x: 471, y: 450 },
        { x: 471, y: 380 }, { x: 471, y: 310 }, { x: 471, y: 240 },
        { x: 471, y: 175 }, { x: 473, y: 125 }, { x: 481, y: 90 },
        { x: 500, y: 62 }, { x: 530, y: 45 }, { x: 562, y: 38 },
        { x: 585, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ધ"),
  }],
  // t30apps writes Gujarati ન in two runs: the small left loop through the long
  // shoulder, then the separate tall spine and foot. These Noto medians retain
  // the observed loop-and-shoulder-before-spine order.
    ["gujarati:ન", {
    script: "gujarati",
    glyph: "ન",
    strokes: [
      { segments: [{ label: "circle the small left loop and continue across the long rightward shoulder", path: [
        { x: 110, y: 365 }, { x: 145, y: 365 }, { x: 170, y: 340 },
        { x: 172, y: 305 }, { x: 160, y: 270 }, { x: 135, y: 245 },
        { x: 105, y: 245 }, { x: 75, y: 265 }, { x: 55, y: 295 },
        { x: 50, y: 330 }, { x: 62, y: 355 }, { x: 85, y: 370 },
        { x: 110, y: 365 }, { x: 160, y: 365 }, { x: 215, y: 365 },
        { x: 270, y: 365 }, { x: 325, y: 365 }, { x: 390, y: 365 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 412, y: 585 }, { x: 412, y: 520 }, { x: 412, y: 450 },
        { x: 412, y: 380 }, { x: 412, y: 310 }, { x: 412, y: 240 },
        { x: 412, y: 175 }, { x: 414, y: 125 }, { x: 422, y: 90 },
        { x: 440, y: 62 }, { x: 470, y: 45 }, { x: 502, y: 38 },
        { x: 525, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ન"),
  }],
  // t30apps writes Gujarati પ in two runs: the high left hook through the
  // broad lower body and right shoulder, then the separate tall spine and foot.
  // These Noto medians retain that hooked-body-before-spine order.
    ["gujarati:પ", {
    script: "gujarati",
    glyph: "પ",
    strokes: [
      { segments: [{ label: "curl over the high left hook, descend, and sweep around the broad lower body into the right shoulder", path: [
        { x: 20, y: 520 }, { x: 25, y: 555 }, { x: 55, y: 570 },
        { x: 90, y: 565 }, { x: 120, y: 540 }, { x: 133, y: 500 },
        { x: 133, y: 450 }, { x: 133, y: 390 }, { x: 133, y: 340 },
        { x: 140, y: 300 }, { x: 160, y: 260 }, { x: 190, y: 225 },
        { x: 225, y: 205 }, { x: 270, y: 197 }, { x: 315, y: 207 },
        { x: 350, y: 225 }, { x: 380, y: 255 }, { x: 410, y: 285 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 433, y: 585 }, { x: 433, y: 520 }, { x: 433, y: 450 },
        { x: 433, y: 380 }, { x: 433, y: 310 }, { x: 433, y: 240 },
        { x: 433, y: 175 }, { x: 435, y: 125 }, { x: 443, y: 90 },
        { x: 460, y: 62 }, { x: 490, y: 45 }, { x: 525, y: 38 },
        { x: 550, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("પ"),
  }],
  // t30apps writes Gujarati ફ in two runs: the winding main body through its
  // lower-left loop and tail, then the separate rising diagonal cross-stroke.
  // These Noto medians retain that complete-body-before-cross-stroke order.
    ["gujarati:ફ", {
    script: "gujarati",
    glyph: "ફ",
    strokes: [
      { segments: [{ label: "sweep left across the high cap, wind around the body and lower-left loop, then exit through the tail", path: [
        { x: 390, y: 580 }, { x: 360, y: 592 }, { x: 319, y: 597 },
        { x: 253, y: 597 }, { x: 200, y: 590 }, { x: 160, y: 560 },
        { x: 125, y: 525 }, { x: 145, y: 460 }, { x: 169, y: 387 },
        { x: 262, y: 340 }, { x: 300, y: 320 }, { x: 340, y: 295 },
        { x: 390, y: 260 }, { x: 415, y: 230 }, { x: 430, y: 190 },
        { x: 435, y: 145 }, { x: 425, y: 100 }, { x: 400, y: 60 },
        { x: 375, y: 30 }, { x: 285, y: 5 }, { x: 170, y: 3 },
        { x: 101, y: 19 }, { x: 57, y: 45 }, { x: 44, y: 73 },
        { x: 75, y: 97 }, { x: 135, y: 82 }, { x: 193, y: 36 },
        { x: 240, y: -35 }, { x: 270, y: -65 }, { x: 330, y: -110 },
        { x: 370, y: -135 }, { x: 395, y: -140 },
      ] }] },
      { segments: [{ label: "lift and draw the diagonal cross-stroke from lower left to upper right", path: [
        { x: 70, y: 245 }, { x: 130, y: 270 }, { x: 200, y: 300 },
        { x: 270, y: 330 }, { x: 340, y: 360 }, { x: 410, y: 390 },
        { x: 470, y: 420 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ફ"),
  }],
  // t30apps writes Gujarati બ in two runs: the rounded body winds through its
  // inner turn into the right shoulder, then the separate tall spine descends.
  // These Noto medians retain that complete-body-before-right-spine order.
    ["gujarati:બ", {
    script: "gujarati",
    glyph: "બ",
    strokes: [
      { segments: [{ label: "circle the rounded body, wind through the inner turn, and exit across the right shoulder", path: [
        { x: 270, y: 565 }, { x: 220, y: 550 }, { x: 175, y: 525 }, { x: 135, y: 490 },
        { x: 105, y: 445 }, { x: 88, y: 395 }, { x: 85, y: 345 }, { x: 92, y: 295 },
        { x: 115, y: 250 }, { x: 150, y: 215 }, { x: 195, y: 190 }, { x: 235, y: 185 },
        { x: 275, y: 188 }, { x: 310, y: 205 }, { x: 335, y: 235 }, { x: 355, y: 275 },
        { x: 372, y: 320 }, { x: 377, y: 365 }, { x: 377, y: 410 }, { x: 377, y: 440 },
        { x: 377, y: 395 }, { x: 377, y: 350 }, { x: 390, y: 305 }, { x: 420, y: 275 },
        { x: 460, y: 265 }, { x: 500, y: 275 }, { x: 535, y: 295 }, { x: 560, y: 315 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 600, y: 585 }, { x: 600, y: 520 }, { x: 600, y: 450 }, { x: 600, y: 380 },
        { x: 600, y: 310 }, { x: 600, y: 240 }, { x: 600, y: 175 }, { x: 600, y: 125 },
        { x: 605, y: 90 }, { x: 620, y: 62 }, { x: 650, y: 45 }, { x: 690, y: 38 },
        { x: 720, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("બ"),
  }],
  // t30apps writes Gujarati ભ as a joined loop and inner turn followed by a
  // separately descended right spine.
    ["gujarati:ભ", {
    script: "gujarati", glyph: "ભ",
    strokes: [
      { segments: [{ label: "circle the broad loop, wind through the inner turn, and exit across the long right shoulder", path: [
        { x: 270, y: 75 }, { x: 220, y: 120 }, { x: 170, y: 175 }, { x: 125, y: 240 },
        { x: 90, y: 310 }, { x: 85, y: 385 }, { x: 105, y: 455 }, { x: 145, y: 520 },
        { x: 195, y: 555 }, { x: 255, y: 565 }, { x: 315, y: 550 }, { x: 365, y: 510 },
        { x: 400, y: 455 }, { x: 410, y: 390 }, { x: 410, y: 330 }, { x: 405, y: 275 },
        { x: 390, y: 240 }, { x: 370, y: 240 }, { x: 340, y: 260 }, { x: 320, y: 290 },
        { x: 320, y: 325 }, { x: 340, y: 330 }, { x: 390, y: 329 }, { x: 450, y: 329 },
        { x: 520, y: 329 }, { x: 580, y: 329 }, { x: 634, y: 329 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 674, y: 585 }, { x: 674, y: 500 }, { x: 674, y: 400 }, { x: 674, y: 300 },
        { x: 674, y: 200 }, { x: 674, y: 125 }, { x: 680, y: 85 }, { x: 700, y: 60 },
        { x: 735, y: 42 }, { x: 775, y: 38 }, { x: 795, y: 38 },
      ] }] },
    ],
    source: gujaratiAlphabetSource("ભ"),
  }],
  // t30apps writes Gujarati મ as a joined left body and shoulder followed by
  // a separately descended right spine.
    ["gujarati:મ", {
    script: "gujarati", glyph: "મ",
    strokes: [
      { segments: [{ label: "curl through the left body and inner turn, then exit across the long right shoulder", path: [
        { x: 50, y: 556 }, { x: 90, y: 560 }, { x: 130, y: 550 }, { x: 165, y: 520 },
        { x: 170, y: 480 }, { x: 170, y: 430 }, { x: 170, y: 380 }, { x: 170, y: 330 },
        { x: 170, y: 280 }, { x: 165, y: 230 }, { x: 150, y: 195 }, { x: 130, y: 190 },
        { x: 100, y: 210 }, { x: 80, y: 240 }, { x: 80, y: 275 }, { x: 105, y: 280 },
        { x: 150, y: 280 }, { x: 210, y: 280 }, { x: 280, y: 280 }, { x: 350, y: 280 }, { x: 420, y: 280 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 460, y: 585 }, { x: 460, y: 500 }, { x: 460, y: 400 }, { x: 460, y: 300 },
        { x: 460, y: 200 }, { x: 460, y: 125 }, { x: 468, y: 80 }, { x: 490, y: 55 },
        { x: 530, y: 40 }, { x: 570, y: 38 }, { x: 582, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("મ"),
  }],
  // t30apps writes Gujarati ય as a joined rounded body and long shoulder
  // followed by a separately descended right spine.
    ["gujarati:ય", {
    script: "gujarati", glyph: "ય",
    strokes: [
      { segments: [{ label: "circle the rounded upper turn and sweep around the broad lower body into the long right shoulder", path: [
        { x: 35, y: 565 }, { x: 75, y: 575 }, { x: 120, y: 565 }, { x: 160, y: 540 }, { x: 190, y: 500 },
        { x: 200, y: 455 }, { x: 195, y: 415 }, { x: 175, y: 380 }, { x: 145, y: 355 }, { x: 105, y: 335 },
        { x: 75, y: 310 }, { x: 90, y: 275 }, { x: 120, y: 240 }, { x: 160, y: 210 }, { x: 205, y: 190 },
        { x: 255, y: 180 }, { x: 305, y: 185 }, { x: 350, y: 205 }, { x: 385, y: 235 }, { x: 415, y: 270 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 459, y: 585 }, { x: 459, y: 500 }, { x: 459, y: 400 }, { x: 459, y: 300 },
        { x: 459, y: 200 }, { x: 459, y: 125 }, { x: 467, y: 80 }, { x: 490, y: 55 },
        { x: 525, y: 40 }, { x: 560, y: 38 }, { x: 578, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("ય"),
  }],
  // t30apps writes Gujarati ર in one run through the rounded upper body,
  // middle loop, and descending tail.
    ["gujarati:ર", {
    script: "gujarati", glyph: "ર",
    strokes: [
      { segments: [{ label: "circle the rounded upper body, curl through the middle loop, and descend into the lower-right tail", path: [
        { x: 45, y: 555 }, { x: 90, y: 575 }, { x: 145, y: 580 }, { x: 200, y: 570 }, { x: 250, y: 545 },
        { x: 295, y: 510 }, { x: 325, y: 465 }, { x: 340, y: 415 }, { x: 335, y: 365 }, { x: 315, y: 320 },
        { x: 280, y: 285 }, { x: 235, y: 260 }, { x: 190, y: 245 }, { x: 150, y: 240 }, { x: 120, y: 250 },
        { x: 95, y: 265 }, { x: 75, y: 250 }, { x: 85, y: 225 }, { x: 120, y: 215 }, { x: 150, y: 220 },
        { x: 165, y: 185 }, { x: 185, y: 145 }, { x: 215, y: 105 }, { x: 250, y: 72 }, { x: 295, y: 50 },
        { x: 345, y: 38 }, { x: 395, y: 38 }, { x: 420, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("ર"),
  }],
  // t30apps writes Gujarati લ as the broad left body, the separate middle
  // shoulder, and then the separate tall right spine.
    ["gujarati:લ", {
    script: "gujarati", glyph: "લ",
    strokes: [
      { segments: [{ label: "circle counterclockwise around the broad rounded left body", path: [
        { x: 300, y: 565 }, { x: 245, y: 550 }, { x: 190, y: 525 }, { x: 145, y: 490 }, { x: 110, y: 445 },
        { x: 90, y: 390 }, { x: 85, y: 335 }, { x: 95, y: 280 }, { x: 120, y: 230 }, { x: 160, y: 190 },
        { x: 210, y: 160 }, { x: 265, y: 145 }, { x: 320, y: 150 }, { x: 370, y: 170 },
      ] }] },
      { segments: [{ label: "lift and sweep the middle shoulder from left to right", path: [
        { x: 235, y: 340 }, { x: 285, y: 350 }, { x: 335, y: 360 }, { x: 385, y: 370 }, { x: 430, y: 380 }, { x: 475, y: 390 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 517, y: 585 }, { x: 517, y: 500 }, { x: 517, y: 400 }, { x: 517, y: 300 },
        { x: 517, y: 200 }, { x: 517, y: 125 }, { x: 525, y: 80 }, { x: 550, y: 55 },
        { x: 585, y: 40 }, { x: 620, y: 38 }, { x: 635, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("લ"),
  }],
  // t30apps writes Gujarati ળ in one continuous run through the left bowl,
  // middle turn, high right arch, and descending spine.
    ["gujarati:ળ", {
    script: "gujarati", glyph: "ળ",
    strokes: [
      { segments: [{ label: "circle the left bowl, rise through the middle turn, and descend the right spine into its foot", path: [
        { x: 260, y: 565 }, { x: 205, y: 545 }, { x: 160, y: 515 }, { x: 125, y: 475 }, { x: 100, y: 430 },
        { x: 85, y: 380 }, { x: 85, y: 330 }, { x: 100, y: 280 }, { x: 130, y: 235 }, { x: 175, y: 195 },
        { x: 225, y: 175 }, { x: 275, y: 180 }, { x: 315, y: 205 }, { x: 345, y: 245 }, { x: 370, y: 300 },
        { x: 390, y: 355 }, { x: 390, y: 420 }, { x: 395, y: 475 }, { x: 410, y: 520 }, { x: 440, y: 550 },
        { x: 475, y: 560 }, { x: 510, y: 555 }, { x: 540, y: 535 }, { x: 565, y: 500 }, { x: 600, y: 520 },
        { x: 615, y: 470 }, { x: 615, y: 400 }, { x: 615, y: 320 }, { x: 615, y: 240 }, { x: 615, y: 160 },
        { x: 620, y: 105 }, { x: 640, y: 70 }, { x: 675, y: 48 }, { x: 710, y: 38 }, { x: 730, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("ળ"),
  }],
  // t30apps writes Gujarati વ as the rounded left body and shoulder followed
  // by the separately descended right spine.
    ["gujarati:વ", {
    script: "gujarati", glyph: "વ",
    strokes: [
      { segments: [{ label: "circle counterclockwise around the rounded left body and return into the right shoulder", path: [
        { x: 340, y: 500 }, { x: 300, y: 505 }, { x: 255, y: 505 }, { x: 210, y: 495 }, { x: 170, y: 475 },
        { x: 135, y: 445 }, { x: 110, y: 405 }, { x: 95, y: 360 }, { x: 95, y: 315 }, { x: 115, y: 275 },
        { x: 150, y: 235 }, { x: 195, y: 205 }, { x: 245, y: 190 }, { x: 295, y: 195 }, { x: 340, y: 215 },
        { x: 375, y: 245 }, { x: 400, y: 275 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 443, y: 585 }, { x: 443, y: 500 }, { x: 443, y: 400 }, { x: 443, y: 300 },
        { x: 443, y: 200 }, { x: 443, y: 125 }, { x: 451, y: 80 }, { x: 475, y: 55 },
        { x: 510, y: 40 }, { x: 545, y: 38 }, { x: 560, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("વ"),
  }],
  // t30apps writes Gujarati શ as a joined upper loop, lower body, and tail
  // followed by the separately descended right spine.
    ["gujarati:શ", {
    script: "gujarati", glyph: "શ",
    strokes: [
      { segments: [{ label: "circle the small upper loop and continue through the broad lower body into its tail", path: [
        { x: 230, y: 390 }, { x: 185, y: 405 }, { x: 145, y: 425 }, { x: 115, y: 450 },
        { x: 105, y: 485 }, { x: 120, y: 520 }, { x: 150, y: 550 }, { x: 195, y: 570 }, { x: 245, y: 575 },
        { x: 295, y: 560 }, { x: 335, y: 525 }, { x: 355, y: 480 }, { x: 350, y: 455 }, { x: 355, y: 415 },
        { x: 340, y: 360 }, { x: 310, y: 315 }, { x: 270, y: 280 }, { x: 220, y: 250 },
        { x: 125, y: 270 }, { x: 95, y: 245 }, { x: 85, y: 220 }, { x: 105, y: 205 }, { x: 140, y: 205 },
        { x: 160, y: 185 }, { x: 180, y: 145 }, { x: 215, y: 105 }, { x: 255, y: 70 }, { x: 305, y: 48 },
        { x: 355, y: 38 }, { x: 395, y: 38 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 542, y: 585 }, { x: 542, y: 500 }, { x: 542, y: 400 }, { x: 542, y: 300 },
        { x: 542, y: 200 }, { x: 542, y: 125 }, { x: 550, y: 80 }, { x: 575, y: 55 },
        { x: 610, y: 40 }, { x: 645, y: 38 }, { x: 660, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("શ"),
  }],
  // t30apps writes Gujarati સ as a joined upper loop, left body, and long
  // right shoulder followed by the separately descended right spine.
    ["gujarati:સ", {
    script: "gujarati", glyph: "સ",
    strokes: [
      { segments: [{ label: "circle the rounded upper loop and continue through the left body into the long right shoulder", path: [
        { x: 45, y: 500 }, { x: 45, y: 530 }, { x: 60, y: 555 }, { x: 90, y: 565 },
        { x: 120, y: 560 }, { x: 150, y: 550 }, { x: 195, y: 575 },
        { x: 245, y: 580 }, { x: 295, y: 565 }, { x: 335, y: 530 }, { x: 355, y: 485 },
        { x: 355, y: 430 }, { x: 365, y: 380 }, { x: 350, y: 340 }, { x: 320, y: 305 },
        { x: 280, y: 275 }, { x: 235, y: 245 }, { x: 185, y: 225 }, { x: 135, y: 215 },
        { x: 95, y: 225 }, { x: 70, y: 250 }, { x: 90, y: 275 }, { x: 130, y: 275 },
        { x: 170, y: 260 }, { x: 215, y: 275 }, { x: 260, y: 300 }, { x: 305, y: 325 },
        { x: 355, y: 330 }, { x: 405, y: 320 }, { x: 455, y: 320 }, { x: 500, y: 335 },
        { x: 535, y: 360 }, { x: 500, y: 335 }, { x: 455, y: 320 }, { x: 405, y: 320 },
        { x: 355, y: 330 }, { x: 305, y: 325 }, { x: 260, y: 300 }, { x: 215, y: 275 },
        { x: 200, y: 225 }, { x: 220, y: 180 }, { x: 250, y: 140 }, { x: 290, y: 100 },
        { x: 340, y: 65 }, { x: 390, y: 40 }, { x: 425, y: 38 },
      ] }] },
      { segments: [{ label: "lift, descend the tall right spine, and turn through its lower foot", path: [
        { x: 577, y: 585 }, { x: 577, y: 500 }, { x: 577, y: 400 }, { x: 577, y: 300 },
        { x: 577, y: 200 }, { x: 577, y: 125 }, { x: 585, y: 80 }, { x: 610, y: 55 },
        { x: 650, y: 40 }, { x: 685, y: 38 }, { x: 700, y: 38 },
      ] }] },
    ], source: gujaratiAlphabetSource("સ"),
  }],
  // t30apps writes Gujarati હ as one continuous upper-loop, middle-turn, and
  // broad-lower-bowl run.
    ["gujarati:હ", {
    script: "gujarati", glyph: "હ",
    strokes: [
      { segments: [{ label: "circle the compact upper loop and continue through the middle turn around the broad lower bowl", path: [
        { x: 500, y: 550 }, { x: 455, y: 565 }, { x: 410, y: 565 }, { x: 365, y: 550 },
        { x: 330, y: 525 }, { x: 315, y: 495 }, { x: 330, y: 465 }, { x: 370, y: 440 },
        { x: 420, y: 420 }, { x: 470, y: 400 }, { x: 510, y: 375 }, { x: 535, y: 340 },
        { x: 540, y: 295 }, { x: 525, y: 250 }, { x: 490, y: 220 }, { x: 445, y: 200 },
        { x: 395, y: 200 }, { x: 350, y: 215 }, { x: 310, y: 240 }, { x: 275, y: 275 },
        { x: 257, y: 332 }, { x: 161, y: 433 }, { x: 125, y: 440 },
        { x: 105, y: 400 }, { x: 90, y: 350 }, { x: 85, y: 295 }, { x: 95, y: 235 },
        { x: 120, y: 180 }, { x: 160, y: 125 }, { x: 215, y: 80 }, { x: 280, y: 50 },
        { x: 350, y: 35 }, { x: 420, y: 35 }, { x: 490, y: 45 }, { x: 550, y: 65 },
        { x: 600, y: 90 },
      ] }] },
    ], source: gujaratiAlphabetSource("હ"),
  }],
];
