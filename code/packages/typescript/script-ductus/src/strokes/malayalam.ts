// Authored malayalam ductus records. This is the stable source-ownership boundary.

import type { LetterDuctus, Point, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import malayalam from "../../../../../learning/human-languages/data/scripts/malayalam.json";

const malayalamIndependentVowelSource = (glyph: string): StrokeSource => {
  const letter = malayalam.independentVowels.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Malayalam independent vowel ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const malayalamAlphabetSource = (glyph: string): StrokeSource => {
  const letter = [...malayalam.letters, ...malayalam.finalConsonants].find(
    (candidate) => candidate.glyph === glyph,
  );
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Malayalam ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
    ["malayalam:എ", {
    script: "malayalam",
    glyph: "എ",
    strokes: [
      {
        segments: [
          {
            label: "turn around the compact left hook and carry the middle bar right",
            path: [
              { x: 75, y: 145 },
              { x: 75, y: 205 },
              { x: 115, y: 270 },
              { x: 175, y: 310 },
              { x: 230, y: 310 },
              { x: 300, y: 300 },
              { x: 360, y: 260 },
              { x: 390, y: 200 },
              { x: 390, y: 130 },
              { x: 370, y: 55 },
              { x: 500, y: 35 },
              { x: 690, y: 35 },
              { x: 890, y: 35 },
            ],
          },
          {
            label: "climb the upright, retrace it downward, and loop below the line",
            path: [
              { x: 890, y: 35 },
              { x: 890, y: 180 },
              { x: 890, y: 390 },
              { x: 890, y: 200 },
              { x: 890, y: 0 },
              { x: 885, y: -95 },
              { x: 835, y: -180 },
              { x: 775, y: -190 },
              { x: 700, y: -140 },
              { x: 650, y: -45 },
              { x: 625, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "sweep up and over through the broad outer arch, ending below the line",
            path: [
              { x: 615, y: 35 },
              { x: 625, y: 165 },
              { x: 675, y: 345 },
              { x: 760, y: 490 },
              { x: 900, y: 530 },
              { x: 1030, y: 485 },
              { x: 1125, y: 365 },
              { x: 1180, y: 215 },
              { x: 1185, y: 90 },
              { x: 1160, y: -45 },
              { x: 1095, y: -175 },
            ],
          },
        ],
      },
    ],
    source: malayalamIndependentVowelSource("എ"),
  }],
    ["malayalam:അ", {
    script: "malayalam",
    glyph: "അ",
    strokes: [
      {
        segments: [
          {
            label: "climb the left outer arch and curve through the upper turn",
            path: [
              { x: 235, y: 45 },
              { x: 175, y: 75 },
              { x: 90, y: 205 },
              { x: 85, y: 335 },
              { x: 160, y: 470 },
              { x: 285, y: 530 },
              { x: 400, y: 530 },
              { x: 410, y: 400 },
              { x: 520, y: 510 },
              { x: 600, y: 480 },
              { x: 660, y: 410 },
              { x: 665, y: 340 },
              { x: 625, y: 285 },
              { x: 550, y: 255 },
              { x: 500, y: 250 },
            ],
          },
          {
            label: "circle the broad lower loop and return to the junction",
            path: [
              { x: 500, y: 250 },
              { x: 610, y: 250 },
              { x: 680, y: 190 },
              { x: 680, y: 115 },
              { x: 615, y: 30 },
              { x: 535, y: 22 },
              { x: 430, y: 35 },
              { x: 350, y: 145 },
              { x: 350, y: 235 },
              { x: 390, y: 250 },
              { x: 500, y: 250 },
            ],
          },
          {
            label: "sweep up through the central crown and descend the upright",
            path: [
              { x: 500, y: 250 },
              { x: 610, y: 250 },
              { x: 665, y: 340 },
              { x: 640, y: 420 },
              { x: 640, y: 470 },
              { x: 650, y: 510 },
              { x: 700, y: 530 },
              { x: 760, y: 530 },
              { x: 855, y: 465 },
              { x: 890, y: 355 },
              { x: 890, y: 210 },
              { x: 890, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "sweep up and over through the right outer arch and descend its far side",
            path: [
              { x: 900, y: 205 },
              { x: 900, y: 350 },
              { x: 950, y: 450 },
              { x: 1025, y: 500 },
              { x: 1125, y: 530 },
              { x: 1240, y: 530 },
              { x: 1360, y: 455 },
              { x: 1415, y: 330 },
              { x: 1415, y: 220 },
              { x: 1395, y: 125 },
            ],
          },
          {
            label: "curl left around the lower inner loop",
            path: [
              { x: 1395, y: 125 },
              { x: 1345, y: 55 },
              { x: 1245, y: 22 },
              { x: 1155, y: 65 },
              { x: 1095, y: 155 },
              { x: 1095, y: 235 },
              { x: 1155, y: 330 },
              { x: 1250, y: 370 },
              { x: 1345, y: 325 },
            ],
          },
        ],
      },
    ],
    source: malayalamIndependentVowelSource("അ"),
  }],
  // Sriveenkat's 73-frame animation draws independent vowel ആ in two runs:
  // the left outer arch stands alone, then the inner curl flows through the
  // lower loop, central upright, rounded right loop, and below-line finish.
  // These five medians preserve that one-lift order on Noto Sans Malayalam.
    ["malayalam:ആ", {
    script: "malayalam",
    glyph: "ആ",
    strokes: [
      {
        segments: [
          {
            label: "climb the left outer arch and curve inward at the top",
            path: [
              { x: 235, y: 45 }, { x: 175, y: 75 }, { x: 90, y: 205 },
              { x: 85, y: 335 }, { x: 160, y: 470 }, { x: 285, y: 530 },
              { x: 400, y: 530 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "turn inward around the compact inner curl and circle the broad lower loop",
            path: [
              { x: 410, y: 400 }, { x: 520, y: 510 }, { x: 600, y: 480 }, { x: 660, y: 410 },
              { x: 665, y: 340 }, { x: 625, y: 285 }, { x: 550, y: 255 },
              { x: 500, y: 250 }, { x: 610, y: 250 }, { x: 680, y: 190 },
              { x: 680, y: 115 }, { x: 615, y: 30 }, { x: 535, y: 22 },
              { x: 430, y: 35 }, { x: 350, y: 145 }, { x: 350, y: 235 },
              { x: 390, y: 250 }, { x: 500, y: 250 },
            ],
          },
          {
            label: "sweep up through the central crown and descend the upright",
            path: [
              { x: 500, y: 250 }, { x: 610, y: 250 }, { x: 665, y: 340 },
              { x: 640, y: 420 }, { x: 640, y: 470 }, { x: 650, y: 510 },
              { x: 700, y: 530 }, { x: 760, y: 530 }, { x: 855, y: 465 },
              { x: 890, y: 355 }, { x: 890, y: 210 }, { x: 890, y: 20 },
            ],
          },
          {
            label: "retrace the upright and sweep around the rounded right loop",
            path: [
              { x: 890, y: 20 }, { x: 890, y: 210 }, { x: 900, y: 350 },
              { x: 950, y: 450 }, { x: 1050, y: 510 }, { x: 1150, y: 530 },
              { x: 1280, y: 520 }, { x: 1380, y: 450 }, { x: 1420, y: 340 },
              { x: 1400, y: 220 }, { x: 1340, y: 100 }, { x: 1250, y: 25 },
              { x: 1160, y: 40 }, { x: 1090, y: 110 }, { x: 1060, y: 220 },
              { x: 1080, y: 350 }, { x: 1140, y: 420 }, { x: 1240, y: 450 },
              { x: 1340, y: 420 }, { x: 1400, y: 350 },
            ],
          },
          {
            label: "descend the far side and curl left below the line",
            path: [
              { x: 1400, y: 350 }, { x: 1500, y: 300 }, { x: 1580, y: 200 },
              { x: 1600, y: 80 }, { x: 1535, y: -40 },
              { x: 1460, y: -135 }, { x: 1360, y: -190 }, { x: 1240, y: -195 },
              { x: 1180, y: -175 }, { x: 1130, y: -130 }, { x: 1120, y: -90 },
              { x: 1150, y: -60 }, { x: 1210, y: -55 },
            ],
          },
        ],
      },
    ],
    source: malayalamIndependentVowelSource("ആ"),
  }],
  // Davis's four-second initial-vowel clip writes ഇ in one uninterrupted run:
  // a compact left spiral expands into the central crown, descends and
  // retraces the stem, flows around the broad right lobe, then curls below the
  // line and finishes along the base.
    ["malayalam:ഇ", {
    script: "malayalam",
    glyph: "ഇ",
    strokes: [
      {
        segments: [
          {
            label: "turn outward around the compact left spiral and descend the central stem",
            path: [
              { x: 215, y: 380 },
              { x: 275, y: 390 },
              { x: 325, y: 355 },
              { x: 350, y: 305 },
              { x: 345, y: 250 },
              { x: 315, y: 200 },
              { x: 260, y: 160 },
              { x: 205, y: 145 },
              { x: 145, y: 180 },
              { x: 95, y: 240 },
              { x: 80, y: 310 },
              { x: 100, y: 395 },
              { x: 160, y: 470 },
              { x: 245, y: 520 },
              { x: 330, y: 535 },
              { x: 410, y: 515 },
              { x: 475, y: 465 },
              { x: 535, y: 390 },
              { x: 540, y: 305 },
              { x: 540, y: 160 },
            ],
          },
          {
            label: "retrace the central stem and sweep around the broad right lobe",
            path: [
              { x: 540, y: 160 },
              { x: 540, y: 305 },
              { x: 575, y: 485 },
              { x: 645, y: 530 },
              { x: 715, y: 535 },
              { x: 800, y: 500 },
              { x: 865, y: 430 },
              { x: 905, y: 340 },
              { x: 905, y: 255 },
              { x: 875, y: 165 },
              { x: 815, y: 95 },
              { x: 735, y: 55 },
              { x: 650, y: 35 },
              { x: 585, y: 35 },
            ],
          },
          {
            label: "curl left below the line",
            path: [
              { x: 585, y: 35 },
              { x: 500, y: 35 },
              { x: 400, y: 35 },
              { x: 300, y: 35 },
              { x: 215, y: 20 },
              { x: 145, y: -15 },
              { x: 105, y: -65 },
              { x: 115, y: -110 },
              { x: 160, y: -150 },
              { x: 240, y: -165 },
            ],
          },
          {
            label: "carry the finishing baseline to the right",
            path: [
              { x: 240, y: -165 },
              { x: 400, y: -165 },
              { x: 600, y: -165 },
              { x: 780, y: -165 },
              { x: 900, y: -165 },
            ],
          },
        ],
      },
    ],
    source: malayalamIndependentVowelSource("ഇ"),
  }],
  // Davis's five-second initial-vowel clip writes ഉ in one uninterrupted run:
  // the compact left spiral expands into the broad upper and right lobe, then
  // curls below the line and finishes along the baseline.
    ["malayalam:ഉ", {
    script: "malayalam",
    glyph: "ഉ",
    strokes: [
      {
        segments: [
          {
            label: "turn outward around the compact left spiral and carry the upper arch right",
            path: [
              { x: 215, y: 380 },
              { x: 275, y: 390 },
              { x: 325, y: 355 },
              { x: 350, y: 305 },
              { x: 345, y: 250 },
              { x: 315, y: 200 },
              { x: 260, y: 160 },
              { x: 205, y: 145 },
              { x: 145, y: 180 },
              { x: 95, y: 240 },
              { x: 80, y: 310 },
              { x: 100, y: 395 },
              { x: 160, y: 470 },
              { x: 245, y: 520 },
              { x: 335, y: 535 },
              { x: 420, y: 520 },
            ],
          },
          {
            label: "descend around the broad right lobe and curl left below the line",
            path: [
              { x: 420, y: 520 },
              { x: 500, y: 480 },
              { x: 570, y: 420 },
              { x: 610, y: 340 },
              { x: 610, y: 260 },
              { x: 580, y: 170 },
              { x: 520, y: 100 },
              { x: 430, y: 55 },
              { x: 340, y: 35 },
              { x: 260, y: 35 },
              { x: 200, y: 20 },
              { x: 145, y: -15 },
              { x: 105, y: -65 },
              { x: 115, y: -110 },
              { x: 160, y: -150 },
              { x: 240, y: -165 },
            ],
          },
          {
            label: "carry the finishing baseline to the right",
            path: [
              { x: 240, y: -165 },
              { x: 360, y: -165 },
              { x: 500, y: -165 },
              { x: 610, y: -165 },
            ],
          },
        ],
      },
    ],
    source: malayalamIndependentVowelSource("ഉ"),
  }],
  // Sriveenkat's 97-frame animation draws chillu ൽ as one uninterrupted run:
  // the left entry arch flows clockwise around the central loop, crosses the
  // upper shoulder into the right loop, then rises into the above-line hook.
  // These five movements preserve that zero-lift order on Noto Sans Malayalam.
    ["malayalam:ൽ", {
    script: "malayalam",
    glyph: "ൽ",
    strokes: [
      {
        segments: [
          {
            label: "climb the left entry arch and turn inward at the top",
            path: [
              { x: 220, y: 28 },
              { x: 155, y: 65 },
              { x: 100, y: 145 },
              { x: 80, y: 245 },
              { x: 92, y: 350 },
              { x: 140, y: 445 },
              { x: 225, y: 510 },
              { x: 330, y: 532 },
              { x: 430, y: 510 },
              { x: 488, y: 492 },
            ],
          },
          {
            label: "descend clockwise around the central loop and return to its upper junction",
            path: [
              { x: 488, y: 492 },
              { x: 555, y: 455 },
              { x: 615, y: 375 },
              { x: 660, y: 280 },
              { x: 665, y: 190 },
              { x: 630, y: 105 },
              { x: 570, y: 45 },
              { x: 505, y: 24 },
              { x: 435, y: 45 },
              { x: 375, y: 105 },
              { x: 345, y: 190 },
              { x: 350, y: 285 },
              { x: 385, y: 385 },
              { x: 435, y: 460 },
              { x: 488, y: 492 },
            ],
          },
          {
            label: "carry the upper shoulder right",
            path: [
              { x: 488, y: 492 },
              { x: 570, y: 525 },
              { x: 660, y: 540 },
              { x: 750, y: 535 },
              { x: 835, y: 520 },
              { x: 915, y: 492 },
            ],
          },
          {
            label: "sweep clockwise around the right loop and return to the upper crossing",
            path: [
              { x: 915, y: 492 },
              { x: 1000, y: 455 },
              { x: 1060, y: 385 },
              { x: 1100, y: 300 },
              { x: 1115, y: 215 },
              { x: 1095, y: 125 },
              { x: 1040, y: 60 },
              { x: 970, y: 25 },
              { x: 900, y: 45 },
              { x: 840, y: 105 },
              { x: 808, y: 190 },
              { x: 815, y: 280 },
              { x: 850, y: 375 },
              { x: 900, y: 455 },
              { x: 915, y: 492 },
            ],
          },
          {
            label: "rise into the chillu hook and curl left above the line",
            path: [
              { x: 915, y: 492 },
              { x: 970, y: 535 },
              { x: 1020, y: 595 },
              { x: 1045, y: 655 },
              { x: 1035, y: 705 },
              { x: 995, y: 742 },
              { x: 935, y: 755 },
              { x: 875, y: 748 },
              { x: 845, y: 738 },
            ],
          },
        ],
      },
    ],
    source: malayalamAlphabetSource("ൽ"),
  }],
  // Sriveenkat's 67-frame animation draws chillu ൻ in two pen-down runs:
  // the left arch descends into the central stem, then a lifted right-side run
  // completes the outer loop, inner return, and above-line chillu hook.
    ["malayalam:ൻ", {
    script: "malayalam",
    glyph: "ൻ",
    strokes: [
      {
        segments: [
          {
            label: "climb clockwise around the left arch and turn inward at the upper junction",
            path: [
              { x: 225, y: 35 },
              { x: 170, y: 75 },
              { x: 120, y: 145 },
              { x: 88, y: 225 },
              { x: 84, y: 305 },
              { x: 112, y: 395 },
              { x: 175, y: 475 },
              { x: 245, y: 520 },
              { x: 315, y: 530 },
              { x: 380, y: 505 },
              { x: 430, y: 455 },
            ],
          },
          {
            label: "descend the central stem to the line",
            path: [
              { x: 430, y: 455 },
              { x: 450, y: 370 },
              { x: 455, y: 280 },
              { x: 455, y: 190 },
              { x: 455, y: 100 },
              { x: 455, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "carry the upper shoulder right, sweep clockwise around the outer loop, and return through its inner curve",
            path: [
              { x: 540, y: 455 },
              { x: 600, y: 500 },
              { x: 675, y: 530 },
              { x: 740, y: 515 },
              { x: 790, y: 480 },
              { x: 860, y: 450 },
              { x: 925, y: 390 },
              { x: 965, y: 305 },
              { x: 978, y: 220 },
              { x: 955, y: 135 },
              { x: 900, y: 65 },
              { x: 825, y: 24 },
              { x: 755, y: 45 },
              { x: 695, y: 110 },
              { x: 665, y: 195 },
              { x: 670, y: 275 },
              { x: 705, y: 355 },
              { x: 750, y: 420 },
              { x: 790, y: 480 },
            ],
          },
          {
            label: "rise into the chillu hook and curl left above the line",
            path: [
              { x: 790, y: 480 },
              { x: 830, y: 515 },
              { x: 875, y: 565 },
              { x: 905, y: 620 },
              { x: 910, y: 675 },
              { x: 875, y: 725 },
              { x: 815, y: 752 },
              { x: 755, y: 750 },
              { x: 720, y: 740 },
            ],
          },
        ],
      },
    ],
    source: malayalamAlphabetSource("ൻ"),
  }],
  // Sriveenkat's 65-frame animation draws chillu ൾ in one uninterrupted run:
  // the left bowl climbs into the upper shoulder, flows clockwise around the
  // right loop, then rises through the crossing into the above-line hook.
    ["malayalam:ൾ", {
    script: "malayalam",
    glyph: "ൾ",
    strokes: [
      {
        segments: [
          {
            label: "descend clockwise around the left bowl and climb the central rise",
            path: [
              { x: 210, y: 520 },
              { x: 145, y: 485 },
              { x: 95, y: 405 },
              { x: 78, y: 300 },
              { x: 98, y: 185 },
              { x: 155, y: 85 },
              { x: 245, y: 25 },
              { x: 335, y: 28 },
              { x: 410, y: 92 },
              { x: 455, y: 190 },
              { x: 470, y: 295 },
              { x: 478, y: 455 },
            ],
          },
          {
            label: "carry the upper shoulder right",
            path: [
              { x: 478, y: 455 },
              { x: 550, y: 505 },
              { x: 635, y: 532 },
              { x: 720, y: 520 },
              { x: 805, y: 480 },
            ],
          },
          {
            label: "sweep clockwise around the right loop and return to the upper crossing",
            path: [
              { x: 805, y: 480 },
              { x: 890, y: 440 },
              { x: 950, y: 365 },
              { x: 978, y: 270 },
              { x: 968, y: 175 },
              { x: 925, y: 85 },
              { x: 850, y: 28 },
              { x: 775, y: 28 },
              { x: 705, y: 85 },
              { x: 665, y: 175 },
              { x: 662, y: 265 },
              { x: 695, y: 365 },
              { x: 755, y: 448 },
              { x: 805, y: 480 },
            ],
          },
          {
            label: "rise into the chillu hook and curl left above the line",
            path: [
              { x: 805, y: 480 },
              { x: 850, y: 530 },
              { x: 892, y: 595 },
              { x: 905, y: 655 },
              { x: 885, y: 705 },
              { x: 840, y: 745 },
              { x: 780, y: 765 },
              { x: 720, y: 758 },
              { x: 690, y: 748 },
            ],
          },
        ],
      },
    ],
    source: malayalamAlphabetSource("ൾ"),
  }],
  // Sriveenkat's 57-frame animation draws chillu ർ in one uninterrupted run:
  // the rising left arch flows through the right loop and inner return, then
  // climbs through the crossing into the above-line hook.
    ["malayalam:ർ", {
    script: "malayalam",
    glyph: "ർ",
    strokes: [
      {
        segments: [
          {
            label: "climb around the left arch and carry the upper shoulder right",
            path: [
              { x: 220, y: 25 }, { x: 160, y: 75 }, { x: 105, y: 155 },
              { x: 78, y: 255 }, { x: 92, y: 355 }, { x: 145, y: 450 },
              { x: 235, y: 515 }, { x: 345, y: 540 }, { x: 445, y: 510 },
            ],
          },
          {
            label: "sweep clockwise around the right loop and return to the upper crossing",
            path: [
              { x: 445, y: 510 }, { x: 545, y: 495 }, { x: 625, y: 430 },
              { x: 672, y: 335 }, { x: 680, y: 225 }, { x: 650, y: 125 },
              { x: 590, y: 48 }, { x: 510, y: 22 }, { x: 430, y: 52 },
              { x: 370, y: 130 }, { x: 340, y: 215 }, { x: 348, y: 300 },
              { x: 385, y: 390 }, { x: 440, y: 470 }, { x: 485, y: 505 },
            ],
          },
          {
            label: "rise into the chillu hook and curl left above the line",
            path: [
              { x: 485, y: 505 }, { x: 530, y: 555 }, { x: 570, y: 620 },
              { x: 582, y: 675 }, { x: 558, y: 720 }, { x: 510, y: 752 },
              { x: 455, y: 765 }, { x: 410, y: 758 }, { x: 380, y: 748 },
            ],
          },
        ],
      },
    ],
    source: malayalamAlphabetSource("ർ"),
  }],
  // Sriveenkat's 47-frame animation draws ഴ as one uninterrupted run: the
  // left entry arch reaches the lower junction, turns clockwise around the
  // right loop, and descends through its inner return into the lower hook.
    ["malayalam:ഴ", {
    script: "malayalam",
    glyph: "ഴ",
    strokes: [
      {
        segments: [
          {
            label: "descend around the left entry arch and sweep right into the lower junction",
            path: [
              { x: 92, y: 525 },
              { x: 65, y: 480 },
              { x: 58, y: 420 },
              { x: 68, y: 360 },
              { x: 105, y: 300 },
              { x: 165, y: 245 },
              { x: 235, y: 210 },
              { x: 300, y: 190 },
              { x: 350, y: 185 },
            ],
          },
          {
            label: "turn clockwise around the right loop and return through its inner side",
            path: [
              { x: 350, y: 185 },
              { x: 430, y: 190 },
              { x: 500, y: 245 },
              { x: 555, y: 330 },
              { x: 572, y: 410 },
              { x: 550, y: 475 },
              { x: 500, y: 525 },
              { x: 440, y: 535 },
              { x: 385, y: 515 },
              { x: 335, y: 470 },
              { x: 305, y: 410 },
              { x: 310, y: 350 },
              { x: 340, y: 290 },
              { x: 385, y: 225 },
            ],
          },
          {
            label: "descend through the inner return and curl left around the lower hook",
            path: [
              { x: 385, y: 225 },
              { x: 395, y: 165 },
              { x: 390, y: 110 },
              { x: 355, y: 60 },
              { x: 300, y: 30 },
              { x: 235, y: 25 },
              { x: 175, y: 38 },
              { x: 125, y: 60 },
            ],
          },
        ],
      },
    ],
    source: malayalamAlphabetSource("ഴ"),
  }],
];
