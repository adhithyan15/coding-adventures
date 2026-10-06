// Authored malayalam ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import malayalam from "../../../../../learning/human-languages/data/scripts/malayalam.json";

const malayalamIndependentVowelSource = (glyph: string): StrokeSource => {
  const letter = malayalam.independentVowels.find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(
      `Malayalam independent vowel ${glyph} has no verified source`,
    );
  }
  return letter.strokeOrderSource;
};

const malayalamAlphabetSource = (glyph: string): StrokeSource => {
  const letter = [...malayalam.letters, ...malayalam.finalConsonants].find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Malayalam ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  [
    "malayalam:എ",
    {
      script: "malayalam",
      glyph: "എ",
      strokes: [
        {
          segments: [
            {
              label:
                "turn around the compact left hook and carry the middle bar right",
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
              label:
                "climb the upright, retrace it downward, and loop below the line",
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
              label:
                "sweep up and over through the broad outer arch, ending below the line",
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
    },
  ],
  // Davis's four-second initial-vowel clip writes ഒ in two runs: the compact
  // inner curl expands continuously into the broad left arch, then one lifted
  // right-side run carries the upper shoulder around the rounded lower lobe.
  [
    "malayalam:ഒ",
    {
      script: "malayalam",
      glyph: "ഒ",
      strokes: [
        {
          segments: [
            {
              label:
                "curl clockwise from the compact inner tip and sweep through the broad left arch",
              path: [
                { x: 190, y: 290 },
                { x: 245, y: 293 },
                { x: 300, y: 280 },
                { x: 350, y: 230 },
                { x: 365, y: 165 },
                { x: 355, y: 95 },
                { x: 300, y: 45 },
                { x: 240, y: 20 },
                { x: 180, y: 35 },
                { x: 120, y: 95 },
                { x: 95, y: 165 },
                { x: 105, y: 220 },
                { x: 145, y: 260 },
                { x: 190, y: 290 },
                { x: 190, y: 310 },
                { x: 125, y: 315 },
                { x: 95, y: 350 },
                { x: 110, y: 390 },
                { x: 140, y: 430 },
                { x: 180, y: 465 },
                { x: 240, y: 495 },
                { x: 310, y: 520 },
                { x: 400, y: 530 },
                { x: 520, y: 510 },
                { x: 620, y: 465 },
                { x: 665, y: 410 },
                { x: 655, y: 350 },
                { x: 600, y: 300 },
                { x: 490, y: 275 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "sweep right from the upper junction and descend around the rounded lower lobe",
              path: [
                { x: 490, y: 275 },
                { x: 560, y: 275 },
                { x: 625, y: 245 },
                { x: 665, y: 195 },
                { x: 675, y: 140 },
                { x: 650, y: 85 },
                { x: 600, y: 45 },
                { x: 540, y: 25 },
                { x: 475, y: 35 },
              ],
            },
          ],
        },
      ],
      source: malayalamIndependentVowelSource("ഒ"),
    },
  ],
  // Davis's four-second initial-vowel clip writes ഓ in three runs: the first
  // two reproduce short ഒ, then a second lift starts the separate far-right
  // outer arc and carries it continuously from top to bottom.
  [
    "malayalam:ഓ",
    {
      script: "malayalam",
      glyph: "ഓ",
      strokes: [
        {
          segments: [
            {
              label:
                "curl clockwise from the compact inner tip and sweep through the broad left arch",
              path: [
                { x: 190, y: 290 },
                { x: 245, y: 293 },
                { x: 300, y: 280 },
                { x: 350, y: 230 },
                { x: 365, y: 165 },
                { x: 355, y: 95 },
                { x: 300, y: 45 },
                { x: 240, y: 20 },
                { x: 180, y: 35 },
                { x: 120, y: 95 },
                { x: 95, y: 165 },
                { x: 105, y: 220 },
                { x: 145, y: 260 },
                { x: 190, y: 290 },
                { x: 190, y: 310 },
                { x: 125, y: 315 },
                { x: 95, y: 350 },
                { x: 110, y: 390 },
                { x: 140, y: 430 },
                { x: 180, y: 465 },
                { x: 240, y: 495 },
                { x: 310, y: 520 },
                { x: 400, y: 530 },
                { x: 520, y: 510 },
                { x: 620, y: 465 },
                { x: 665, y: 410 },
                { x: 655, y: 350 },
                { x: 600, y: 300 },
                { x: 490, y: 275 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "sweep right from the upper junction and descend around the rounded lower lobe",
              path: [
                { x: 490, y: 275 },
                { x: 560, y: 275 },
                { x: 625, y: 245 },
                { x: 665, y: 195 },
                { x: 675, y: 140 },
                { x: 650, y: 85 },
                { x: 600, y: 45 },
                { x: 540, y: 25 },
                { x: 475, y: 35 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "descend around the separate outer arc at the far right",
              path: [
                { x: 815, y: 385 },
                { x: 850, y: 465 },
                { x: 915, y: 520 },
                { x: 995, y: 530 },
                { x: 1070, y: 500 },
                { x: 1130, y: 440 },
                { x: 1170, y: 360 },
                { x: 1175, y: 275 },
                { x: 1160, y: 190 },
                { x: 1120, y: 110 },
                { x: 1050, y: 50 },
                { x: 980, y: 20 },
                { x: 910, y: 45 },
                { x: 850, y: 100 },
                { x: 815, y: 170 },
              ],
            },
          ],
        },
      ],
      source: malayalamIndependentVowelSource("ഓ"),
    },
  ],
  [
    "malayalam:അ",
    {
      script: "malayalam",
      glyph: "അ",
      strokes: [
        {
          segments: [
            {
              label:
                "climb the left outer arch and curve through the upper turn",
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
              label:
                "sweep up through the central crown and descend the upright",
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
              label:
                "sweep up and over through the right outer arch and descend its far side",
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
    },
  ],
  // Sriveenkat's 73-frame animation draws independent vowel ആ in two runs:
  // the left outer arch stands alone, then the inner curl flows through the
  // lower loop, central upright, rounded right loop, and below-line finish.
  // These five medians preserve that one-lift order on Noto Sans Malayalam.
  [
    "malayalam:ആ",
    {
      script: "malayalam",
      glyph: "ആ",
      strokes: [
        {
          segments: [
            {
              label: "climb the left outer arch and curve inward at the top",
              path: [
                { x: 235, y: 45 },
                { x: 175, y: 75 },
                { x: 90, y: 205 },
                { x: 85, y: 335 },
                { x: 160, y: 470 },
                { x: 285, y: 530 },
                { x: 400, y: 530 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "turn inward around the compact inner curl and circle the broad lower loop",
              path: [
                { x: 410, y: 400 },
                { x: 520, y: 510 },
                { x: 600, y: 480 },
                { x: 660, y: 410 },
                { x: 665, y: 340 },
                { x: 625, y: 285 },
                { x: 550, y: 255 },
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
              label:
                "sweep up through the central crown and descend the upright",
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
            {
              label:
                "retrace the upright and sweep around the rounded right loop",
              path: [
                { x: 890, y: 20 },
                { x: 890, y: 210 },
                { x: 900, y: 350 },
                { x: 950, y: 450 },
                { x: 1050, y: 510 },
                { x: 1150, y: 530 },
                { x: 1280, y: 520 },
                { x: 1380, y: 450 },
                { x: 1420, y: 340 },
                { x: 1400, y: 220 },
                { x: 1340, y: 100 },
                { x: 1250, y: 25 },
                { x: 1160, y: 40 },
                { x: 1090, y: 110 },
                { x: 1060, y: 220 },
                { x: 1080, y: 350 },
                { x: 1140, y: 420 },
                { x: 1240, y: 450 },
                { x: 1340, y: 420 },
                { x: 1400, y: 350 },
              ],
            },
            {
              label: "descend the far side and curl left below the line",
              path: [
                { x: 1400, y: 350 },
                { x: 1500, y: 300 },
                { x: 1580, y: 200 },
                { x: 1600, y: 80 },
                { x: 1535, y: -40 },
                { x: 1460, y: -135 },
                { x: 1360, y: -190 },
                { x: 1240, y: -195 },
                { x: 1180, y: -175 },
                { x: 1130, y: -130 },
                { x: 1120, y: -90 },
                { x: 1150, y: -60 },
                { x: 1210, y: -55 },
              ],
            },
          ],
        },
      ],
      source: malayalamIndependentVowelSource("ആ"),
    },
  ],
  // Davis's four-second initial-vowel clip writes ഇ in one uninterrupted run:
  // a compact left spiral expands into the central crown, descends and
  // retraces the stem, flows around the broad right lobe, then curls below the
  // line and finishes along the base.
  [
    "malayalam:ഇ",
    {
      script: "malayalam",
      glyph: "ഇ",
      strokes: [
        {
          segments: [
            {
              label:
                "turn outward around the compact left spiral and descend the central stem",
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
              label:
                "retrace the central stem and sweep around the broad right lobe",
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
    },
  ],
  // Davis's five-second initial-vowel clip writes ഉ in one uninterrupted run:
  // the compact left spiral expands into the broad upper and right lobe, then
  // curls below the line and finishes along the baseline.
  [
    "malayalam:ഉ",
    {
      script: "malayalam",
      glyph: "ഉ",
      strokes: [
        {
          segments: [
            {
              label:
                "turn outward around the compact left spiral and carry the upper arch right",
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
              label:
                "descend around the broad right lobe and curl left below the line",
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
    },
  ],
  // Davis's six-second initial-vowel clip writes ഊ in two runs: the main
  // spiral-to-baseline body stays joined, then one lifted flourish carries a
  // compact upper arch into the broad right loop and descending tail.
  [
    "malayalam:ഊ",
    {
      script: "malayalam",
      glyph: "ഊ",
      strokes: [
        {
          segments: [
            {
              label:
                "turn outward around the compact left spiral and carry the upper arch right",
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
              label:
                "descend around the broad right lobe and curl left below the line",
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
        {
          segments: [
            {
              label:
                "sweep over the compact upper arch and cross into the right lobe",
              path: [
                { x: 805, y: 275 },
                { x: 775, y: 320 },
                { x: 765, y: 375 },
                { x: 780, y: 425 },
                { x: 820, y: 470 },
                { x: 865, y: 495 },
                { x: 920, y: 515 },
                { x: 975, y: 490 },
                { x: 1010, y: 430 },
                { x: 1035, y: 333 },
              ],
            },
            {
              label:
                "circle the broad right lobe and descend its finishing tail",
              path: [
                { x: 1035, y: 333 },
                { x: 1025, y: 430 },
                { x: 1080, y: 515 },
                { x: 1205, y: 530 },
                { x: 1290, y: 485 },
                { x: 1340, y: 390 },
                { x: 1360, y: 285 },
                { x: 1340, y: 175 },
                { x: 1285, y: 85 },
                { x: 1205, y: 45 },
                { x: 1135, y: 35 },
                { x: 1060, y: 35 },
                { x: 1000, y: 55 },
              ],
            },
          ],
        },
      ],
      source: malayalamIndependentVowelSource("ഊ"),
    },
  ],
  // Sriveenkat's 97-frame animation draws chillu ൽ as one uninterrupted run:
  // the left entry arch flows clockwise around the central loop, crosses the
  // upper shoulder into the right loop, then rises into the above-line hook.
  // These five movements preserve that zero-lift order on Noto Sans Malayalam.
  [
    "malayalam:ൽ",
    {
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
              label:
                "descend clockwise around the central loop and return to its upper junction",
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
              label:
                "sweep clockwise around the right loop and return to the upper crossing",
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
    },
  ],
  // Sriveenkat's 67-frame animation draws chillu ൻ in two pen-down runs:
  // the left arch descends into the central stem, then a lifted right-side run
  // completes the outer loop, inner return, and above-line chillu hook.
  [
    "malayalam:ൻ",
    {
      script: "malayalam",
      glyph: "ൻ",
      strokes: [
        {
          segments: [
            {
              label:
                "climb clockwise around the left arch and turn inward at the upper junction",
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
              label:
                "carry the upper shoulder right, sweep clockwise around the outer loop, and return through its inner curve",
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
    },
  ],
  // Sriveenkat's 97-frame animation draws chillu ൺ as one uninterrupted run:
  // the compact inner loop flows around the broad outer-left bowl, retraces
  // through the paired central stems, circles the right loop, and finishes in
  // the above-line hook. These five movements preserve that zero-lift order on
  // Noto Sans Malayalam.
  [
    "malayalam:ൺ",
    {
      script: "malayalam",
      glyph: "ൺ",
      strokes: [
        {
          segments: [
            {
              label:
                "start at the inner-left tip and sweep clockwise around the compact inner loop",
              path: [
                { x: 95, y: 220 },
                { x: 155, y: 285 },
                { x: 245, y: 295 },
                { x: 330, y: 245 },
                { x: 368, y: 165 },
                { x: 345, y: 80 },
                { x: 292, y: 28 },
                { x: 240, y: 22 },
                { x: 175, y: 50 },
                { x: 115, y: 130 },
                { x: 95, y: 220 },
              ],
            },
            {
              label:
                "without lifting, continue clockwise around the broad outer-left bowl and climb to the upper junction",
              path: [
                { x: 95, y: 220 },
                { x: 82, y: 300 },
                { x: 110, y: 405 },
                { x: 180, y: 495 },
                { x: 270, y: 535 },
                { x: 360, y: 530 },
                { x: 445, y: 485 },
                { x: 520, y: 430 },
              ],
            },
            {
              label:
                "without lifting, descend the first central stem, retrace upward, carry the middle arch, and descend the second stem",
              path: [
                { x: 520, y: 430 },
                { x: 566, y: 350 },
                { x: 566, y: 240 },
                { x: 566, y: 125 },
                { x: 566, y: 28 },
                { x: 566, y: 240 },
                { x: 585, y: 420 },
                { x: 650, y: 500 },
                { x: 740, y: 530 },
                { x: 825, y: 500 },
                { x: 905, y: 430 },
                { x: 912, y: 300 },
                { x: 912, y: 150 },
                { x: 912, y: 28 },
              ],
            },
            {
              label:
                "without lifting, carry the upper shoulder right and sweep clockwise around the right loop",
              path: [
                { x: 912, y: 28 },
                { x: 912, y: 230 },
                { x: 935, y: 420 },
                { x: 1010, y: 500 },
                { x: 1100, y: 530 },
                { x: 1200, y: 530 },
                { x: 1290, y: 490 },
                { x: 1385, y: 390 },
                { x: 1435, y: 265 },
                { x: 1435, y: 155 },
                { x: 1375, y: 55 },
                { x: 1280, y: 25 },
                { x: 1200, y: 65 },
                { x: 1135, y: 150 },
                { x: 1125, y: 245 },
                { x: 1160, y: 355 },
                { x: 1245, y: 473 },
              ],
            },
            {
              label:
                "without lifting, rise into the chillu hook and curl left above the line",
              path: [
                { x: 1245, y: 473 },
                { x: 1305, y: 535 },
                { x: 1365, y: 620 },
                { x: 1365, y: 675 },
                { x: 1330, y: 730 },
                { x: 1260, y: 755 },
                { x: 1195, y: 748 },
                { x: 1165, y: 738 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ൺ"),
    },
  ],
  // Sriveenkat's 65-frame animation draws chillu ൾ in one uninterrupted run:
  // the left bowl climbs into the upper shoulder, flows clockwise around the
  // right loop, then rises through the crossing into the above-line hook.
  [
    "malayalam:ൾ",
    {
      script: "malayalam",
      glyph: "ൾ",
      strokes: [
        {
          segments: [
            {
              label:
                "descend clockwise around the left bowl and climb the central rise",
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
              label:
                "sweep clockwise around the right loop and return to the upper crossing",
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
    },
  ],
  // Sriveenkat's 57-frame animation draws chillu ർ in one uninterrupted run:
  // the rising left arch flows through the right loop and inner return, then
  // climbs through the crossing into the above-line hook.
  [
    "malayalam:ർ",
    {
      script: "malayalam",
      glyph: "ർ",
      strokes: [
        {
          segments: [
            {
              label:
                "climb around the left arch and carry the upper shoulder right",
              path: [
                { x: 220, y: 25 },
                { x: 160, y: 75 },
                { x: 105, y: 155 },
                { x: 78, y: 255 },
                { x: 92, y: 355 },
                { x: 145, y: 450 },
                { x: 235, y: 515 },
                { x: 345, y: 540 },
                { x: 445, y: 510 },
              ],
            },
            {
              label:
                "sweep clockwise around the right loop and return to the upper crossing",
              path: [
                { x: 445, y: 510 },
                { x: 545, y: 495 },
                { x: 625, y: 430 },
                { x: 672, y: 335 },
                { x: 680, y: 225 },
                { x: 650, y: 125 },
                { x: 590, y: 48 },
                { x: 510, y: 22 },
                { x: 430, y: 52 },
                { x: 370, y: 130 },
                { x: 340, y: 215 },
                { x: 348, y: 300 },
                { x: 385, y: 390 },
                { x: 440, y: 470 },
                { x: 485, y: 505 },
              ],
            },
            {
              label: "rise into the chillu hook and curl left above the line",
              path: [
                { x: 485, y: 505 },
                { x: 530, y: 555 },
                { x: 570, y: 620 },
                { x: 582, y: 675 },
                { x: 558, y: 720 },
                { x: 510, y: 752 },
                { x: 455, y: 765 },
                { x: 410, y: 758 },
                { x: 380, y: 748 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ർ"),
    },
  ],
  // Sriveenkat's 47-frame animation draws ഴ as one uninterrupted run: the
  // left entry arch reaches the lower junction, turns clockwise around the
  // right loop, and descends through its inner return into the lower hook.
  [
    "malayalam:ഴ",
    {
      script: "malayalam",
      glyph: "ഴ",
      strokes: [
        {
          segments: [
            {
              label:
                "descend around the left entry arch and sweep right into the lower junction",
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
              label:
                "turn clockwise around the right loop and return through its inner side",
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
              label:
                "descend through the inner return and curl left around the lower hook",
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
    },
  ],
  // Base consonants. Each bare consonant row (role "syllable", inherent a)
  // cites SPACE's Thooval, a Malayalam alphabet teaching tool whose formation
  // image for each letter marks the start (green), every turn where the pen
  // runs back along its own ink (blue) and the end (red), with arrows between.
  // Thooval is GPL-3.0, so it is cited for those FACTS only: no image, path or
  // template point is copied. Thooval asks the learner to keep the pen down to
  // the end of the letter, so its "no lift" is not evidence by itself; the lift
  // count rests on two recorded sources that could have shown a lift and did
  // not (Santhosh Thottingal's hand, MIT, and grahyam's samples, counts only).
  // Every path follows the skeleton of the bundled Noto Sans Malayalam outline
  // between the turning points Thooval marks; where Thooval turns back at a
  // stem's foot the path retraces the stem.
  // Thooval's NA.png writes ന in one movement: from the foot of the left
  // arch the pen climbs clockwise over it and runs down the stem to the
  // line (blue), climbs the stem again, and sweeps over the right arch to
  // finish at its foot (red).
  [
    "malayalam:ന",
    {
      script: "malayalam",
      glyph: "ന",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the left arch",
              path: [
                { x: 203, y: 52 },
                { x: 150, y: 97 },
                { x: 111, y: 154 },
                { x: 90, y: 222 },
                { x: 87, y: 292 },
                { x: 90, y: 362 },
                { x: 115, y: 427 },
                { x: 158, y: 482 },
                { x: 217, y: 518 },
                { x: 287, y: 528 },
                { x: 355, y: 516 },
                { x: 410, y: 473 },
                { x: 455, y: 420 },
              ],
            },
            {
              label: "descend the stem to the line",
              path: [
                { x: 455, y: 420 },
                { x: 455, y: 344 },
                { x: 455, y: 268 },
                { x: 455, y: 192 },
                { x: 455, y: 116 },
                { x: 455, y: 40 },
              ],
            },
            {
              label: "retrace the stem upward",
              path: [
                { x: 455, y: 40 },
                { x: 455, y: 114 },
                { x: 455, y: 189 },
                { x: 455, y: 263 },
                { x: 455, y: 338 },
                { x: 455, y: 412 },
              ],
            },
            {
              label: "sweep over the right arch to its foot",
              path: [
                { x: 455, y: 412 },
                { x: 498, y: 467 },
                { x: 550, y: 513 },
                { x: 618, y: 528 },
                { x: 688, y: 521 },
                { x: 749, y: 488 },
                { x: 793, y: 434 },
                { x: 818, y: 369 },
                { x: 827, y: 299 },
                { x: 824, y: 229 },
                { x: 803, y: 162 },
                { x: 769, y: 101 },
                { x: 723, y: 48 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ന"),
    },
  ],
  // Thooval's MA.png starts at the bottom left, climbs the left side and
  // arches clockwise over the top, curves down the inner stroke to the
  // bottom-left corner, runs right along the base and climbs the right side
  // to finish at the top, where it meets the arch.
  [
    "malayalam:മ",
    {
      script: "malayalam",
      glyph: "മ",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side and arch over",
              path: [
                { x: 108, y: 48 },
                { x: 104, y: 116 },
                { x: 104, y: 185 },
                { x: 104, y: 253 },
                { x: 105, y: 322 },
                { x: 121, y: 388 },
                { x: 153, y: 448 },
                { x: 203, y: 496 },
                { x: 265, y: 523 },
                { x: 334, y: 528 },
                { x: 400, y: 512 },
              ],
            },
            {
              label: "curve down the inner stroke",
              path: [
                { x: 400, y: 512 },
                { x: 428, y: 447 },
                { x: 435, y: 378 },
                { x: 412, y: 309 },
                { x: 379, y: 244 },
                { x: 336, y: 186 },
                { x: 285, y: 133 },
                { x: 236, y: 80 },
                { x: 179, y: 41 },
                { x: 108, y: 48 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 108, y: 48 },
                { x: 171, y: 36 },
                { x: 237, y: 36 },
                { x: 303, y: 36 },
                { x: 369, y: 36 },
                { x: 434, y: 36 },
                { x: 500, y: 36 },
                { x: 566, y: 36 },
                { x: 632, y: 40 },
              ],
            },
            {
              label: "climb the right side to the top",
              path: [
                { x: 632, y: 40 },
                { x: 620, y: 104 },
                { x: 620, y: 171 },
                { x: 620, y: 237 },
                { x: 620, y: 304 },
                { x: 612, y: 369 },
                { x: 601, y: 434 },
                { x: 551, y: 478 },
                { x: 492, y: 508 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("മ"),
    },
  ],
  // Thooval's SA.png begins like ന: up and over the left arch, down the
  // stem to the line (blue) and back up it. Then the pen sweeps over the
  // middle arch, down and round the bowl, and up to the top-right tip (red).
  [
    "malayalam:സ",
    {
      script: "malayalam",
      glyph: "സ",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the left arch",
              path: [
                { x: 203, y: 52 },
                { x: 150, y: 97 },
                { x: 111, y: 154 },
                { x: 91, y: 221 },
                { x: 87, y: 290 },
                { x: 90, y: 360 },
                { x: 114, y: 425 },
                { x: 156, y: 480 },
                { x: 215, y: 517 },
                { x: 284, y: 528 },
                { x: 353, y: 517 },
                { x: 407, y: 475 },
                { x: 455, y: 424 },
              ],
            },
            {
              label: "descend the stem to the line",
              path: [
                { x: 455, y: 424 },
                { x: 455, y: 347 },
                { x: 455, y: 270 },
                { x: 455, y: 194 },
                { x: 455, y: 117 },
                { x: 455, y: 40 },
              ],
            },
            {
              label: "retrace the stem upward",
              path: [
                { x: 455, y: 40 },
                { x: 455, y: 114 },
                { x: 455, y: 189 },
                { x: 455, y: 263 },
                { x: 455, y: 338 },
                { x: 455, y: 412 },
              ],
            },
            {
              label: "sweep over the middle arch and down",
              path: [
                { x: 455, y: 412 },
                { x: 498, y: 467 },
                { x: 550, y: 513 },
                { x: 618, y: 528 },
                { x: 687, y: 517 },
                { x: 739, y: 471 },
                { x: 769, y: 408 },
                { x: 775, y: 338 },
                { x: 783, y: 269 },
                { x: 791, y: 200 },
              ],
            },
            {
              label: "round the bowl and up to the top",
              path: [
                { x: 791, y: 200 },
                { x: 793, y: 131 },
                { x: 833, y: 74 },
                { x: 887, y: 32 },
                { x: 957, y: 28 },
                { x: 1025, y: 40 },
                { x: 1078, y: 83 },
                { x: 1117, y: 140 },
                { x: 1139, y: 206 },
                { x: 1148, y: 274 },
                { x: 1135, y: 341 },
                { x: 1129, y: 409 },
                { x: 1085, y: 462 },
                { x: 1035, y: 508 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("സ"),
    },
  ],
  // Thooval's RA.png climbs from the left foot clockwise over the top,
  // descends the right side, runs left along the bottom and curls up the
  // inner loop, finishing where it meets the right side.
  [
    "malayalam:ര",
    {
      script: "malayalam",
      glyph: "ര",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the top",
              path: [
                { x: 219, y: 48 },
                { x: 161, y: 82 },
                { x: 116, y: 136 },
                { x: 92, y: 202 },
                { x: 87, y: 272 },
                { x: 91, y: 342 },
                { x: 116, y: 408 },
                { x: 161, y: 461 },
                { x: 219, y: 500 },
                { x: 286, y: 522 },
                { x: 356, y: 528 },
                { x: 426, y: 527 },
                { x: 493, y: 508 },
                { x: 555, y: 475 },
                { x: 604, y: 425 },
                { x: 639, y: 364 },
              ],
            },
            {
              label: "descend the right side",
              path: [
                { x: 639, y: 364 },
                { x: 653, y: 290 },
                { x: 655, y: 214 },
                { x: 643, y: 140 },
                { x: 607, y: 73 },
                { x: 543, y: 32 },
              ],
            },
            {
              label: "curl up around the inner loop",
              path: [
                { x: 543, y: 32 },
                { x: 471, y: 28 },
                { x: 404, y: 51 },
                { x: 357, y: 106 },
                { x: 339, y: 174 },
                { x: 351, y: 245 },
                { x: 368, y: 314 },
                { x: 425, y: 357 },
                { x: 497, y: 368 },
                { x: 566, y: 347 },
                { x: 635, y: 328 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ര"),
    },
  ],
  // Thooval's TA.png climbs from the left foot over the left arch, crosses
  // down the right side of the middle loop, rounds its base and climbs its
  // left side back through the crossing, then sweeps over the right arch to
  // finish at the right foot.
  [
    "malayalam:ത",
    {
      script: "malayalam",
      glyph: "ത",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the left arch",
              path: [
                { x: 215, y: 48 },
                { x: 158, y: 85 },
                { x: 114, y: 140 },
                { x: 92, y: 206 },
                { x: 87, y: 276 },
                { x: 92, y: 345 },
                { x: 117, y: 410 },
                { x: 161, y: 465 },
                { x: 220, y: 503 },
                { x: 286, y: 524 },
                { x: 356, y: 528 },
                { x: 426, y: 525 },
                { x: 487, y: 492 },
              ],
            },
            {
              label: "cross down the middle loop",
              path: [
                { x: 487, y: 492 },
                { x: 550, y: 465 },
                { x: 600, y: 414 },
                { x: 637, y: 353 },
                { x: 660, y: 285 },
                { x: 671, y: 214 },
                { x: 665, y: 143 },
                { x: 630, y: 81 },
                { x: 575, y: 36 },
              ],
            },
            {
              label: "round its base and climb back up",
              path: [
                { x: 575, y: 36 },
                { x: 505, y: 28 },
                { x: 436, y: 38 },
                { x: 385, y: 85 },
                { x: 359, y: 150 },
                { x: 360, y: 220 },
                { x: 359, y: 291 },
                { x: 386, y: 354 },
                { x: 418, y: 415 },
                { x: 470, y: 463 },
                { x: 531, y: 492 },
              ],
            },
            {
              label: "sweep over the right arch to its foot",
              path: [
                { x: 531, y: 492 },
                { x: 589, y: 524 },
                { x: 657, y: 532 },
                { x: 724, y: 528 },
                { x: 789, y: 511 },
                { x: 847, y: 476 },
                { x: 890, y: 424 },
                { x: 918, y: 363 },
                { x: 932, y: 299 },
                { x: 927, y: 232 },
                { x: 912, y: 166 },
                { x: 880, y: 106 },
                { x: 835, y: 56 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ത"),
    },
  ],
  // Thooval's SSA.png climbs over the small arch to the line, runs right
  // along the base, climbs the right upright, slants down to the left into
  // the loop, circles it and runs straight down the stem to the line (red).
  [
    "malayalam:ഷ",
    {
      script: "malayalam",
      glyph: "ഷ",
      strokes: [
        {
          segments: [
            {
              label: "climb over the small arch",
              path: [
                { x: 144, y: 41 },
                { x: 96, y: 88 },
                { x: 80, y: 152 },
                { x: 84, y: 219 },
                { x: 123, y: 273 },
                { x: 183, y: 304 },
                { x: 250, y: 310 },
                { x: 313, y: 290 },
                { x: 361, y: 242 },
                { x: 385, y: 180 },
                { x: 377, y: 114 },
                { x: 360, y: 49 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 360, y: 49 },
                { x: 427, y: 37 },
                { x: 497, y: 37 },
                { x: 566, y: 37 },
                { x: 635, y: 37 },
                { x: 705, y: 37 },
                { x: 774, y: 37 },
                { x: 844, y: 37 },
                { x: 913, y: 37 },
                { x: 983, y: 37 },
                { x: 1052, y: 37 },
              ],
            },
            {
              label: "climb the right upright",
              path: [
                { x: 1052, y: 37 },
                { x: 1036, y: 104 },
                { x: 1036, y: 176 },
                { x: 1036, y: 248 },
                { x: 1039, y: 318 },
                { x: 1036, y: 390 },
                { x: 1036, y: 462 },
                { x: 1052, y: 529 },
              ],
            },
            {
              label: "slant down to the left",
              path: [
                { x: 1052, y: 529 },
                { x: 1005, y: 471 },
                { x: 952, y: 418 },
                { x: 904, y: 361 },
                { x: 845, y: 314 },
                { x: 775, y: 289 },
                { x: 712, y: 321 },
              ],
            },
            {
              label: "circle the loop",
              path: [
                { x: 712, y: 321 },
                { x: 660, y: 275 },
                { x: 589, y: 277 },
                { x: 527, y: 310 },
                { x: 485, y: 367 },
                { x: 480, y: 438 },
                { x: 511, y: 499 },
                { x: 574, y: 525 },
                { x: 645, y: 518 },
                { x: 694, y: 468 },
                { x: 712, y: 401 },
              ],
            },
            {
              label: "descend the stem to the line",
              path: [
                { x: 712, y: 401 },
                { x: 700, y: 330 },
                { x: 700, y: 257 },
                { x: 702, y: 184 },
                { x: 700, y: 110 },
                { x: 716, y: 41 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഷ"),
    },
  ],
  // Thooval's PA.png climbs from the left foot over the small arch, comes
  // down to the line, runs right along the base and climbs the upright to
  // finish at its top.
  [
    "malayalam:പ",
    {
      script: "malayalam",
      glyph: "പ",
      strokes: [
        {
          segments: [
            {
              label: "climb over the small arch",
              path: [
                { x: 124, y: 45 },
                { x: 85, y: 106 },
                { x: 76, y: 178 },
                { x: 100, y: 244 },
                { x: 154, y: 293 },
                { x: 224, y: 311 },
                { x: 295, y: 300 },
                { x: 351, y: 254 },
                { x: 383, y: 190 },
                { x: 378, y: 118 },
                { x: 356, y: 49 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 356, y: 49 },
                { x: 425, y: 37 },
                { x: 496, y: 37 },
                { x: 566, y: 37 },
                { x: 637, y: 37 },
                { x: 708, y: 37 },
                { x: 776, y: 49 },
              ],
            },
            {
              label: "climb the upright to the top",
              path: [
                { x: 776, y: 49 },
                { x: 780, y: 115 },
                { x: 780, y: 181 },
                { x: 780, y: 248 },
                { x: 780, y: 314 },
                { x: 780, y: 380 },
                { x: 780, y: 447 },
                { x: 780, y: 513 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("പ"),
    },
  ],
  // Thooval's VA.png is പ with a taller arch: from the left foot clockwise
  // over the arch, down to the line, right along the base and up the
  // upright to its top.
  [
    "malayalam:വ",
    {
      script: "malayalam",
      glyph: "വ",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the arch",
              path: [
                { x: 191, y: 47 },
                { x: 143, y: 96 },
                { x: 107, y: 155 },
                { x: 89, y: 221 },
                { x: 87, y: 290 },
                { x: 95, y: 358 },
                { x: 122, y: 420 },
                { x: 168, y: 471 },
                { x: 226, y: 508 },
                { x: 292, y: 525 },
                { x: 361, y: 527 },
                { x: 429, y: 518 },
                { x: 491, y: 488 },
                { x: 540, y: 440 },
                { x: 573, y: 380 },
                { x: 587, y: 313 },
                { x: 587, y: 244 },
                { x: 574, y: 177 },
                { x: 542, y: 116 },
                { x: 511, y: 55 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 511, y: 55 },
                { x: 573, y: 35 },
                { x: 640, y: 35 },
                { x: 707, y: 35 },
                { x: 774, y: 35 },
                { x: 839, y: 47 },
              ],
            },
            {
              label: "climb the upright to the top",
              path: [
                { x: 839, y: 47 },
                { x: 843, y: 113 },
                { x: 843, y: 179 },
                { x: 843, y: 246 },
                { x: 843, y: 312 },
                { x: 843, y: 378 },
                { x: 843, y: 445 },
                { x: 843, y: 511 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("വ"),
    },
  ],
  // Thooval's NNA.png starts inside the small loop (green), circles it
  // clockwise into the big left arch, and then writes the two stems as ന
  // writes its one: down to the line (blue), back up and over the next arch,
  // finishing at the foot of the right arch.
  [
    "malayalam:ണ",
    {
      script: "malayalam",
      glyph: "ണ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small inner loop",
              path: [
                { x: 186, y: 284 },
                { x: 255, y: 288 },
                { x: 320, y: 268 },
                { x: 359, y: 212 },
                { x: 362, y: 143 },
                { x: 346, y: 78 },
                { x: 293, y: 34 },
                { x: 225, y: 24 },
                { x: 160, y: 47 },
                { x: 114, y: 98 },
                { x: 91, y: 163 },
                { x: 90, y: 232 },
              ],
            },
            {
              label: "climb over the big left arch",
              path: [
                { x: 90, y: 232 },
                { x: 90, y: 300 },
                { x: 105, y: 366 },
                { x: 138, y: 425 },
                { x: 185, y: 474 },
                { x: 244, y: 508 },
                { x: 309, y: 525 },
                { x: 377, y: 528 },
                { x: 444, y: 518 },
                { x: 502, y: 484 },
                { x: 550, y: 436 },
              ],
            },
            {
              label: "descend the first stem",
              path: [
                { x: 550, y: 436 },
                { x: 566, y: 374 },
                { x: 566, y: 307 },
                { x: 566, y: 240 },
                { x: 566, y: 174 },
                { x: 566, y: 107 },
                { x: 566, y: 40 },
              ],
            },
            {
              label: "climb back up over the middle arch",
              path: [
                { x: 566, y: 40 },
                { x: 566, y: 112 },
                { x: 566, y: 184 },
                { x: 566, y: 255 },
                { x: 566, y: 327 },
                { x: 566, y: 399 },
                { x: 601, y: 459 },
                { x: 654, y: 507 },
                { x: 722, y: 528 },
                { x: 793, y: 524 },
                { x: 854, y: 488 },
              ],
            },
            {
              label: "descend the second stem",
              path: [
                { x: 854, y: 488 },
                { x: 899, y: 438 },
                { x: 914, y: 375 },
                { x: 914, y: 308 },
                { x: 914, y: 241 },
                { x: 910, y: 174 },
                { x: 910, y: 107 },
                { x: 910, y: 40 },
              ],
            },
            {
              label: "climb back up and round the right arch",
              path: [
                { x: 910, y: 40 },
                { x: 910, y: 111 },
                { x: 910, y: 183 },
                { x: 910, y: 254 },
                { x: 910, y: 326 },
                { x: 914, y: 397 },
                { x: 946, y: 458 },
                { x: 998, y: 508 },
                { x: 1065, y: 527 },
                { x: 1137, y: 524 },
                { x: 1200, y: 493 },
                { x: 1247, y: 439 },
                { x: 1274, y: 373 },
                { x: 1282, y: 303 },
                { x: 1281, y: 231 },
                { x: 1261, y: 163 },
                { x: 1227, y: 100 },
                { x: 1178, y: 48 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ണ"),
    },
  ],
  // Thooval's TTA.png starts at the top-right tip and curves left over
  // the top and down, swings right through the middle, and rounds the
  // lower bowl to finish at the lower-left tail.
  [
    "malayalam:ട",
    {
      script: "malayalam",
      glyph: "ട",
      strokes: [
        {
          segments: [
            {
              label: "curve left over the top and down",
              path: [
                { x: 476, y: 484 },
                { x: 415, y: 508 },
                { x: 351, y: 523 },
                { x: 286, y: 528 },
                { x: 220, y: 528 },
                { x: 157, y: 511 },
                { x: 104, y: 473 },
                { x: 84, y: 412 },
                { x: 92, y: 348 },
              ],
            },
            {
              label: "swing right through the middle",
              path: [
                { x: 92, y: 348 },
                { x: 145, y: 301 },
                { x: 214, y: 285 },
                { x: 286, y: 280 },
                { x: 357, y: 278 },
                { x: 424, y: 257 },
                { x: 476, y: 208 },
              ],
            },
            {
              label: "round the lower bowl to the left",
              path: [
                { x: 476, y: 208 },
                { x: 483, y: 142 },
                { x: 455, y: 83 },
                { x: 403, y: 43 },
                { x: 339, y: 28 },
                { x: 272, y: 24 },
                { x: 206, y: 30 },
                { x: 142, y: 47 },
                { x: 80, y: 72 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ട"),
    },
  ],
  // Thooval's DA.png climbs from the left foot clockwise over the top and
  // curves in to the middle tip (blue), then runs back right and round
  // the lower bowl to finish at its lower-left end. In the Noto outline the
  // two bowls share one band in the middle, so the path retraces it.
  [
    "malayalam:ദ",
    {
      script: "malayalam",
      glyph: "ദ",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the top",
              path: [
                { x: 112, y: 49 },
                { x: 102, y: 118 },
                { x: 100, y: 188 },
                { x: 100, y: 257 },
                { x: 108, y: 326 },
                { x: 132, y: 392 },
                { x: 174, y: 447 },
                { x: 226, y: 493 },
                { x: 290, y: 520 },
                { x: 359, y: 525 },
                { x: 429, y: 524 },
                { x: 494, y: 501 },
                { x: 540, y: 449 },
              ],
            },
            {
              label: "curve in to the middle tip",
              path: [
                { x: 540, y: 449 },
                { x: 542, y: 374 },
                { x: 496, y: 315 },
                { x: 432, y: 277 },
                { x: 356, y: 277 },
              ],
            },
            {
              label: "retrace and round the lower bowl",
              path: [
                { x: 356, y: 277 },
                { x: 424, y: 277 },
                { x: 485, y: 248 },
                { x: 537, y: 205 },
                { x: 556, y: 141 },
                { x: 536, y: 78 },
                { x: 483, y: 36 },
                { x: 416, y: 25 },
                { x: 348, y: 29 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ദ"),
    },
  ],
  // Thooval's HA.png climbs over the small arch to the line, runs right
  // along the base, climbs the left side of the big loop and sweeps
  // clockwise over it to finish at its right foot.
  [
    "malayalam:ഹ",
    {
      script: "malayalam",
      glyph: "ഹ",
      strokes: [
        {
          segments: [
            {
              label: "climb over the small arch",
              path: [
                { x: 124, y: 47 },
                { x: 85, y: 107 },
                { x: 76, y: 177 },
                { x: 98, y: 244 },
                { x: 151, y: 292 },
                { x: 220, y: 307 },
                { x: 291, y: 302 },
                { x: 349, y: 261 },
                { x: 381, y: 198 },
                { x: 381, y: 127 },
                { x: 360, y: 59 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 360, y: 59 },
                { x: 417, y: 35 },
                { x: 484, y: 35 },
                { x: 551, y: 35 },
                { x: 618, y: 35 },
                { x: 680, y: 55 },
              ],
            },
            {
              label: "climb the left side of the loop",
              path: [
                { x: 680, y: 55 },
                { x: 648, y: 116 },
                { x: 617, y: 177 },
                { x: 603, y: 244 },
                { x: 601, y: 313 },
                { x: 618, y: 379 },
                { x: 652, y: 439 },
              ],
            },
            {
              label: "sweep over and down to the foot",
              path: [
                { x: 652, y: 439 },
                { x: 702, y: 486 },
                { x: 764, y: 516 },
                { x: 832, y: 527 },
                { x: 900, y: 528 },
                { x: 966, y: 510 },
                { x: 1025, y: 474 },
                { x: 1068, y: 421 },
                { x: 1096, y: 358 },
                { x: 1104, y: 290 },
                { x: 1103, y: 221 },
                { x: 1083, y: 155 },
                { x: 1048, y: 96 },
                { x: 1000, y: 47 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഹ"),
    },
  ],
  // Thooval's GA.png starts at the top of the left bowl and curves
  // counterclockwise down round it, climbs the middle stem and sweeps
  // clockwise over the right arch to its foot.
  [
    "malayalam:ഗ",
    {
      script: "malayalam",
      glyph: "ഗ",
      strokes: [
        {
          segments: [
            {
              label: "curve down around the left bowl",
              path: [
                { x: 183, y: 504 },
                { x: 140, y: 453 },
                { x: 107, y: 394 },
                { x: 90, y: 329 },
                { x: 86, y: 262 },
                { x: 88, y: 195 },
                { x: 109, y: 131 },
                { x: 149, y: 78 },
                { x: 203, y: 38 },
                { x: 269, y: 28 },
                { x: 336, y: 32 },
                { x: 395, y: 64 },
              ],
            },
            {
              label: "climb the middle stem",
              path: [
                { x: 395, y: 64 },
                { x: 429, y: 118 },
                { x: 443, y: 181 },
                { x: 447, y: 245 },
                { x: 447, y: 309 },
                { x: 452, y: 374 },
                { x: 467, y: 436 },
              ],
            },
            {
              label: "sweep over the right arch to its foot",
              path: [
                { x: 467, y: 436 },
                { x: 506, y: 490 },
                { x: 564, y: 522 },
                { x: 631, y: 528 },
                { x: 697, y: 515 },
                { x: 748, y: 471 },
                { x: 789, y: 418 },
                { x: 808, y: 353 },
                { x: 811, y: 286 },
                { x: 808, y: 219 },
                { x: 787, y: 155 },
                { x: 753, y: 97 },
                { x: 707, y: 48 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഗ"),
    },
  ],
  // Thooval's RRA.png is one arch: from the left foot up the left side and
  // clockwise over the top, down to the right foot.
  [
    "malayalam:റ",
    {
      script: "malayalam",
      glyph: "റ",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side",
              path: [
                { x: 191, y: 47 },
                { x: 138, y: 102 },
                { x: 102, y: 168 },
                { x: 87, y: 242 },
                { x: 87, y: 318 },
                { x: 107, y: 391 },
              ],
            },
            {
              label: "arch over and down to the foot",
              path: [
                { x: 107, y: 391 },
                { x: 144, y: 447 },
                { x: 195, y: 492 },
                { x: 257, y: 519 },
                { x: 324, y: 527 },
                { x: 392, y: 526 },
                { x: 457, y: 507 },
                { x: 513, y: 469 },
                { x: 556, y: 416 },
                { x: 581, y: 353 },
                { x: 587, y: 286 },
                { x: 585, y: 218 },
                { x: 566, y: 153 },
                { x: 531, y: 96 },
                { x: 483, y: 47 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("റ"),
    },
  ],
  // Thooval's LA.png starts on the middle bar (green), draws it to the
  // right, climbs and arches back counterclockwise over the top, descends
  // the left side, runs right along the base and climbs the upright.
  [
    "malayalam:ല",
    {
      script: "malayalam",
      glyph: "ല",
      strokes: [
        {
          segments: [
            {
              label: "draw the middle bar to the right",
              path: [
                { x: 123, y: 256 },
                { x: 198, y: 256 },
                { x: 273, y: 256 },
                { x: 348, y: 256 },
                { x: 423, y: 256 },
                { x: 495, y: 272 },
              ],
            },
            {
              label: "climb and arch back over the top",
              path: [
                { x: 495, y: 272 },
                { x: 499, y: 340 },
                { x: 491, y: 407 },
                { x: 460, y: 467 },
                { x: 409, y: 511 },
                { x: 343, y: 528 },
                { x: 275, y: 530 },
                { x: 209, y: 514 },
                { x: 156, y: 473 },
                { x: 119, y: 416 },
              ],
            },
            {
              label: "descend the left side",
              path: [
                { x: 119, y: 416 },
                { x: 107, y: 343 },
                { x: 109, y: 269 },
                { x: 107, y: 196 },
                { x: 107, y: 122 },
                { x: 111, y: 48 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 111, y: 48 },
                { x: 179, y: 36 },
                { x: 249, y: 36 },
                { x: 320, y: 36 },
                { x: 390, y: 36 },
                { x: 460, y: 36 },
                { x: 531, y: 36 },
                { x: 601, y: 36 },
                { x: 671, y: 36 },
                { x: 739, y: 48 },
              ],
            },
            {
              label: "climb the upright to the top",
              path: [
                { x: 739, y: 48 },
                { x: 743, y: 114 },
                { x: 743, y: 180 },
                { x: 743, y: 247 },
                { x: 747, y: 313 },
                { x: 747, y: 379 },
                { x: 747, y: 446 },
                { x: 747, y: 512 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ല"),
    },
  ],
  // Thooval's SHA.png begins like ഗ, down round the left bowl and up the
  // middle stem, then sweeps over the top and down the right side, rounds
  // the base and curls up inside the right loop to finish (red).
  [
    "malayalam:ശ",
    {
      script: "malayalam",
      glyph: "ശ",
      strokes: [
        {
          segments: [
            {
              label: "curve down around the left bowl",
              path: [
                { x: 183, y: 504 },
                { x: 140, y: 453 },
                { x: 107, y: 394 },
                { x: 90, y: 329 },
                { x: 86, y: 262 },
                { x: 88, y: 195 },
                { x: 109, y: 131 },
                { x: 149, y: 78 },
                { x: 203, y: 38 },
                { x: 269, y: 28 },
                { x: 336, y: 32 },
                { x: 395, y: 64 },
              ],
            },
            {
              label: "climb the middle stem",
              path: [
                { x: 395, y: 64 },
                { x: 433, y: 129 },
                { x: 445, y: 204 },
                { x: 447, y: 280 },
                { x: 455, y: 356 },
                { x: 479, y: 428 },
              ],
            },
            {
              label: "sweep over and down the right side",
              path: [
                { x: 479, y: 428 },
                { x: 524, y: 480 },
                { x: 584, y: 515 },
                { x: 652, y: 527 },
                { x: 722, y: 528 },
                { x: 790, y: 514 },
                { x: 852, y: 483 },
                { x: 902, y: 435 },
                { x: 936, y: 375 },
                { x: 948, y: 307 },
                { x: 959, y: 239 },
                { x: 953, y: 170 },
                { x: 931, y: 104 },
              ],
            },
            {
              label: "round the base and curl up inside",
              path: [
                { x: 931, y: 104 },
                { x: 885, y: 55 },
                { x: 824, y: 27 },
                { x: 756, y: 26 },
                { x: 696, y: 55 },
                { x: 655, y: 108 },
                { x: 639, y: 173 },
                { x: 641, y: 241 },
                { x: 665, y: 304 },
                { x: 714, y: 350 },
                { x: 779, y: 367 },
                { x: 845, y: 359 },
                { x: 907, y: 332 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ശ"),
    },
  ],
  // Thooval's BA.png begins like ണ, round the small loop and over the big
  // left arch, down the stem to the line (blue) and back up it; then it
  // rounds the right arch down into the base, runs right and climbs the
  // upright to its top.
  [
    "malayalam:ബ",
    {
      script: "malayalam",
      glyph: "ബ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small inner loop",
              path: [
                { x: 186, y: 284 },
                { x: 255, y: 288 },
                { x: 320, y: 268 },
                { x: 359, y: 212 },
                { x: 362, y: 143 },
                { x: 346, y: 78 },
                { x: 293, y: 34 },
                { x: 225, y: 24 },
                { x: 160, y: 47 },
                { x: 114, y: 98 },
                { x: 91, y: 163 },
                { x: 90, y: 232 },
              ],
            },
            {
              label: "climb over the big left arch",
              path: [
                { x: 90, y: 232 },
                { x: 90, y: 300 },
                { x: 105, y: 366 },
                { x: 138, y: 425 },
                { x: 185, y: 474 },
                { x: 244, y: 508 },
                { x: 309, y: 525 },
                { x: 377, y: 528 },
                { x: 444, y: 518 },
                { x: 502, y: 484 },
                { x: 550, y: 436 },
              ],
            },
            {
              label: "descend the stem to the line",
              path: [
                { x: 550, y: 436 },
                { x: 566, y: 373 },
                { x: 566, y: 307 },
                { x: 566, y: 240 },
                { x: 566, y: 173 },
                { x: 566, y: 107 },
                { x: 566, y: 40 },
              ],
            },
            {
              label: "climb back up over the right arch",
              path: [
                { x: 566, y: 40 },
                { x: 566, y: 110 },
                { x: 566, y: 181 },
                { x: 566, y: 251 },
                { x: 566, y: 322 },
                { x: 566, y: 392 },
                { x: 592, y: 454 },
                { x: 643, y: 502 },
                { x: 708, y: 526 },
                { x: 778, y: 526 },
                { x: 843, y: 501 },
                { x: 892, y: 450 },
                { x: 923, y: 387 },
                { x: 934, y: 318 },
                { x: 938, y: 248 },
              ],
            },
            {
              label: "curve down and run along the base",
              path: [
                { x: 938, y: 248 },
                { x: 922, y: 183 },
                { x: 893, y: 122 },
                { x: 864, y: 62 },
                { x: 919, y: 36 },
                { x: 986, y: 36 },
                { x: 1054, y: 36 },
                { x: 1121, y: 36 },
                { x: 1186, y: 48 },
              ],
            },
            {
              label: "climb the upright to the top",
              path: [
                { x: 1186, y: 48 },
                { x: 1190, y: 114 },
                { x: 1190, y: 180 },
                { x: 1190, y: 247 },
                { x: 1190, y: 313 },
                { x: 1190, y: 379 },
                { x: 1190, y: 446 },
                { x: 1190, y: 512 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ബ"),
    },
  ],
];
