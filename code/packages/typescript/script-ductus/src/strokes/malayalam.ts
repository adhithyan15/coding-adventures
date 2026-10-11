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

// The script's own digits keep their cited stroke order on their digit row
// (malayalam.json `digits`), as the Kannada digits do. A digit row without a
// `strokeOrderSource` is recognition only and has no ductus.
const malayalamDigitSource = (glyph: string): StrokeSource => {
  const digit = (malayalam.digits ?? []).find(
    (candidate) => candidate.glyph === glyph,
  );
  if (!digit || !("strokeOrderSource" in digit) || !digit.strokeOrderSource) {
    throw new Error(`Malayalam digit ${glyph} has no verified source`);
  }
  return digit.strokeOrderSource;
};

// The anusvara and the vowel signs keep their cited stroke order on their
// mark record (malayalam.json `marks`), like the Gujarati and Devanagari signs.
const malayalamMarkSource = (mark: string): StrokeSource => {
  const record = (malayalam.marks ?? []).find(
    (candidate) => candidate.mark === mark,
  );
  if (
    !record ||
    !("strokeOrderSource" in record) ||
    !record.strokeOrderSource
  ) {
    throw new Error(`Malayalam mark ${mark} has no verified source`);
  }
  return record.strokeOrderSource;
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
  // ---------------------------------------------------------------------
  // Glyphs cited to Rodney F. Moag's Malayalam: A University Course and
  // Reference Grammar (UT Austin South Asia Institute / COERLL, April 2018, CC
  // BY-NC-SA 4.0), whose Tables II-IV ('How to Write ... Symbols') number
  // every movement of every letter and sign, with arrows, in the hand of a
  // native writer, Thomas Joseph. Only facts are cited (order, start,
  // direction, end); no drawing is copied, so nothing under the book's licence
  // is reproduced. Moag's numbers mark MOVEMENTS, not pen lifts: each numbered
  // movement is one segment of one pen-down run, and the lift count comes from
  // recorded writers instead (santhoshtr/hand, MIT, and grahyam's samples,
  // counts only), which show one run for every letter and vowel sign below.
  // Every path follows the skeleton of the bundled Noto Sans Malayalam outline
  // between the points Moag's arrows mark, at default tolerance; where the
  // next movement starts on ink already drawn, the path retraces that ink
  // rather than lifting.
  // ---------------------------------------------------------------------
  // Moag's Table IV (p. xxv) writes ക: it starts on the left leg of the top
  // loop at the height of the crossbar, climbs clockwise over the loop and
  // down its right leg (1), rounds the bottom of the middle bowl to the left
  // (2), climbs the middle stem (3), runs back down into the left bowl (4),
  // rounds the left bowl up to the left end of the crossbar (5), draws the
  // crossbar left to right (6) and curls the right-hand hook down to finish at
  // the lower right (7).
  [
    "malayalam:ക",
    {
      script: "malayalam",
      glyph: "ക",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the top loop",
              path: [
                { x: 332, y: 292 },
                { x: 338, y: 360 },
                { x: 358, y: 426 },
                { x: 398, y: 482 },
                { x: 455, y: 519 },
                { x: 523, y: 528 },
                { x: 591, y: 520 },
                { x: 646, y: 482 },
                { x: 686, y: 426 },
                { x: 706, y: 360 },
                { x: 712, y: 292 },
              ],
            },
            {
              label: "round the bottom to the left",
              path: [
                { x: 712, y: 292 },
                { x: 710, y: 224 },
                { x: 698, y: 158 },
                { x: 668, y: 98 },
                { x: 620, y: 50 },
                { x: 557, y: 26 },
                { x: 489, y: 26 },
                { x: 426, y: 48 },
                { x: 373, y: 89 },
                { x: 348, y: 152 },
              ],
            },
            {
              label: "climb the middle stem",
              path: [
                { x: 348, y: 152 },
                { x: 334, y: 221 },
                { x: 332, y: 292 },
              ],
            },
            {
              label: "retrace down into the left bowl",
              path: [
                { x: 332, y: 292 },
                { x: 334, y: 221 },
                { x: 348, y: 152 },
                { x: 365, y: 85 },
                { x: 312, y: 42 },
                { x: 243, y: 28 },
                { x: 172, y: 28 },
              ],
            },
            {
              label: "round the left bowl to the bar",
              path: [
                { x: 172, y: 28 },
                { x: 112, y: 58 },
                { x: 80, y: 116 },
                { x: 79, y: 184 },
                { x: 115, y: 239 },
                { x: 172, y: 276 },
              ],
            },
            {
              label: "draw the crossbar to the right",
              path: [
                { x: 172, y: 276 },
                { x: 237, y: 289 },
                { x: 304, y: 292 },
                { x: 370, y: 292 },
                { x: 437, y: 292 },
                { x: 504, y: 292 },
                { x: 570, y: 292 },
                { x: 637, y: 292 },
                { x: 703, y: 292 },
                { x: 770, y: 292 },
                { x: 836, y: 284 },
              ],
            },
            {
              label: "curl the right hook down",
              path: [
                { x: 836, y: 284 },
                { x: 897, y: 261 },
                { x: 944, y: 215 },
                { x: 960, y: 153 },
                { x: 949, y: 90 },
                { x: 904, y: 43 },
                { x: 841, y: 25 },
                { x: 776, y: 28 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ക"),
    },
  ],
  // Moag's Table IV (p. xxx) writes യ: it starts at the top of the left bowl
  // and runs down its left side anticlockwise to the bottom (1), climbs
  // through the crossing to the top of the middle loop (2), comes back down
  // the loop's inner side (3) and rounds the right bowl to finish at the top
  // right (4).
  [
    "malayalam:യ",
    {
      script: "malayalam",
      glyph: "യ",
      strokes: [
        {
          segments: [
            {
              label: "curve down round the left bowl",
              path: [
                { x: 255, y: 532 },
                { x: 188, y: 518 },
                { x: 133, y: 478 },
                { x: 97, y: 420 },
                { x: 84, y: 353 },
                { x: 86, y: 285 },
                { x: 101, y: 218 },
                { x: 129, y: 155 },
                { x: 171, y: 102 },
                { x: 226, y: 60 },
                { x: 289, y: 34 },
                { x: 357, y: 24 },
                { x: 425, y: 25 },
                { x: 487, y: 52 },
              ],
            },
            {
              label: "climb to the top of the middle loop",
              path: [
                { x: 487, y: 52 },
                { x: 553, y: 60 },
                { x: 601, y: 106 },
                { x: 642, y: 159 },
                { x: 672, y: 220 },
                { x: 687, y: 285 },
                { x: 691, y: 352 },
                { x: 678, y: 418 },
                { x: 644, y: 475 },
                { x: 592, y: 517 },
                { x: 527, y: 532 },
              ],
            },
            {
              label: "come back down its inner side",
              path: [
                { x: 527, y: 532 },
                { x: 460, y: 519 },
                { x: 403, y: 482 },
                { x: 371, y: 423 },
                { x: 359, y: 356 },
                { x: 359, y: 288 },
                { x: 373, y: 221 },
                { x: 403, y: 159 },
                { x: 443, y: 105 },
                { x: 487, y: 52 },
              ],
            },
            {
              label: "round the right bowl to the top",
              path: [
                { x: 487, y: 52 },
                { x: 558, y: 50 },
                { x: 623, y: 24 },
                { x: 694, y: 24 },
                { x: 764, y: 36 },
                { x: 828, y: 65 },
                { x: 881, y: 112 },
                { x: 919, y: 171 },
                { x: 942, y: 238 },
                { x: 947, y: 309 },
                { x: 944, y: 379 },
                { x: 924, y: 448 },
                { x: 895, y: 512 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("യ"),
    },
  ],
  // Moag's Table II (p. xviii) writes ഏ: it starts at the foot of the small
  // left arch and climbs clockwise over it to the bar (1), draws the bar to
  // the right (2), climbs the stem (3), runs back down it (4), climbs
  // clockwise over the big arch (5), comes down its right side to the middle
  // (6) and rounds the lower bowl to finish at the bottom right (7).
  [
    "malayalam:ഏ",
    {
      script: "malayalam",
      glyph: "ഏ",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the small arch",
              path: [
                { x: 124, y: 44 },
                { x: 85, y: 106 },
                { x: 76, y: 177 },
                { x: 100, y: 244 },
                { x: 154, y: 293 },
                { x: 224, y: 308 },
                { x: 296, y: 300 },
                { x: 353, y: 255 },
                { x: 383, y: 190 },
                { x: 378, y: 118 },
                { x: 360, y: 48 },
              ],
            },
            {
              label: "run right along the bar",
              path: [
                { x: 360, y: 48 },
                { x: 424, y: 36 },
                { x: 489, y: 36 },
                { x: 555, y: 36 },
                { x: 621, y: 34 },
                { x: 687, y: 32 },
                { x: 753, y: 32 },
                { x: 819, y: 32 },
                { x: 884, y: 36 },
              ],
            },
            {
              label: "climb the stem",
              path: [
                { x: 884, y: 36 },
                { x: 888, y: 101 },
                { x: 888, y: 167 },
                { x: 892, y: 233 },
                { x: 892, y: 298 },
                { x: 892, y: 364 },
              ],
            },
            {
              label: "retrace it down and round the loop",
              path: [
                { x: 892, y: 364 },
                { x: 892, y: 296 },
                { x: 892, y: 229 },
                { x: 892, y: 161 },
                { x: 892, y: 94 },
                { x: 885, y: 27 },
                { x: 888, y: -41 },
                { x: 888, y: -108 },
                { x: 859, y: -168 },
                { x: 798, y: -192 },
                { x: 733, y: -179 },
                { x: 682, y: -134 },
                { x: 647, y: -77 },
                { x: 628, y: -12 },
              ],
            },
            {
              label: "climb clockwise over the big arch",
              path: [
                { x: 628, y: -12 },
                { x: 616, y: 57 },
                { x: 616, y: 128 },
                { x: 616, y: 199 },
                { x: 627, y: 269 },
                { x: 647, y: 336 },
                { x: 679, y: 400 },
                { x: 725, y: 453 },
                { x: 782, y: 494 },
                { x: 849, y: 518 },
                { x: 919, y: 528 },
                { x: 989, y: 529 },
                { x: 1058, y: 513 },
                { x: 1117, y: 475 },
                { x: 1163, y: 422 },
                { x: 1182, y: 354 },
                { x: 1180, y: 284 },
              ],
            },
            {
              label: "come down to the middle",
              path: [
                { x: 1180, y: 284 },
                { x: 1148, y: 225 },
                { x: 1101, y: 177 },
                { x: 1044, y: 144 },
              ],
            },
            {
              label: "round the lower bowl",
              path: [
                { x: 1044, y: 144 },
                { x: 1102, y: 115 },
                { x: 1154, y: 75 },
                { x: 1186, y: 18 },
                { x: 1188, y: -49 },
                { x: 1179, y: -113 },
                { x: 1134, y: -162 },
                { x: 1074, y: -188 },
                { x: 1008, y: -192 },
              ],
            },
          ],
        },
      ],
      source: malayalamIndependentVowelSource("ഏ"),
    },
  ],
  // Moag's Table IV (p. xxv) writes ഖ: it circles the small inner loop (1),
  // arches over the top and comes down to the base (2), runs right along the
  // base (3) and climbs the upright (4).
  [
    "malayalam:ഖ",
    {
      script: "malayalam",
      glyph: "ഖ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small inner loop",
              path: [
                { x: 183, y: 284 },
                { x: 252, y: 288 },
                { x: 317, y: 268 },
                { x: 358, y: 213 },
                { x: 363, y: 144 },
                { x: 343, y: 79 },
                { x: 291, y: 34 },
                { x: 223, y: 24 },
                { x: 157, y: 47 },
                { x: 111, y: 97 },
                { x: 88, y: 162 },
                { x: 87, y: 232 },
              ],
            },
            {
              label: "arch over and down to the base",
              path: [
                { x: 87, y: 232 },
                { x: 88, y: 303 },
                { x: 108, y: 371 },
                { x: 147, y: 430 },
                { x: 200, y: 478 },
                { x: 263, y: 511 },
                { x: 332, y: 526 },
                { x: 403, y: 528 },
                { x: 473, y: 515 },
                { x: 536, y: 483 },
                { x: 585, y: 432 },
                { x: 617, y: 369 },
                { x: 630, y: 299 },
                { x: 629, y: 228 },
                { x: 608, y: 160 },
                { x: 572, y: 99 },
                { x: 531, y: 44 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 531, y: 44 },
                { x: 601, y: 36 },
                { x: 671, y: 36 },
                { x: 741, y: 36 },
                { x: 811, y: 36 },
                { x: 879, y: 48 },
              ],
            },
            {
              label: "climb the upright to the top",
              path: [
                { x: 879, y: 48 },
                { x: 883, y: 114 },
                { x: 883, y: 180 },
                { x: 883, y: 247 },
                { x: 887, y: 313 },
                { x: 887, y: 379 },
                { x: 887, y: 446 },
                { x: 887, y: 512 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഖ"),
    },
  ],
  // Moag's Table IV (p. xxv) writes ങ: it circles the small inner loop (1),
  // arches over to the stem (2), runs down the stem (3), draws the upper right
  // bowl (4) and rounds the lower bowl to finish at the bottom (5).
  [
    "malayalam:ങ",
    {
      script: "malayalam",
      glyph: "ങ",
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
              label: "arch over to the stem",
              path: [
                { x: 90, y: 232 },
                { x: 90, y: 300 },
                { x: 105, y: 366 },
                { x: 138, y: 425 },
                { x: 185, y: 474 },
                { x: 243, y: 508 },
                { x: 309, y: 525 },
                { x: 377, y: 528 },
                { x: 443, y: 518 },
                { x: 500, y: 482 },
                { x: 546, y: 432 },
              ],
            },
            {
              label: "descend the stem to the line",
              path: [
                { x: 546, y: 432 },
                { x: 570, y: 373 },
                { x: 566, y: 307 },
                { x: 566, y: 240 },
                { x: 566, y: 173 },
                { x: 566, y: 107 },
                { x: 566, y: 40 },
              ],
            },
            {
              label: "climb back and round the top bowl",
              path: [
                { x: 566, y: 40 },
                { x: 566, y: 110 },
                { x: 566, y: 180 },
                { x: 566, y: 249 },
                { x: 566, y: 319 },
                { x: 570, y: 389 },
                { x: 609, y: 439 },
                { x: 660, y: 487 },
                { x: 722, y: 518 },
                { x: 791, y: 528 },
                { x: 860, y: 525 },
                { x: 924, y: 497 },
                { x: 965, y: 442 },
                { x: 967, y: 373 },
                { x: 924, y: 319 },
                { x: 868, y: 278 },
                { x: 798, y: 276 },
              ],
            },
            {
              label: "round the lower bowl",
              path: [
                { x: 798, y: 276 },
                { x: 871, y: 273 },
                { x: 931, y: 231 },
                { x: 977, y: 174 },
                { x: 976, y: 102 },
                { x: 928, y: 47 },
                { x: 859, y: 25 },
                { x: 786, y: 24 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ങ"),
    },
  ],
  // Moag's Table IV (p. xxvi) writes ച: it starts at the inner tip and curls
  // clockwise over the top and down (1), runs left along the base to its end
  // (2), runs back right along the whole base (3) and climbs the upright (4).
  [
    "malayalam:ച",
    {
      script: "malayalam",
      glyph: "ച",
      strokes: [
        {
          segments: [
            {
              label: "curl clockwise over the top",
              path: [
                { x: 205, y: 176 },
                { x: 213, y: 243 },
                { x: 257, y: 296 },
                { x: 317, y: 326 },
                { x: 385, y: 328 },
                { x: 447, y: 301 },
                { x: 485, y: 244 },
                { x: 497, y: 178 },
                { x: 481, y: 112 },
                { x: 457, y: 48 },
              ],
            },
            {
              label: "run left along the base",
              path: [
                { x: 457, y: 48 },
                { x: 390, y: 36 },
                { x: 322, y: 36 },
                { x: 254, y: 36 },
                { x: 186, y: 36 },
                { x: 117, y: 36 },
                { x: 49, y: 36 },
              ],
            },
            {
              label: "run back right along the base",
              path: [
                { x: 49, y: 36 },
                { x: 119, y: 36 },
                { x: 188, y: 36 },
                { x: 258, y: 36 },
                { x: 328, y: 36 },
                { x: 397, y: 36 },
                { x: 467, y: 36 },
                { x: 537, y: 36 },
                { x: 606, y: 36 },
                { x: 676, y: 36 },
                { x: 746, y: 36 },
                { x: 813, y: 48 },
              ],
            },
            {
              label: "climb the upright to the top",
              path: [
                { x: 813, y: 48 },
                { x: 817, y: 114 },
                { x: 817, y: 180 },
                { x: 817, y: 247 },
                { x: 821, y: 313 },
                { x: 821, y: 379 },
                { x: 821, y: 446 },
                { x: 821, y: 512 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ച"),
    },
  ],
  // Moag's Table IV (p. xxvi) writes ഛ: it starts at the inner tip and curls
  // clockwise over the top and down (1), runs left along the base (2) and back
  // right along it (3), then climbs clockwise over the big arch and down its
  // right side (4) and curls up round the inner loop (5).
  [
    "malayalam:ഛ",
    {
      script: "malayalam",
      glyph: "ഛ",
      strokes: [
        {
          segments: [
            {
              label: "curl clockwise over the top",
              path: [
                { x: 205, y: 176 },
                { x: 213, y: 243 },
                { x: 257, y: 296 },
                { x: 317, y: 326 },
                { x: 385, y: 327 },
                { x: 447, y: 300 },
                { x: 485, y: 244 },
                { x: 497, y: 177 },
                { x: 481, y: 111 },
                { x: 457, y: 48 },
              ],
            },
            {
              label: "run left along the base",
              path: [
                { x: 457, y: 48 },
                { x: 390, y: 36 },
                { x: 322, y: 36 },
                { x: 254, y: 36 },
                { x: 186, y: 36 },
                { x: 117, y: 36 },
                { x: 49, y: 36 },
              ],
            },
            {
              label: "run back right along the base",
              path: [
                { x: 49, y: 36 },
                { x: 122, y: 36 },
                { x: 195, y: 36 },
                { x: 268, y: 36 },
                { x: 342, y: 36 },
                { x: 415, y: 36 },
                { x: 488, y: 36 },
                { x: 561, y: 36 },
                { x: 634, y: 36 },
                { x: 707, y: 36 },
                { x: 777, y: 52 },
              ],
            },
            {
              label: "climb clockwise over the big arch",
              path: [
                { x: 777, y: 52 },
                { x: 747, y: 113 },
                { x: 715, y: 174 },
                { x: 700, y: 241 },
                { x: 698, y: 309 },
                { x: 715, y: 376 },
                { x: 748, y: 436 },
                { x: 799, y: 482 },
                { x: 860, y: 512 },
                { x: 928, y: 526 },
                { x: 996, y: 532 },
                { x: 1064, y: 521 },
                { x: 1129, y: 499 },
                { x: 1185, y: 460 },
                { x: 1228, y: 407 },
                { x: 1249, y: 342 },
                { x: 1266, y: 276 },
                { x: 1268, y: 207 },
                { x: 1255, y: 139 },
                { x: 1221, y: 80 },
                { x: 1168, y: 38 },
                { x: 1101, y: 24 },
              ],
            },
            {
              label: "curl up round the inner loop",
              path: [
                { x: 1101, y: 24 },
                { x: 1031, y: 37 },
                { x: 977, y: 84 },
                { x: 951, y: 150 },
                { x: 947, y: 222 },
                { x: 966, y: 291 },
                { x: 1015, y: 344 },
                { x: 1082, y: 367 },
                { x: 1153, y: 360 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഛ"),
    },
  ],
  // Moag's Table IV (p. xxvi) writes ഞ: it circles the small inner loop (1),
  // arches over to the stem (2), runs down the stem (3) and back up it (4),
  // loops over and round the oval (5), arches over the right side (6) and
  // comes down to finish at the right foot (7).
  [
    "malayalam:ഞ",
    {
      script: "malayalam",
      glyph: "ഞ",
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
              label: "arch over to the stem",
              path: [
                { x: 90, y: 232 },
                { x: 90, y: 300 },
                { x: 105, y: 366 },
                { x: 138, y: 426 },
                { x: 186, y: 474 },
                { x: 245, y: 508 },
                { x: 311, y: 525 },
                { x: 379, y: 528 },
                { x: 445, y: 517 },
                { x: 502, y: 480 },
                { x: 550, y: 432 },
              ],
            },
            {
              label: "descend the stem to the line",
              path: [
                { x: 550, y: 432 },
                { x: 566, y: 371 },
                { x: 566, y: 305 },
                { x: 566, y: 239 },
                { x: 566, y: 172 },
                { x: 566, y: 106 },
                { x: 566, y: 40 },
              ],
            },
            {
              label: "climb the stem again",
              path: [
                { x: 566, y: 40 },
                { x: 566, y: 114 },
                { x: 566, y: 189 },
                { x: 566, y: 263 },
                { x: 566, y: 338 },
                { x: 570, y: 412 },
              ],
            },
            {
              label: "loop over and round the oval",
              path: [
                { x: 570, y: 412 },
                { x: 620, y: 462 },
                { x: 674, y: 505 },
                { x: 740, y: 527 },
                { x: 810, y: 532 },
                { x: 879, y: 522 },
                { x: 944, y: 495 },
                { x: 999, y: 455 },
                { x: 1043, y: 401 },
                { x: 1077, y: 340 },
                { x: 1094, y: 272 },
                { x: 1098, y: 202 },
                { x: 1088, y: 133 },
                { x: 1052, y: 74 },
                { x: 996, y: 33 },
                { x: 927, y: 24 },
                { x: 861, y: 44 },
                { x: 811, y: 93 },
                { x: 785, y: 158 },
                { x: 782, y: 228 },
                { x: 788, y: 297 },
                { x: 814, y: 362 },
                { x: 854, y: 419 },
                { x: 903, y: 469 },
                { x: 966, y: 492 },
              ],
            },
            {
              label: "arch over the right side",
              path: [
                { x: 966, y: 492 },
                { x: 1024, y: 524 },
                { x: 1092, y: 532 },
                { x: 1159, y: 528 },
                { x: 1224, y: 509 },
                { x: 1278, y: 468 },
                { x: 1326, y: 420 },
              ],
            },
            {
              label: "come down to the right foot",
              path: [
                { x: 1326, y: 420 },
                { x: 1352, y: 355 },
                { x: 1362, y: 287 },
                { x: 1358, y: 217 },
                { x: 1337, y: 151 },
                { x: 1300, y: 92 },
                { x: 1254, y: 40 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഞ"),
    },
  ],
  // Moag's Table IV (p. xxviii) writes ഥ: it starts at the top of the left
  // stem and runs down it (1), runs right along the base (2), then climbs the
  // right leg, arches anticlockwise over the top and comes down the inner leg
  // to the base (3).
  [
    "malayalam:ഥ",
    {
      script: "malayalam",
      glyph: "ഥ",
      strokes: [
        {
          segments: [
            {
              label: "descend the left stem",
              path: [
                { x: 115, y: 512 },
                { x: 115, y: 446 },
                { x: 115, y: 379 },
                { x: 115, y: 313 },
                { x: 115, y: 247 },
                { x: 115, y: 180 },
                { x: 115, y: 114 },
                { x: 119, y: 48 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 119, y: 48 },
                { x: 184, y: 36 },
                { x: 251, y: 36 },
                { x: 318, y: 36 },
                { x: 384, y: 44 },
                { x: 450, y: 36 },
                { x: 517, y: 36 },
                { x: 584, y: 36 },
                { x: 651, y: 36 },
                { x: 718, y: 36 },
                { x: 783, y: 48 },
              ],
            },
            {
              label: "climb, arch over and come down",
              path: [
                { x: 783, y: 48 },
                { x: 789, y: 118 },
                { x: 791, y: 189 },
                { x: 791, y: 259 },
                { x: 787, y: 330 },
                { x: 777, y: 399 },
                { x: 743, y: 460 },
                { x: 690, y: 506 },
                { x: 623, y: 526 },
                { x: 552, y: 528 },
                { x: 484, y: 509 },
                { x: 430, y: 466 },
                { x: 397, y: 404 },
                { x: 386, y: 334 },
                { x: 383, y: 264 },
                { x: 383, y: 193 },
                { x: 383, y: 123 },
                { x: 383, y: 52 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഥ"),
    },
  ],
  // Moag's Table IV (p. xxviii) writes ധ: it starts at the top left and runs
  // down round the left bowl (1), climbs the middle stem (2), runs back down
  // it (3) and rounds the right bowl to finish at the top right (4).
  [
    "malayalam:ധ",
    {
      script: "malayalam",
      glyph: "ധ",
      strokes: [
        {
          segments: [
            {
              label: "curve down round the left bowl",
              path: [
                { x: 191, y: 508 },
                { x: 146, y: 458 },
                { x: 112, y: 399 },
                { x: 91, y: 335 },
                { x: 87, y: 268 },
                { x: 88, y: 200 },
                { x: 110, y: 136 },
                { x: 151, y: 84 },
                { x: 204, y: 41 },
                { x: 269, y: 28 },
                { x: 337, y: 31 },
                { x: 393, y: 66 },
                { x: 439, y: 116 },
              ],
            },
            {
              label: "climb the middle stem",
              path: [
                { x: 439, y: 116 },
                { x: 455, y: 178 },
                { x: 455, y: 245 },
                { x: 455, y: 312 },
                { x: 455, y: 378 },
                { x: 455, y: 445 },
                { x: 455, y: 512 },
              ],
            },
            {
              label: "retrace the stem down",
              path: [
                { x: 455, y: 512 },
                { x: 455, y: 446 },
                { x: 455, y: 380 },
                { x: 455, y: 313 },
                { x: 455, y: 247 },
                { x: 455, y: 181 },
                { x: 471, y: 120 },
              ],
            },
            {
              label: "round the right bowl to the top",
              path: [
                { x: 471, y: 120 },
                { x: 517, y: 70 },
                { x: 572, y: 32 },
                { x: 639, y: 24 },
                { x: 705, y: 40 },
                { x: 757, y: 82 },
                { x: 799, y: 135 },
                { x: 821, y: 199 },
                { x: 823, y: 267 },
                { x: 822, y: 334 },
                { x: 802, y: 399 },
                { x: 768, y: 457 },
                { x: 723, y: 508 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ധ"),
    },
  ],
  // Moag's Table IV (p. xxix) writes ഭ: it starts at the left foot and climbs
  // clockwise over the top (1), comes down into the middle (2), curls left and
  // then right (3) and rounds the lower bowl to finish at the bottom, heading
  // left (4).
  [
    "malayalam:ഭ",
    {
      script: "malayalam",
      glyph: "ഭ",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the top",
              path: [
                { x: 112, y: 44 },
                { x: 103, y: 115 },
                { x: 96, y: 187 },
                { x: 96, y: 259 },
                { x: 110, y: 329 },
                { x: 137, y: 396 },
                { x: 179, y: 454 },
                { x: 238, y: 496 },
                { x: 305, y: 520 },
                { x: 376, y: 529 },
                { x: 448, y: 528 },
                { x: 516, y: 506 },
                { x: 566, y: 454 },
                { x: 568, y: 384 },
              ],
            },
            {
              label: "come down into the middle",
              path: [
                { x: 568, y: 384 },
                { x: 509, y: 349 },
                { x: 438, y: 348 },
                { x: 368, y: 339 },
                { x: 312, y: 300 },
              ],
            },
            {
              label: "curl left, then right",
              path: [
                { x: 312, y: 300 },
                { x: 323, y: 230 },
                { x: 385, y: 198 },
                { x: 457, y: 195 },
                { x: 528, y: 184 },
              ],
            },
            {
              label: "round the lower bowl to the left",
              path: [
                { x: 528, y: 184 },
                { x: 573, y: 135 },
                { x: 564, y: 70 },
                { x: 509, y: 32 },
                { x: 441, y: 24 },
                { x: 374, y: 28 },
                { x: 308, y: 44 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഭ"),
    },
  ],
  // Moag's Table IV (p. xxix) writes ഫ: it starts at the foot of the small
  // arch and climbs clockwise over it to the base (1), runs right along the
  // base (2), then climbs the right leg, arches anticlockwise over the top and
  // comes down the inner leg to the base (3).
  [
    "malayalam:ഫ",
    {
      script: "malayalam",
      glyph: "ഫ",
      strokes: [
        {
          segments: [
            {
              label: "climb clockwise over the small arch",
              path: [
                { x: 124, y: 45 },
                { x: 85, y: 106 },
                { x: 76, y: 177 },
                { x: 100, y: 244 },
                { x: 153, y: 293 },
                { x: 223, y: 309 },
                { x: 295, y: 300 },
                { x: 351, y: 254 },
                { x: 382, y: 190 },
                { x: 379, y: 118 },
                { x: 360, y: 49 },
              ],
            },
            {
              label: "run right along the base",
              path: [
                { x: 360, y: 49 },
                { x: 431, y: 37 },
                { x: 505, y: 37 },
                { x: 579, y: 37 },
                { x: 650, y: 38 },
                { x: 724, y: 37 },
                { x: 798, y: 37 },
                { x: 871, y: 37 },
                { x: 945, y: 37 },
                { x: 1016, y: 49 },
              ],
            },
            {
              label: "climb, arch over and come down",
              path: [
                { x: 1016, y: 49 },
                { x: 1022, y: 119 },
                { x: 1024, y: 189 },
                { x: 1024, y: 260 },
                { x: 1020, y: 330 },
                { x: 1010, y: 399 },
                { x: 975, y: 458 },
                { x: 923, y: 506 },
                { x: 856, y: 526 },
                { x: 786, y: 528 },
                { x: 718, y: 510 },
                { x: 666, y: 463 },
                { x: 630, y: 403 },
                { x: 617, y: 334 },
                { x: 616, y: 264 },
                { x: 616, y: 194 },
                { x: 616, y: 123 },
                { x: 616, y: 53 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഫ"),
    },
  ],
  // Moag's Table IV (p. xxxi) writes ള: it circles the small inner loop (1),
  // arches over the top and comes down to the middle (2), rounds the right
  // side and runs left (3), then loops down at the left and runs right along
  // the bottom (4).
  [
    "malayalam:ള",
    {
      script: "malayalam",
      glyph: "ള",
      strokes: [
        {
          segments: [
            {
              label: "circle the small inner loop",
              path: [
                { x: 180, y: 379 },
                { x: 254, y: 378 },
                { x: 314, y: 339 },
                { x: 336, y: 270 },
                { x: 314, y: 202 },
                { x: 257, y: 157 },
                { x: 184, y: 152 },
                { x: 122, y: 189 },
                { x: 84, y: 251 },
              ],
            },
            {
              label: "arch over and down to the middle",
              path: [
                { x: 84, y: 251 },
                { x: 85, y: 321 },
                { x: 98, y: 390 },
                { x: 136, y: 447 },
                { x: 193, y: 488 },
                { x: 259, y: 513 },
                { x: 327, y: 526 },
                { x: 397, y: 531 },
                { x: 467, y: 528 },
                { x: 536, y: 514 },
                { x: 594, y: 477 },
                { x: 627, y: 416 },
                { x: 615, y: 349 },
                { x: 567, y: 298 },
                { x: 504, y: 275 },
              ],
            },
            {
              label: "round the right side to the left",
              path: [
                { x: 504, y: 275 },
                { x: 566, y: 256 },
                { x: 616, y: 211 },
                { x: 640, y: 151 },
                { x: 619, y: 90 },
                { x: 567, y: 48 },
                { x: 501, y: 36 },
                { x: 434, y: 35 },
                { x: 367, y: 35 },
                { x: 300, y: 35 },
              ],
            },
            {
              label: "loop down and run right below",
              path: [
                { x: 300, y: 35 },
                { x: 229, y: 35 },
                { x: 160, y: 25 },
                { x: 106, y: -19 },
                { x: 100, y: -88 },
                { x: 141, y: -143 },
                { x: 209, y: -161 },
                { x: 279, y: -165 },
                { x: 350, y: -165 },
                { x: 420, y: -165 },
                { x: 491, y: -165 },
                { x: 561, y: -165 },
                { x: 632, y: -165 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ള"),
    },
  ],
  // The anusvara and the vowel signs, each drawn by itself (no consonant).
  // Moag draws every sign beside a dash that stands for the consonant. Only
  // the anusvara also has a cited written order (after its base, from അം,
  // where it is movement 9): human-language-data's WRITTEN_SIGN_SIDES gives
  // it, and no other Malayalam sign, a row.
  // ം: Moag's Table III (p. xxiv) draws it as one movement to the right of the
  // dash that stands for the consonant; in Table II (p. xix) it is movement 9
  // of അം, after the eight movements of അ. The drawn line starts at the upper
  // left of the ring and runs clockwise, ending with its arrowhead on the left
  // side pointing up.
  [
    "malayalam:ം",
    {
      script: "malayalam",
      glyph: "ം",
      strokes: [
        {
          segments: [
            {
              label: "circle clockwise",
              path: [
                { x: 100, y: 268 },
                { x: 151, y: 312 },
                { x: 216, y: 328 },
                { x: 283, y: 320 },
                { x: 339, y: 284 },
                { x: 372, y: 226 },
                { x: 379, y: 159 },
                { x: 358, y: 95 },
                { x: 312, y: 46 },
                { x: 248, y: 25 },
                { x: 181, y: 28 },
                { x: 123, y: 61 },
                { x: 85, y: 116 },
                { x: 76, y: 182 },
                { x: 88, y: 248 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ം"),
    },
  ],
  // ാ: Moag's Table III (p. xx) draws it as one movement to the right of the
  // dash that stands for the consonant: from the upper tip clockwise round to
  // the lower tip.
  [
    "malayalam:ാ",
    {
      script: "malayalam",
      glyph: "ാ",
      strokes: [
        {
          segments: [
            {
              label: "curve clockwise round to the foot",
              path: [
                { x: 81, y: 424 },
                { x: 121, y: 480 },
                { x: 177, y: 520 },
                { x: 245, y: 528 },
                { x: 312, y: 516 },
                { x: 366, y: 473 },
                { x: 398, y: 412 },
                { x: 413, y: 345 },
                { x: 417, y: 276 },
                { x: 415, y: 207 },
                { x: 398, y: 140 },
                { x: 360, y: 83 },
                { x: 308, y: 38 },
                { x: 241, y: 24 },
                { x: 172, y: 32 },
                { x: 117, y: 72 },
                { x: 77, y: 128 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ാ"),
    },
  ],
  // ി: Moag's Table III (p. xx) draws it as one movement to the right of the
  // dash: from the curled tip over the top and down the stem.
  [
    "malayalam:ി",
    {
      script: "malayalam",
      glyph: "ി",
      strokes: [
        {
          segments: [
            {
              label: "arch over and draw the stem down",
              path: [
                { x: -167, y: 676 },
                { x: -151, y: 744 },
                { x: -100, y: 791 },
                { x: -32, y: 804 },
                { x: 37, y: 792 },
                { x: 86, y: 743 },
                { x: 108, y: 676 },
                { x: 113, y: 606 },
                { x: 113, y: 535 },
                { x: 113, y: 464 },
                { x: 113, y: 393 },
                { x: 113, y: 323 },
                { x: 113, y: 252 },
                { x: 113, y: 181 },
                { x: 113, y: 111 },
                { x: 113, y: 40 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ി"),
    },
  ],
  // ീ: Moag's Table III (p. xx) draws it as two movements to the right of the
  // dash: the small loop at the top (1), then over the top and down the stem
  // (2).
  [
    "malayalam:ീ",
    {
      script: "malayalam",
      glyph: "ീ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small loop",
              path: [
                { x: -55, y: 772 },
                { x: -47, y: 702 },
                { x: -72, y: 639 },
                { x: -136, y: 612 },
                { x: -204, y: 627 },
                { x: -242, y: 684 },
                { x: -230, y: 752 },
                { x: -179, y: 800 },
              ],
            },
            {
              label: "arch over and draw the stem down",
              path: [
                { x: -179, y: 800 },
                { x: -110, y: 808 },
                { x: -42, y: 793 },
                { x: 26, y: 781 },
                { x: 76, y: 733 },
                { x: 102, y: 669 },
                { x: 113, y: 600 },
                { x: 113, y: 530 },
                { x: 113, y: 460 },
                { x: 113, y: 390 },
                { x: 113, y: 320 },
                { x: 113, y: 250 },
                { x: 113, y: 180 },
                { x: 113, y: 110 },
                { x: 113, y: 40 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ീ"),
    },
  ],
  // ു: Moag's Table III (p. xxi) draws it as three movements to the right of
  // the dash: down the stem from its hook (1), round the left side and bottom
  // of the loop at its foot (2) and up the loop's right side to close it (3).
  [
    "malayalam:ു",
    {
      script: "malayalam",
      glyph: "ു",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down from the hook",
              path: [
                { x: 58, y: 529 },
                { x: 126, y: 527 },
                { x: 184, y: 497 },
                { x: 213, y: 437 },
                { x: 218, y: 370 },
                { x: 210, y: 303 },
                { x: 197, y: 236 },
                { x: 182, y: 171 },
                { x: 166, y: 105 },
              ],
            },
            {
              label: "round the left side and bottom",
              path: [
                { x: 166, y: 105 },
                { x: 105, y: 79 },
                { x: 49, y: 42 },
                { x: 20, y: -17 },
                { x: 19, y: -84 },
                { x: 46, y: -145 },
                { x: 99, y: -184 },
                { x: 166, y: -191 },
                { x: 230, y: -175 },
              ],
            },
            {
              label: "climb its right side",
              path: [
                { x: 230, y: -175 },
                { x: 278, y: -122 },
                { x: 294, y: -53 },
                { x: 278, y: 16 },
                { x: 230, y: 69 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ു"),
    },
  ],
  // ൂ: Moag's Table III (p. xxi) draws it as four movements to the right of
  // the dash: down the stem from its hook (1), round the left side and bottom
  // of the loop at its foot (2), up the loop's right side (3) and round the
  // small inner loop (4).
  [
    "malayalam:ൂ",
    {
      script: "malayalam",
      glyph: "ൂ",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down from the hook",
              path: [
                { x: 58, y: 529 },
                { x: 126, y: 527 },
                { x: 184, y: 497 },
                { x: 213, y: 437 },
                { x: 218, y: 370 },
                { x: 210, y: 303 },
                { x: 197, y: 236 },
                { x: 182, y: 171 },
                { x: 166, y: 105 },
              ],
            },
            {
              label: "round the left side and bottom",
              path: [
                { x: 166, y: 105 },
                { x: 105, y: 82 },
                { x: 54, y: 42 },
                { x: 16, y: -11 },
                { x: 10, y: -76 },
                { x: 32, y: -136 },
                { x: 82, y: -179 },
                { x: 145, y: -195 },
                { x: 210, y: -187 },
              ],
            },
            {
              label: "climb its right side",
              path: [
                { x: 210, y: -187 },
                { x: 269, y: -144 },
                { x: 298, y: -78 },
                { x: 290, y: -6 },
                { x: 246, y: 53 },
              ],
            },
            {
              label: "circle the small inner loop",
              path: [
                { x: 246, y: 53 },
                { x: 190, y: 88 },
                { x: 127, y: 88 },
                { x: 69, y: 59 },
                { x: 62, y: 1 },
                { x: 105, y: -49 },
                { x: 168, y: -63 },
                { x: 223, y: -29 },
                { x: 266, y: 21 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ൂ"),
    },
  ],
  // ൃ: Moag's Table III (p. xxi) draws it as two movements to the right of the
  // dash: down the stem (1), then clockwise round the loop at its foot (2).
  [
    "malayalam:ൃ",
    {
      script: "malayalam",
      glyph: "ൃ",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 160, y: 524 },
                { x: 123, y: 466 },
                { x: 101, y: 400 },
                { x: 90, y: 331 },
                { x: 90, y: 261 },
                { x: 104, y: 193 },
                { x: 126, y: 127 },
                { x: 152, y: 62 },
                { x: 175, y: -3 },
                { x: 184, y: -72 },
                { x: 192, y: -141 },
                { x: 176, y: -208 },
              ],
            },
            {
              label: "circle the loop clockwise",
              path: [
                { x: 176, y: -208 },
                { x: 126, y: -256 },
                { x: 62, y: -279 },
                { x: -7, y: -277 },
                { x: -68, y: -247 },
                { x: -100, y: -188 },
                { x: -88, y: -121 },
                { x: -36, y: -76 },
                { x: 32, y: -68 },
                { x: 99, y: -78 },
                { x: 164, y: -100 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ൃ"),
    },
  ],
  // െ: Moag's Table III (p. xxii) draws it as two movements to the left of the
  // dash that stands for the consonant: the small inner loop (1), then over
  // the top and down to the lower right (2).
  [
    "malayalam:െ",
    {
      script: "malayalam",
      glyph: "െ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small inner loop",
              path: [
                { x: 183, y: 284 },
                { x: 252, y: 288 },
                { x: 317, y: 268 },
                { x: 358, y: 213 },
                { x: 363, y: 144 },
                { x: 343, y: 79 },
                { x: 291, y: 34 },
                { x: 222, y: 24 },
                { x: 157, y: 47 },
                { x: 110, y: 97 },
                { x: 88, y: 162 },
                { x: 87, y: 232 },
              ],
            },
            {
              label: "arch over and down to the foot",
              path: [
                { x: 87, y: 232 },
                { x: 91, y: 302 },
                { x: 109, y: 370 },
                { x: 147, y: 430 },
                { x: 199, y: 477 },
                { x: 261, y: 510 },
                { x: 330, y: 526 },
                { x: 400, y: 528 },
                { x: 470, y: 516 },
                { x: 533, y: 486 },
                { x: 584, y: 437 },
                { x: 616, y: 374 },
                { x: 629, y: 305 },
                { x: 630, y: 234 },
                { x: 612, y: 166 },
                { x: 579, y: 104 },
                { x: 531, y: 52 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("െ"),
    },
  ],
  // േ: Moag's Table III (p. xxii) draws it as three movements to the left of
  // the dash: the small top loop (1), a sweep left and round the bottom (2)
  // and a curl up round the lower loop (3).
  [
    "malayalam:േ",
    {
      script: "malayalam",
      glyph: "േ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small top loop",
              path: [
                { x: 327, y: 512 },
                { x: 319, y: 441 },
                { x: 332, y: 371 },
                { x: 392, y: 337 },
                { x: 463, y: 342 },
                { x: 511, y: 393 },
                { x: 512, y: 464 },
                { x: 464, y: 516 },
                { x: 395, y: 532 },
                { x: 327, y: 516 },
              ],
            },
            {
              label: "sweep left and round the bottom",
              path: [
                { x: 327, y: 516 },
                { x: 257, y: 518 },
                { x: 192, y: 490 },
                { x: 140, y: 444 },
                { x: 103, y: 384 },
                { x: 87, y: 315 },
                { x: 87, y: 245 },
                { x: 102, y: 176 },
                { x: 139, y: 116 },
                { x: 190, y: 68 },
                { x: 253, y: 37 },
                { x: 323, y: 36 },
              ],
            },
            {
              label: "curl up round the lower loop",
              path: [
                { x: 323, y: 36 },
                { x: 395, y: 24 },
                { x: 465, y: 42 },
                { x: 506, y: 100 },
                { x: 502, y: 174 },
                { x: 445, y: 217 },
                { x: 372, y: 217 },
                { x: 318, y: 169 },
                { x: 311, y: 96 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("േ"),
    },
  ],
  // Moag's Table IV (p. xxvi) writes ജ: it circles the small top-left loop
  // clockwise (1), arches over the left hump (2), climbs from the foot of the
  // short stem and arches over the right hump (3), runs back left along the
  // band and round the lower left (4), runs right along the lower stroke to the
  // top of the last loop (5) and rounds that loop clockwise (6). Moag's arrow 2
  // stops above the stem and arrow 3 starts at its foot; with no lift between
  // them the pen can only get there down the stem, so movement 2 ends with that
  // descent and movement 3 retraces it.
  [
    "malayalam:ജ",
    {
      script: "malayalam",
      glyph: "ജ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small loop",
              path: [
                { x: 165, y: 452 },
                { x: 202, y: 450 },
                { x: 238, y: 443 },
                { x: 268, y: 425 },
                { x: 287, y: 396 },
                { x: 293, y: 361 },
                { x: 285, y: 326 },
                { x: 264, y: 298 },
                { x: 233, y: 281 },
                { x: 196, y: 275 },
                { x: 159, y: 280 },
                { x: 126, y: 295 },
                { x: 102, y: 322 },
                { x: 89, y: 356 },
                { x: 87, y: 392 },
                { x: 97, y: 427 },
                { x: 117, y: 457 },
              ],
            },
            {
              label: "arch over and down the stem",
              path: [
                { x: 117, y: 457 },
                { x: 140, y: 485 },
                { x: 168, y: 506 },
                { x: 204, y: 518 },
                { x: 242, y: 524 },
                { x: 281, y: 528 },
                { x: 321, y: 527 },
                { x: 360, y: 523 },
                { x: 396, y: 514 },
                { x: 430, y: 498 },
                { x: 461, y: 476 },
                { x: 485, y: 448 },
                { x: 500, y: 415 },
                { x: 506, y: 378 },
                { x: 508, y: 338 },
                { x: 510, y: 298 },
              ],
            },
            {
              label: "climb the stem and arch over the right",
              path: [
                { x: 510, y: 298 },
                { x: 508, y: 337 },
                { x: 508, y: 375 },
                { x: 513, y: 412 },
                { x: 527, y: 444 },
                { x: 549, y: 472 },
                { x: 577, y: 495 },
                { x: 609, y: 512 },
                { x: 644, y: 523 },
                { x: 681, y: 526 },
                { x: 718, y: 524 },
                { x: 754, y: 515 },
                { x: 786, y: 498 },
                { x: 813, y: 474 },
                { x: 833, y: 444 },
                { x: 845, y: 410 },
                { x: 848, y: 373 },
                { x: 844, y: 337 },
                { x: 832, y: 302 },
                { x: 814, y: 271 },
                { x: 791, y: 243 },
                { x: 768, y: 214 },
              ],
            },
            {
              label: "run back left and round the lower left",
              path: [
                { x: 768, y: 214 },
                { x: 730, y: 211 },
                { x: 692, y: 208 },
                { x: 655, y: 203 },
                { x: 619, y: 200 },
                { x: 584, y: 199 },
                { x: 547, y: 201 },
                { x: 509, y: 201 },
                { x: 470, y: 200 },
                { x: 431, y: 199 },
                { x: 392, y: 199 },
                { x: 353, y: 199 },
                { x: 314, y: 199 },
                { x: 275, y: 199 },
                { x: 236, y: 198 },
                { x: 198, y: 195 },
                { x: 161, y: 190 },
                { x: 128, y: 178 },
                { x: 103, y: 156 },
                { x: 91, y: 125 },
                { x: 91, y: 92 },
                { x: 105, y: 62 },
                { x: 131, y: 40 },
                { x: 164, y: 28 },
                { x: 201, y: 22 },
              ],
            },
            {
              label: "run right to the top of the last loop",
              path: [
                { x: 201, y: 22 },
                { x: 239, y: 25 },
                { x: 276, y: 30 },
                { x: 312, y: 38 },
                { x: 347, y: 47 },
                { x: 382, y: 58 },
                { x: 416, y: 71 },
                { x: 449, y: 85 },
                { x: 482, y: 101 },
                { x: 515, y: 116 },
                { x: 547, y: 131 },
                { x: 577, y: 147 },
                { x: 602, y: 168 },
                { x: 626, y: 189 },
                { x: 656, y: 202 },
                { x: 692, y: 208 },
                { x: 730, y: 211 },
                { x: 768, y: 214 },
              ],
            },
            {
              label: "circle down and round the last loop",
              path: [
                { x: 768, y: 214 },
                { x: 794, y: 187 },
                { x: 815, y: 157 },
                { x: 824, y: 123 },
                { x: 822, y: 88 },
                { x: 806, y: 57 },
                { x: 778, y: 36 },
                { x: 744, y: 26 },
                { x: 707, y: 26 },
                { x: 672, y: 35 },
                { x: 643, y: 56 },
                { x: 623, y: 85 },
                { x: 611, y: 119 },
                { x: 600, y: 154 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ജ"),
    },
  ],
  // ൈ: Moag's Table III (p. xxii) draws it as four movements to the left of the
  // dash: the first coil's small loop (1) and arch (2), then the second coil's
  // loop (3) and arch (4). Each coil is െ, and Noto builds the sign from two
  // copies of it 715 units apart, so each run is the cited െ path; the one lift
  // is the gap between the two pieces of ink.
  [
    "malayalam:ൈ",
    {
      script: "malayalam",
      glyph: "ൈ",
      strokes: [
        {
          segments: [
            {
              label: "circle the first small loop",
              path: [
                { x: 183, y: 284 },
                { x: 252, y: 288 },
                { x: 317, y: 268 },
                { x: 358, y: 213 },
                { x: 363, y: 144 },
                { x: 343, y: 79 },
                { x: 291, y: 34 },
                { x: 222, y: 24 },
                { x: 157, y: 47 },
                { x: 110, y: 97 },
                { x: 88, y: 162 },
                { x: 87, y: 232 },
              ],
            },
            {
              label: "arch over and down to the foot",
              path: [
                { x: 87, y: 232 },
                { x: 91, y: 302 },
                { x: 109, y: 370 },
                { x: 147, y: 430 },
                { x: 199, y: 477 },
                { x: 261, y: 510 },
                { x: 330, y: 526 },
                { x: 400, y: 528 },
                { x: 470, y: 516 },
                { x: 533, y: 486 },
                { x: 584, y: 437 },
                { x: 616, y: 374 },
                { x: 629, y: 305 },
                { x: 630, y: 234 },
                { x: 612, y: 166 },
                { x: 579, y: 104 },
                { x: 531, y: 52 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then circle the second small loop",
              path: [
                { x: 898, y: 284 },
                { x: 967, y: 288 },
                { x: 1032, y: 268 },
                { x: 1073, y: 213 },
                { x: 1078, y: 144 },
                { x: 1058, y: 79 },
                { x: 1006, y: 34 },
                { x: 937, y: 24 },
                { x: 872, y: 47 },
                { x: 825, y: 97 },
                { x: 803, y: 162 },
                { x: 802, y: 232 },
              ],
            },
            {
              label: "arch over and down to its foot",
              path: [
                { x: 802, y: 232 },
                { x: 806, y: 302 },
                { x: 824, y: 370 },
                { x: 862, y: 430 },
                { x: 914, y: 477 },
                { x: 976, y: 510 },
                { x: 1045, y: 526 },
                { x: 1115, y: 528 },
                { x: 1185, y: 516 },
                { x: 1248, y: 486 },
                { x: 1299, y: 437 },
                { x: 1331, y: 374 },
                { x: 1344, y: 305 },
                { x: 1345, y: 234 },
                { x: 1327, y: 166 },
                { x: 1294, y: 104 },
                { x: 1246, y: 52 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ൈ"),
    },
  ],
  // -------------------------------------------------------------------------
  // Recorded by hand: the candrakkala, ഠ and the digits ൧-൯, cited to Jayasree.
  //
  // Jayasree (github.com/sachn1/jayasree, Sachin Nandakumar) is a Malayalam
  // handwriting animator built on about 300 hand-recorded centre lines: one
  // recorder traced each character over the Manjari typeface, and every
  // pen-down gesture became one stroke. So unlike a textbook's numbered
  // arrows, a recording shows the LIFTS directly, as well as the start, the
  // order and the direction. Its stroke data is CC BY 4.0 ("Jayasree" by
  // Sachin Nandakumar), so each record credits it by name and licence.
  //
  // What is taken: the number of strokes, where each starts, the order of its
  // movements and which way it turns. What is not: the recorded coordinates.
  // Manjari is rounder and wider than Noto Sans Malayalam, so every path below
  // is fitted to the bundled Noto outline (on its ink, joined, covering all of
  // it, at the default tolerances), and the captions are this package's own.
  // Every glyph here is ONE stroke: the recorder never lifted inside them.
  //
  // The two-part signs ൊ and ോ come last: two recorded strokes each, the left
  // sign and then ാ. Noto's standalone glyphs print a placeholder dot between
  // the parts where the consonant would sit; that dot is not written, so the
  // paths leave it alone and the coverage check skips exactly that contour for
  // exactly these two glyphs (NOTO_PLACEHOLDER_CONTOURS, in the stroke-honesty
  // tests). ൦ is recorded too, but no writing lesson draws it alone, so it
  // waits.
  // -------------------------------------------------------------------------
  // ് (the candrakkala): Jayasree records it as one stroke that starts at the
  // left tip, dips round the bottom of the cup and rises to the right tip. Noto
  // prints it as a zero-advance mark above and to the left of the origin, so the
  // fitted path has negative x.
  [
    "malayalam:്",
    {
      script: "malayalam",
      glyph: "്",
      strokes: [
        {
          segments: [
            {
              label: "start at the left tip and dip down",
              path: [
                { x: -254, y: 793 },
                { x: -252, y: 765 },
                { x: -249, y: 736 },
                { x: -241, y: 708 },
                { x: -227, y: 682 },
                { x: -208, y: 660 },
                { x: -184, y: 644 },
                { x: -157, y: 634 },
                { x: -128, y: 630 },
              ],
            },
            {
              label: "round the bottom and rise to the right",
              path: [
                { x: -128, y: 630 },
                { x: -98, y: 630 },
                { x: -69, y: 634 },
                { x: -41, y: 644 },
                { x: -16, y: 658 },
                { x: 4, y: 679 },
                { x: 17, y: 704 },
                { x: 26, y: 732 },
                { x: 34, y: 761 },
                { x: 45, y: 791 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("്"),
    },
  ],
  // ഠ: Jayasree records the ring as one stroke that starts at the top and runs
  // anticlockwise (down the left side first), closing back at the top. Thooval
  // and grahyam turn the same way; Moag runs it clockwise, and the record names
  // that disagreement. The path ends where it began.
  [
    "malayalam:ഠ",
    {
      script: "malayalam",
      glyph: "ഠ",
      strokes: [
        {
          segments: [
            {
              label: "circle anticlockwise down the left",
              path: [
                { x: 334, y: 527 },
                { x: 304, y: 526 },
                { x: 274, y: 523 },
                { x: 246, y: 516 },
                { x: 218, y: 506 },
                { x: 192, y: 492 },
                { x: 168, y: 475 },
                { x: 147, y: 455 },
                { x: 129, y: 431 },
                { x: 113, y: 406 },
                { x: 102, y: 379 },
                { x: 93, y: 350 },
                { x: 86, y: 321 },
                { x: 84, y: 291 },
                { x: 85, y: 261 },
                { x: 89, y: 231 },
                { x: 95, y: 202 },
                { x: 104, y: 174 },
                { x: 116, y: 147 },
                { x: 130, y: 121 },
                { x: 149, y: 98 },
                { x: 171, y: 78 },
                { x: 195, y: 61 },
                { x: 221, y: 48 },
                { x: 249, y: 38 },
                { x: 278, y: 32 },
                { x: 308, y: 30 },
              ],
            },
            {
              label: "climb the right side back to the top",
              path: [
                { x: 308, y: 30 },
                { x: 338, y: 29 },
                { x: 367, y: 31 },
                { x: 397, y: 35 },
                { x: 425, y: 43 },
                { x: 452, y: 55 },
                { x: 477, y: 71 },
                { x: 499, y: 90 },
                { x: 518, y: 113 },
                { x: 535, y: 137 },
                { x: 548, y: 164 },
                { x: 559, y: 191 },
                { x: 566, y: 220 },
                { x: 570, y: 249 },
                { x: 571, y: 279 },
                { x: 570, y: 309 },
                { x: 565, y: 339 },
                { x: 558, y: 368 },
                { x: 547, y: 396 },
                { x: 533, y: 422 },
                { x: 516, y: 446 },
                { x: 496, y: 467 },
                { x: 473, y: 486 },
                { x: 448, y: 502 },
                { x: 421, y: 514 },
                { x: 392, y: 521 },
                { x: 363, y: 525 },
                { x: 333, y: 527 },
              ],
            },
          ],
        },
      ],
      source: malayalamAlphabetSource("ഠ"),
    },
  ],
  // ൧: one recorded stroke. Up the left stem from its foot, clockwise over the
  // arch and down the right side, back left along the baseline past the stem,
  // then a curl below the line that ends heading right.
  [
    "malayalam:൧",
    {
      script: "malayalam",
      glyph: "൧",
      strokes: [
        {
          segments: [
            {
              label: "climb the left stem",
              path: [
                { x: 360, y: 50 },
                { x: 376, y: 68 },
                { x: 382, y: 93 },
                { x: 382, y: 122 },
                { x: 382, y: 152 },
                { x: 382, y: 182 },
                { x: 382, y: 212 },
                { x: 382, y: 242 },
                { x: 382, y: 272 },
                { x: 383, y: 302 },
                { x: 385, y: 332 },
                { x: 389, y: 362 },
                { x: 395, y: 392 },
              ],
            },
            {
              label: "arch clockwise over and down the right",
              path: [
                { x: 395, y: 392 },
                { x: 404, y: 420 },
                { x: 417, y: 446 },
                { x: 435, y: 469 },
                { x: 457, y: 489 },
                { x: 482, y: 505 },
                { x: 509, y: 516 },
                { x: 538, y: 522 },
                { x: 568, y: 525 },
                { x: 598, y: 526 },
                { x: 627, y: 523 },
                { x: 657, y: 517 },
                { x: 684, y: 506 },
                { x: 710, y: 492 },
                { x: 732, y: 473 },
                { x: 750, y: 450 },
                { x: 765, y: 425 },
                { x: 775, y: 397 },
                { x: 781, y: 368 },
                { x: 785, y: 338 },
                { x: 786, y: 308 },
                { x: 787, y: 278 },
                { x: 789, y: 248 },
                { x: 790, y: 218 },
                { x: 791, y: 188 },
                { x: 792, y: 158 },
                { x: 793, y: 128 },
                { x: 791, y: 99 },
                { x: 783, y: 72 },
                { x: 767, y: 52 },
              ],
            },
            {
              label: "run left along the baseline",
              path: [
                { x: 767, y: 52 },
                { x: 743, y: 41 },
                { x: 714, y: 37 },
                { x: 684, y: 36 },
                { x: 654, y: 36 },
                { x: 624, y: 36 },
                { x: 594, y: 36 },
                { x: 564, y: 36 },
                { x: 534, y: 36 },
                { x: 504, y: 36 },
                { x: 474, y: 36 },
                { x: 444, y: 38 },
                { x: 414, y: 40 },
                { x: 385, y: 42 },
                { x: 356, y: 41 },
                { x: 327, y: 38 },
                { x: 297, y: 36 },
                { x: 267, y: 36 },
                { x: 237, y: 36 },
                { x: 207, y: 36 },
                { x: 177, y: 34 },
              ],
            },
            {
              label: "curl down and back to the right",
              path: [
                { x: 177, y: 34 },
                { x: 148, y: 30 },
                { x: 120, y: 21 },
                { x: 95, y: 7 },
                { x: 76, y: -14 },
                { x: 64, y: -40 },
                { x: 61, y: -68 },
                { x: 68, y: -95 },
                { x: 83, y: -119 },
                { x: 104, y: -137 },
                { x: 131, y: -149 },
                { x: 159, y: -156 },
                { x: 189, y: -160 },
                { x: 219, y: -162 },
                { x: 249, y: -163 },
                { x: 279, y: -164 },
                { x: 309, y: -167 },
                { x: 338, y: -173 },
                { x: 367, y: -181 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൧"),
    },
  ],
  // ൨: one recorded stroke. Up the left side from its lower tip, clockwise over
  // the top and down to the foot of the right side, then right along the
  // baseline.
  [
    "malayalam:൨",
    {
      script: "malayalam",
      glyph: "൨",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side",
              path: [
                { x: 217, y: 47 },
                { x: 192, y: 54 },
                { x: 169, y: 68 },
                { x: 149, y: 89 },
                { x: 131, y: 112 },
                { x: 116, y: 138 },
                { x: 104, y: 165 },
                { x: 94, y: 193 },
                { x: 88, y: 222 },
                { x: 85, y: 252 },
                { x: 84, y: 282 },
                { x: 87, y: 311 },
                { x: 92, y: 341 },
              ],
            },
            {
              label: "arch clockwise over and down to the foot",
              path: [
                { x: 92, y: 341 },
                { x: 100, y: 370 },
                { x: 112, y: 397 },
                { x: 127, y: 423 },
                { x: 144, y: 446 },
                { x: 165, y: 467 },
                { x: 188, y: 485 },
                { x: 214, y: 500 },
                { x: 241, y: 512 },
                { x: 270, y: 520 },
                { x: 299, y: 524 },
                { x: 329, y: 526 },
                { x: 359, y: 526 },
                { x: 389, y: 523 },
                { x: 419, y: 518 },
                { x: 447, y: 509 },
                { x: 473, y: 496 },
                { x: 498, y: 479 },
                { x: 519, y: 460 },
                { x: 539, y: 438 },
                { x: 555, y: 413 },
                { x: 568, y: 386 },
                { x: 577, y: 358 },
                { x: 583, y: 329 },
                { x: 585, y: 299 },
                { x: 586, y: 269 },
                { x: 585, y: 239 },
                { x: 581, y: 210 },
                { x: 573, y: 182 },
                { x: 562, y: 154 },
                { x: 548, y: 127 },
                { x: 533, y: 101 },
                { x: 521, y: 77 },
                { x: 518, y: 57 },
              ],
            },
            {
              label: "run right along the baseline",
              path: [
                { x: 518, y: 57 },
                { x: 531, y: 45 },
                { x: 556, y: 39 },
                { x: 584, y: 37 },
                { x: 614, y: 36 },
                { x: 644, y: 36 },
                { x: 674, y: 36 },
                { x: 704, y: 36 },
                { x: 734, y: 36 },
                { x: 764, y: 35 },
                { x: 791, y: 31 },
                { x: 816, y: 22 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൨"),
    },
  ],
  // ൩: one recorded stroke. Up the left side, clockwise over the first arch and
  // down the stem to its foot, back up the stem (retracing it: the next arch
  // starts at its top), clockwise over the second arch, then right along the
  // baseline.
  [
    "malayalam:൩",
    {
      script: "malayalam",
      glyph: "൩",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side",
              path: [
                { x: 225, y: 51 },
                { x: 198, y: 59 },
                { x: 173, y: 72 },
                { x: 152, y: 92 },
                { x: 134, y: 115 },
                { x: 119, y: 141 },
                { x: 107, y: 168 },
                { x: 99, y: 196 },
                { x: 93, y: 225 },
                { x: 88, y: 255 },
                { x: 86, y: 285 },
                { x: 85, y: 315 },
                { x: 88, y: 345 },
              ],
            },
            {
              label: "arch clockwise and run down the stem",
              path: [
                { x: 88, y: 345 },
                { x: 95, y: 373 },
                { x: 106, y: 401 },
                { x: 119, y: 428 },
                { x: 135, y: 452 },
                { x: 154, y: 474 },
                { x: 177, y: 494 },
                { x: 202, y: 509 },
                { x: 229, y: 520 },
                { x: 258, y: 525 },
                { x: 287, y: 526 },
                { x: 317, y: 523 },
                { x: 346, y: 516 },
                { x: 372, y: 504 },
                { x: 396, y: 486 },
                { x: 417, y: 465 },
                { x: 435, y: 442 },
                { x: 448, y: 416 },
                { x: 454, y: 389 },
                { x: 456, y: 359 },
                { x: 456, y: 329 },
                { x: 456, y: 299 },
                { x: 456, y: 269 },
                { x: 456, y: 239 },
                { x: 456, y: 209 },
                { x: 456, y: 179 },
                { x: 456, y: 149 },
                { x: 456, y: 119 },
                { x: 456, y: 89 },
                { x: 456, y: 59 },
              ],
            },
            {
              label: "climb back up the stem",
              path: [
                { x: 456, y: 59 },
                { x: 456, y: 71 },
                { x: 456, y: 92 },
                { x: 456, y: 120 },
                { x: 456, y: 150 },
                { x: 456, y: 180 },
                { x: 456, y: 210 },
                { x: 456, y: 240 },
                { x: 456, y: 270 },
                { x: 456, y: 300 },
                { x: 456, y: 330 },
                { x: 456, y: 360 },
                { x: 458, y: 390 },
              ],
            },
            {
              label: "arch clockwise over and down",
              path: [
                { x: 458, y: 390 },
                { x: 465, y: 417 },
                { x: 478, y: 443 },
                { x: 496, y: 466 },
                { x: 517, y: 487 },
                { x: 540, y: 505 },
                { x: 566, y: 517 },
                { x: 595, y: 523 },
                { x: 624, y: 526 },
                { x: 654, y: 525 },
                { x: 684, y: 520 },
                { x: 711, y: 510 },
                { x: 737, y: 495 },
                { x: 759, y: 476 },
                { x: 778, y: 454 },
                { x: 794, y: 429 },
                { x: 807, y: 402 },
                { x: 817, y: 374 },
                { x: 824, y: 345 },
                { x: 828, y: 315 },
                { x: 829, y: 285 },
                { x: 827, y: 255 },
                { x: 823, y: 226 },
                { x: 815, y: 197 },
                { x: 806, y: 169 },
                { x: 793, y: 141 },
                { x: 779, y: 115 },
                { x: 767, y: 89 },
                { x: 764, y: 66 },
                { x: 775, y: 49 },
              ],
            },
            {
              label: "run right along the baseline",
              path: [
                { x: 775, y: 49 },
                { x: 799, y: 40 },
                { x: 828, y: 37 },
                { x: 858, y: 37 },
                { x: 888, y: 37 },
                { x: 918, y: 37 },
                { x: 948, y: 38 },
                { x: 978, y: 37 },
                { x: 1005, y: 34 },
                { x: 1027, y: 28 },
                { x: 1041, y: 19 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൩"),
    },
  ],
  // ൪: one recorded stroke. Up the left side and over the top, down through the
  // crossing and round the bottom of the bowl, back up through the crossing,
  // and on to the flourish at the top right. Manjari, the font Jayasree was
  // drawn over, ends this stroke with a straight rise where Noto curls to the
  // upper right; the order and turning are the same, and the path follows Noto.
  [
    "malayalam:൪",
    {
      script: "malayalam",
      glyph: "൪",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side and over the top",
              path: [
                { x: 196, y: 15 },
                { x: 185, y: 41 },
                { x: 169, y: 66 },
                { x: 151, y: 89 },
                { x: 133, y: 112 },
                { x: 118, y: 138 },
                { x: 105, y: 165 },
                { x: 96, y: 193 },
                { x: 91, y: 222 },
                { x: 89, y: 252 },
                { x: 88, y: 282 },
                { x: 90, y: 312 },
                { x: 95, y: 341 },
                { x: 103, y: 370 },
                { x: 114, y: 397 },
                { x: 129, y: 422 },
                { x: 147, y: 446 },
                { x: 169, y: 466 },
                { x: 193, y: 484 },
                { x: 218, y: 498 },
                { x: 246, y: 510 },
                { x: 274, y: 518 },
                { x: 303, y: 523 },
              ],
            },
            {
              label: "cross down the right and round the bottom",
              path: [
                { x: 303, y: 523 },
                { x: 333, y: 526 },
                { x: 363, y: 527 },
                { x: 393, y: 525 },
                { x: 421, y: 521 },
                { x: 444, y: 512 },
                { x: 460, y: 501 },
                { x: 472, y: 490 },
                { x: 484, y: 480 },
                { x: 496, y: 470 },
                { x: 508, y: 460 },
                { x: 520, y: 450 },
                { x: 532, y: 440 },
                { x: 544, y: 430 },
                { x: 556, y: 420 },
                { x: 569, y: 411 },
                { x: 585, y: 404 },
                { x: 604, y: 394 },
                { x: 621, y: 377 },
                { x: 635, y: 352 },
                { x: 645, y: 325 },
                { x: 653, y: 296 },
                { x: 657, y: 266 },
                { x: 658, y: 236 },
                { x: 658, y: 206 },
                { x: 656, y: 177 },
                { x: 649, y: 148 },
                { x: 639, y: 121 },
                { x: 625, y: 95 },
                { x: 607, y: 73 },
                { x: 584, y: 54 },
                { x: 559, y: 40 },
                { x: 531, y: 31 },
                { x: 501, y: 27 },
                { x: 471, y: 28 },
              ],
            },
            {
              label: "climb back up through the crossing",
              path: [
                { x: 471, y: 28 },
                { x: 442, y: 33 },
                { x: 415, y: 43 },
                { x: 390, y: 58 },
                { x: 369, y: 78 },
                { x: 353, y: 103 },
                { x: 342, y: 130 },
                { x: 335, y: 158 },
                { x: 333, y: 188 },
                { x: 336, y: 218 },
                { x: 343, y: 247 },
                { x: 354, y: 275 },
                { x: 369, y: 301 },
                { x: 387, y: 324 },
                { x: 406, y: 346 },
                { x: 425, y: 366 },
                { x: 438, y: 384 },
                { x: 445, y: 398 },
                { x: 453, y: 408 },
                { x: 465, y: 416 },
                { x: 477, y: 424 },
                { x: 489, y: 432 },
                { x: 502, y: 440 },
                { x: 514, y: 448 },
                { x: 526, y: 457 },
                { x: 538, y: 465 },
                { x: 551, y: 473 },
                { x: 563, y: 481 },
                { x: 575, y: 489 },
                { x: 588, y: 495 },
                { x: 604, y: 497 },
              ],
            },
            {
              label: "curl up to the top right",
              path: [
                { x: 604, y: 497 },
                { x: 623, y: 497 },
                { x: 647, y: 504 },
                { x: 673, y: 518 },
                { x: 696, y: 536 },
                { x: 717, y: 556 },
                { x: 732, y: 581 },
                { x: 742, y: 608 },
                { x: 745, y: 637 },
                { x: 747, y: 666 },
                { x: 749, y: 691 },
                { x: 753, y: 709 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൪"),
    },
  ],
  // ൫: one recorded stroke. Up the small inner curve from its lower tip, round
  // its top into the notch in the middle, back out and round the right bowl,
  // along the bottom and up the outer left side, then over the top and down
  // the right to the outer tip.
  [
    "malayalam:൫",
    {
      script: "malayalam",
      glyph: "൫",
      strokes: [
        {
          segments: [
            {
              label: "climb the small inner curve",
              path: [
                { x: 352, y: 25 },
                { x: 346, y: 54 },
                { x: 337, y: 81 },
                { x: 325, y: 108 },
                { x: 314, y: 136 },
                { x: 305, y: 164 },
                { x: 298, y: 193 },
                { x: 294, y: 223 },
                { x: 292, y: 253 },
                { x: 293, y: 283 },
                { x: 295, y: 313 },
                { x: 300, y: 342 },
                { x: 308, y: 371 },
                { x: 319, y: 398 },
                { x: 334, y: 423 },
                { x: 353, y: 446 },
                { x: 375, y: 466 },
                { x: 400, y: 481 },
                { x: 427, y: 492 },
                { x: 455, y: 500 },
                { x: 485, y: 504 },
              ],
            },
            {
              label: "round it into the notch",
              path: [
                { x: 485, y: 504 },
                { x: 515, y: 504 },
                { x: 544, y: 500 },
                { x: 573, y: 493 },
                { x: 599, y: 480 },
                { x: 623, y: 463 },
                { x: 641, y: 440 },
                { x: 652, y: 414 },
                { x: 657, y: 386 },
                { x: 654, y: 358 },
                { x: 644, y: 331 },
                { x: 627, y: 307 },
                { x: 605, y: 288 },
                { x: 581, y: 271 },
                { x: 556, y: 258 },
                { x: 529, y: 252 },
                { x: 506, y: 250 },
                { x: 489, y: 250 },
              ],
            },
            {
              label: "turn back and round the right bowl",
              path: [
                { x: 489, y: 250 },
                { x: 498, y: 249 },
                { x: 517, y: 247 },
                { x: 543, y: 241 },
                { x: 569, y: 228 },
                { x: 593, y: 211 },
                { x: 615, y: 191 },
                { x: 635, y: 169 },
                { x: 651, y: 144 },
                { x: 662, y: 118 },
                { x: 668, y: 89 },
                { x: 670, y: 59 },
                { x: 669, y: 30 },
                { x: 663, y: 1 },
                { x: 652, y: -26 },
                { x: 637, y: -52 },
                { x: 619, y: -75 },
                { x: 597, y: -93 },
                { x: 572, y: -109 },
                { x: 545, y: -121 },
                { x: 516, y: -129 },
                { x: 487, y: -134 },
                { x: 457, y: -136 },
              ],
            },
            {
              label: "round the bottom and up the left",
              path: [
                { x: 457, y: -136 },
                { x: 427, y: -137 },
                { x: 397, y: -136 },
                { x: 367, y: -134 },
                { x: 338, y: -129 },
                { x: 309, y: -121 },
                { x: 282, y: -110 },
                { x: 255, y: -97 },
                { x: 231, y: -81 },
                { x: 207, y: -62 },
                { x: 186, y: -41 },
                { x: 167, y: -18 },
                { x: 151, y: 6 },
                { x: 136, y: 32 },
                { x: 124, y: 59 },
                { x: 114, y: 87 },
                { x: 106, y: 116 },
                { x: 99, y: 146 },
                { x: 95, y: 175 },
                { x: 92, y: 205 },
                { x: 90, y: 235 },
                { x: 90, y: 265 },
                { x: 91, y: 295 },
                { x: 94, y: 325 },
                { x: 98, y: 354 },
                { x: 104, y: 384 },
                { x: 112, y: 413 },
                { x: 121, y: 441 },
                { x: 133, y: 469 },
                { x: 146, y: 495 },
                { x: 162, y: 520 },
                { x: 180, y: 544 },
                { x: 200, y: 566 },
                { x: 221, y: 587 },
              ],
            },
            {
              label: "arch over and down the right",
              path: [
                { x: 221, y: 587 },
                { x: 244, y: 606 },
                { x: 269, y: 623 },
                { x: 294, y: 638 },
                { x: 321, y: 651 },
                { x: 349, y: 662 },
                { x: 378, y: 670 },
                { x: 407, y: 677 },
                { x: 437, y: 681 },
                { x: 466, y: 683 },
                { x: 496, y: 684 },
                { x: 526, y: 683 },
                { x: 556, y: 681 },
                { x: 586, y: 677 },
                { x: 615, y: 671 },
                { x: 644, y: 662 },
                { x: 672, y: 651 },
                { x: 699, y: 638 },
                { x: 724, y: 622 },
                { x: 748, y: 605 },
                { x: 770, y: 585 },
                { x: 790, y: 563 },
                { x: 808, y: 540 },
                { x: 824, y: 515 },
                { x: 838, y: 488 },
                { x: 850, y: 460 },
                { x: 860, y: 432 },
                { x: 867, y: 403 },
                { x: 873, y: 374 },
                { x: 877, y: 344 },
                { x: 880, y: 314 },
                { x: 881, y: 284 },
                { x: 880, y: 254 },
                { x: 879, y: 224 },
                { x: 876, y: 195 },
                { x: 872, y: 165 },
                { x: 867, y: 136 },
                { x: 859, y: 107 },
                { x: 850, y: 79 },
                { x: 843, y: 51 },
                { x: 842, y: 21 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൫"),
    },
  ],
  // ൬: one recorded stroke. Up the left side, over the first arch and down the
  // first stem to its foot, back up it, over the second arch and down the
  // second stem, back up it, then over the third arch, down the right side and
  // round below the line to the left. Each climb retraces its stem.
  [
    "malayalam:൬",
    {
      script: "malayalam",
      glyph: "൬",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side",
              path: [
                { x: 235, y: 49 },
                { x: 207, y: 55 },
                { x: 181, y: 66 },
                { x: 159, y: 84 },
                { x: 141, y: 107 },
                { x: 126, y: 133 },
                { x: 112, y: 159 },
                { x: 101, y: 187 },
                { x: 94, y: 215 },
                { x: 89, y: 245 },
                { x: 87, y: 275 },
                { x: 87, y: 305 },
                { x: 89, y: 335 },
              ],
            },
            {
              label: "arch over and down the first stem",
              path: [
                { x: 89, y: 335 },
                { x: 94, y: 364 },
                { x: 102, y: 393 },
                { x: 113, y: 420 },
                { x: 127, y: 446 },
                { x: 145, y: 469 },
                { x: 167, y: 489 },
                { x: 192, y: 505 },
                { x: 219, y: 517 },
                { x: 247, y: 524 },
                { x: 277, y: 525 },
                { x: 307, y: 522 },
                { x: 335, y: 515 },
                { x: 361, y: 502 },
                { x: 384, y: 484 },
                { x: 404, y: 463 },
                { x: 422, y: 441 },
                { x: 435, y: 416 },
                { x: 439, y: 389 },
                { x: 440, y: 359 },
                { x: 440, y: 329 },
                { x: 440, y: 299 },
                { x: 440, y: 269 },
                { x: 440, y: 239 },
                { x: 440, y: 209 },
                { x: 440, y: 179 },
                { x: 440, y: 149 },
                { x: 440, y: 119 },
                { x: 440, y: 89 },
                { x: 440, y: 59 },
                { x: 440, y: 30 },
              ],
            },
            {
              label: "climb back up the stem",
              path: [
                { x: 440, y: 30 },
                { x: 440, y: 60 },
                { x: 440, y: 90 },
                { x: 440, y: 120 },
                { x: 440, y: 150 },
                { x: 440, y: 180 },
                { x: 440, y: 210 },
                { x: 440, y: 240 },
                { x: 440, y: 270 },
                { x: 441, y: 300 },
                { x: 441, y: 330 },
                { x: 441, y: 360 },
                { x: 443, y: 390 },
                { x: 449, y: 418 },
              ],
            },
            {
              label: "arch over and down the second stem",
              path: [
                { x: 449, y: 418 },
                { x: 462, y: 444 },
                { x: 480, y: 467 },
                { x: 501, y: 487 },
                { x: 526, y: 503 },
                { x: 552, y: 516 },
                { x: 580, y: 523 },
                { x: 609, y: 525 },
                { x: 639, y: 523 },
                { x: 667, y: 515 },
                { x: 693, y: 502 },
                { x: 717, y: 484 },
                { x: 738, y: 464 },
                { x: 755, y: 443 },
                { x: 767, y: 421 },
                { x: 770, y: 395 },
                { x: 770, y: 366 },
                { x: 770, y: 336 },
                { x: 770, y: 306 },
                { x: 770, y: 276 },
                { x: 770, y: 246 },
                { x: 770, y: 216 },
                { x: 770, y: 186 },
                { x: 770, y: 156 },
                { x: 770, y: 126 },
                { x: 770, y: 96 },
                { x: 770, y: 64 },
                { x: 770, y: 30 },
              ],
            },
            {
              label: "climb back up it",
              path: [
                { x: 770, y: 30 },
                { x: 770, y: 60 },
                { x: 770, y: 90 },
                { x: 770, y: 120 },
                { x: 770, y: 150 },
                { x: 770, y: 180 },
                { x: 770, y: 210 },
                { x: 770, y: 240 },
                { x: 770, y: 270 },
                { x: 770, y: 300 },
                { x: 771, y: 330 },
                { x: 772, y: 360 },
                { x: 774, y: 390 },
                { x: 780, y: 417 },
              ],
            },
            {
              label: "arch over and curl down to the left",
              path: [
                { x: 780, y: 417 },
                { x: 793, y: 443 },
                { x: 811, y: 467 },
                { x: 832, y: 488 },
                { x: 856, y: 505 },
                { x: 882, y: 517 },
                { x: 911, y: 523 },
                { x: 940, y: 525 },
                { x: 970, y: 523 },
                { x: 999, y: 517 },
                { x: 1025, y: 506 },
                { x: 1049, y: 489 },
                { x: 1070, y: 468 },
                { x: 1087, y: 443 },
                { x: 1100, y: 417 },
                { x: 1111, y: 389 },
                { x: 1119, y: 361 },
                { x: 1126, y: 331 },
                { x: 1130, y: 302 },
                { x: 1132, y: 272 },
                { x: 1134, y: 242 },
                { x: 1134, y: 212 },
                { x: 1132, y: 182 },
                { x: 1130, y: 152 },
                { x: 1126, y: 122 },
                { x: 1121, y: 93 },
                { x: 1115, y: 64 },
                { x: 1108, y: 35 },
                { x: 1097, y: 7 },
                { x: 1085, y: -20 },
                { x: 1070, y: -45 },
                { x: 1052, y: -69 },
                { x: 1032, y: -90 },
                { x: 1009, y: -109 },
                { x: 984, y: -125 },
                { x: 957, y: -138 },
                { x: 929, y: -147 },
                { x: 900, y: -154 },
                { x: 870, y: -158 },
                { x: 840, y: -160 },
                { x: 810, y: -160 },
                { x: 780, y: -162 },
                { x: 753, y: -166 },
                { x: 732, y: -173 },
                { x: 717, y: -182 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൬"),
    },
  ],
  // ൭: one recorded stroke. Jayasree starts at the inner end of the small loop
  // and curls clockwise round it; in Noto that end joins the outer stroke on
  // the left, so the path starts at the join. Then up the left side, over the
  // top, down the right and round below the line to the left.
  [
    "malayalam:൭",
    {
      script: "malayalam",
      glyph: "൭",
      strokes: [
        {
          segments: [
            {
              label: "curl clockwise round the small loop",
              path: [
                { x: 101, y: 289 },
                { x: 120, y: 279 },
                { x: 147, y: 279 },
                { x: 177, y: 283 },
                { x: 206, y: 288 },
                { x: 235, y: 291 },
                { x: 265, y: 289 },
                { x: 292, y: 280 },
                { x: 317, y: 266 },
                { x: 337, y: 245 },
                { x: 352, y: 220 },
                { x: 361, y: 192 },
                { x: 364, y: 163 },
                { x: 361, y: 134 },
                { x: 352, y: 106 },
                { x: 339, y: 80 },
                { x: 321, y: 58 },
                { x: 297, y: 41 },
                { x: 270, y: 31 },
                { x: 241, y: 28 },
                { x: 212, y: 30 },
                { x: 184, y: 38 },
                { x: 158, y: 51 },
                { x: 135, y: 70 },
                { x: 117, y: 93 },
                { x: 104, y: 119 },
              ],
            },
            {
              label: "climb the left side to the top",
              path: [
                { x: 104, y: 119 },
                { x: 93, y: 146 },
                { x: 87, y: 175 },
                { x: 85, y: 205 },
                { x: 85, y: 235 },
                { x: 86, y: 265 },
                { x: 88, y: 295 },
                { x: 93, y: 324 },
                { x: 103, y: 353 },
                { x: 115, y: 380 },
                { x: 130, y: 405 },
                { x: 148, y: 429 },
                { x: 169, y: 450 },
                { x: 192, y: 470 },
                { x: 216, y: 486 },
                { x: 243, y: 500 },
                { x: 270, y: 511 },
                { x: 299, y: 519 },
                { x: 328, y: 524 },
                { x: 358, y: 526 },
              ],
            },
            {
              label: "sweep down the right and curl left below",
              path: [
                { x: 358, y: 526 },
                { x: 388, y: 526 },
                { x: 418, y: 525 },
                { x: 448, y: 521 },
                { x: 476, y: 513 },
                { x: 504, y: 502 },
                { x: 530, y: 488 },
                { x: 554, y: 470 },
                { x: 575, y: 450 },
                { x: 594, y: 428 },
                { x: 610, y: 403 },
                { x: 623, y: 376 },
                { x: 633, y: 348 },
                { x: 642, y: 319 },
                { x: 648, y: 290 },
                { x: 653, y: 260 },
                { x: 655, y: 230 },
                { x: 656, y: 200 },
                { x: 655, y: 170 },
                { x: 652, y: 141 },
                { x: 647, y: 111 },
                { x: 641, y: 82 },
                { x: 633, y: 53 },
                { x: 623, y: 25 },
                { x: 611, y: -2 },
                { x: 597, y: -28 },
                { x: 580, y: -53 },
                { x: 561, y: -75 },
                { x: 539, y: -95 },
                { x: 514, y: -112 },
                { x: 489, y: -127 },
                { x: 462, y: -139 },
                { x: 433, y: -148 },
                { x: 404, y: -154 },
                { x: 374, y: -158 },
                { x: 344, y: -160 },
                { x: 314, y: -160 },
                { x: 284, y: -160 },
                { x: 256, y: -159 },
                { x: 231, y: -151 },
                { x: 209, y: -136 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൭"),
    },
  ],
  // ൮: one recorded stroke. Up the left side, clockwise over the top and down
  // to the foot, right along the baseline, up the right stem to its top, then
  // back down it (retracing) and round below the line to the left.
  [
    "malayalam:൮",
    {
      script: "malayalam",
      glyph: "൮",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side",
              path: [
                { x: 236, y: 52 },
                { x: 208, y: 54 },
                { x: 181, y: 63 },
                { x: 158, y: 79 },
                { x: 140, y: 102 },
                { x: 124, y: 127 },
                { x: 111, y: 154 },
                { x: 101, y: 181 },
                { x: 94, y: 210 },
                { x: 90, y: 239 },
                { x: 88, y: 269 },
                { x: 89, y: 299 },
                { x: 92, y: 329 },
              ],
            },
            {
              label: "arch clockwise over and down to the foot",
              path: [
                { x: 92, y: 329 },
                { x: 99, y: 359 },
                { x: 109, y: 387 },
                { x: 122, y: 413 },
                { x: 139, y: 438 },
                { x: 158, y: 460 },
                { x: 180, y: 479 },
                { x: 205, y: 495 },
                { x: 232, y: 508 },
                { x: 260, y: 518 },
                { x: 289, y: 523 },
                { x: 318, y: 526 },
                { x: 348, y: 526 },
                { x: 378, y: 524 },
                { x: 408, y: 520 },
                { x: 437, y: 512 },
                { x: 464, y: 501 },
                { x: 489, y: 486 },
                { x: 512, y: 468 },
                { x: 533, y: 447 },
                { x: 550, y: 423 },
                { x: 564, y: 397 },
                { x: 575, y: 369 },
                { x: 582, y: 340 },
                { x: 585, y: 310 },
                { x: 587, y: 280 },
                { x: 586, y: 251 },
                { x: 583, y: 221 },
                { x: 576, y: 192 },
                { x: 566, y: 164 },
                { x: 553, y: 137 },
                { x: 538, y: 111 },
                { x: 525, y: 85 },
                { x: 522, y: 62 },
              ],
            },
            {
              label: "run right along the baseline",
              path: [
                { x: 522, y: 62 },
                { x: 535, y: 46 },
                { x: 561, y: 39 },
                { x: 591, y: 37 },
                { x: 621, y: 36 },
                { x: 651, y: 36 },
                { x: 681, y: 36 },
                { x: 711, y: 36 },
                { x: 741, y: 36 },
                { x: 771, y: 35 },
                { x: 800, y: 35 },
                { x: 822, y: 41 },
              ],
            },
            {
              label: "climb the right stem",
              path: [
                { x: 822, y: 41 },
                { x: 836, y: 60 },
                { x: 841, y: 87 },
                { x: 842, y: 117 },
                { x: 842, y: 147 },
                { x: 842, y: 177 },
                { x: 842, y: 207 },
                { x: 843, y: 237 },
                { x: 843, y: 267 },
                { x: 843, y: 297 },
                { x: 842, y: 327 },
                { x: 842, y: 357 },
                { x: 842, y: 387 },
                { x: 842, y: 417 },
                { x: 842, y: 447 },
                { x: 842, y: 477 },
                { x: 842, y: 504 },
                { x: 842, y: 530 },
              ],
            },
            {
              label: "come back down and curl left below",
              path: [
                { x: 842, y: 530 },
                { x: 842, y: 500 },
                { x: 842, y: 470 },
                { x: 842, y: 440 },
                { x: 843, y: 410 },
                { x: 843, y: 380 },
                { x: 844, y: 350 },
                { x: 844, y: 320 },
                { x: 844, y: 290 },
                { x: 844, y: 260 },
                { x: 843, y: 230 },
                { x: 843, y: 200 },
                { x: 843, y: 170 },
                { x: 842, y: 140 },
                { x: 842, y: 110 },
                { x: 841, y: 80 },
                { x: 838, y: 51 },
                { x: 833, y: 21 },
                { x: 827, y: -8 },
                { x: 817, y: -35 },
                { x: 803, y: -61 },
                { x: 785, y: -85 },
                { x: 763, y: -105 },
                { x: 739, y: -122 },
                { x: 713, y: -135 },
                { x: 685, y: -145 },
                { x: 656, y: -153 },
                { x: 626, y: -157 },
                { x: 596, y: -159 },
                { x: 566, y: -160 },
                { x: 536, y: -159 },
                { x: 509, y: -157 },
                { x: 488, y: -151 },
                { x: 475, y: -144 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൮"),
    },
  ],
  // ൯: one recorded stroke, the first half of ൩ and the second half of ൪. Up
  // the left side, over the arch and down the stem, back up it, over the second
  // arch and down the right side through the crossing, round the bottom, then
  // up through the crossing to the flourish at the top right.
  [
    "malayalam:൯",
    {
      script: "malayalam",
      glyph: "൯",
      strokes: [
        {
          segments: [
            {
              label: "climb the left side",
              path: [
                { x: 186, y: 25 },
                { x: 175, y: 51 },
                { x: 161, y: 75 },
                { x: 143, y: 99 },
                { x: 127, y: 124 },
                { x: 114, y: 151 },
                { x: 102, y: 178 },
                { x: 93, y: 207 },
                { x: 87, y: 236 },
                { x: 84, y: 266 },
                { x: 83, y: 296 },
                { x: 85, y: 326 },
                { x: 90, y: 355 },
              ],
            },
            {
              label: "arch over and down the stem",
              path: [
                { x: 90, y: 355 },
                { x: 99, y: 384 },
                { x: 110, y: 412 },
                { x: 124, y: 438 },
                { x: 141, y: 462 },
                { x: 161, y: 483 },
                { x: 185, y: 501 },
                { x: 211, y: 514 },
                { x: 239, y: 523 },
                { x: 268, y: 528 },
                { x: 298, y: 527 },
                { x: 327, y: 523 },
                { x: 355, y: 514 },
                { x: 380, y: 499 },
                { x: 403, y: 480 },
                { x: 422, y: 458 },
                { x: 439, y: 434 },
                { x: 451, y: 408 },
                { x: 456, y: 380 },
                { x: 456, y: 350 },
                { x: 455, y: 320 },
                { x: 454, y: 290 },
                { x: 454, y: 260 },
                { x: 454, y: 230 },
                { x: 454, y: 200 },
                { x: 454, y: 170 },
                { x: 454, y: 140 },
                { x: 454, y: 110 },
                { x: 454, y: 81 },
                { x: 453, y: 52 },
                { x: 450, y: 22 },
              ],
            },
            {
              label: "climb back up the stem",
              path: [
                { x: 450, y: 22 },
                { x: 453, y: 52 },
                { x: 454, y: 82 },
                { x: 454, y: 112 },
                { x: 454, y: 142 },
                { x: 454, y: 172 },
                { x: 454, y: 202 },
                { x: 454, y: 232 },
                { x: 454, y: 262 },
                { x: 454, y: 292 },
                { x: 455, y: 322 },
                { x: 456, y: 352 },
              ],
            },
            {
              label: "arch over and down the right side",
              path: [
                { x: 456, y: 352 },
                { x: 461, y: 381 },
                { x: 470, y: 408 },
                { x: 485, y: 433 },
                { x: 504, y: 456 },
                { x: 526, y: 476 },
                { x: 550, y: 494 },
                { x: 576, y: 509 },
                { x: 603, y: 518 },
                { x: 632, y: 524 },
                { x: 662, y: 527 },
                { x: 692, y: 528 },
                { x: 721, y: 526 },
                { x: 748, y: 523 },
                { x: 770, y: 519 },
                { x: 786, y: 513 },
                { x: 800, y: 506 },
                { x: 812, y: 497 },
                { x: 824, y: 488 },
                { x: 836, y: 480 },
                { x: 848, y: 471 },
                { x: 860, y: 463 },
                { x: 872, y: 454 },
                { x: 885, y: 445 },
                { x: 900, y: 434 },
                { x: 919, y: 417 },
                { x: 937, y: 396 },
                { x: 952, y: 371 },
                { x: 965, y: 344 },
                { x: 974, y: 315 },
                { x: 981, y: 287 },
                { x: 985, y: 257 },
                { x: 987, y: 227 },
                { x: 986, y: 197 },
                { x: 983, y: 168 },
                { x: 975, y: 139 },
                { x: 964, y: 112 },
                { x: 950, y: 86 },
              ],
            },
            {
              label: "round the bottom up to the crossing",
              path: [
                { x: 950, y: 86 },
                { x: 930, y: 65 },
                { x: 906, y: 48 },
                { x: 879, y: 37 },
                { x: 850, y: 30 },
                { x: 821, y: 27 },
                { x: 791, y: 29 },
                { x: 763, y: 36 },
                { x: 736, y: 47 },
                { x: 711, y: 64 },
                { x: 691, y: 85 },
                { x: 677, y: 110 },
                { x: 668, y: 137 },
                { x: 665, y: 166 },
                { x: 665, y: 196 },
                { x: 668, y: 226 },
                { x: 675, y: 255 },
                { x: 686, y: 282 },
                { x: 702, y: 307 },
                { x: 721, y: 331 },
                { x: 740, y: 351 },
                { x: 758, y: 368 },
                { x: 771, y: 380 },
                { x: 781, y: 389 },
                { x: 791, y: 399 },
                { x: 803, y: 408 },
                { x: 815, y: 416 },
                { x: 827, y: 424 },
                { x: 839, y: 432 },
              ],
            },
            {
              label: "sweep up to the top right",
              path: [
                { x: 839, y: 432 },
                { x: 852, y: 440 },
                { x: 864, y: 448 },
                { x: 876, y: 457 },
                { x: 888, y: 465 },
                { x: 901, y: 473 },
                { x: 914, y: 482 },
                { x: 931, y: 491 },
                { x: 952, y: 500 },
                { x: 977, y: 508 },
                { x: 1002, y: 521 },
                { x: 1025, y: 538 },
                { x: 1046, y: 559 },
                { x: 1061, y: 584 },
                { x: 1068, y: 612 },
                { x: 1070, y: 641 },
                { x: 1072, y: 670 },
                { x: 1075, y: 693 },
                { x: 1078, y: 709 },
              ],
            },
          ],
        },
      ],
      source: malayalamDigitSource("൯"),
    },
  ],
  // ൊ: Jayasree records it as two strokes: the left sign െ (its small loop,
  // then over and down to the foot), a lift, then ാ clockwise from its upper
  // tip. Noto prints the standalone sign as the cited െ outline, a placeholder
  // dot where the consonant would sit, and the cited ാ outline 923 units
  // right, so each run is the cited path, the second shifted with it. The dot is
  // not drawn: see NOTO_PLACEHOLDER_CONTOURS in the stroke-honesty tests.
  [
    "malayalam:ൊ",
    {
      script: "malayalam",
      glyph: "ൊ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small inner loop",
              path: [
                { x: 183, y: 284 },
                { x: 252, y: 288 },
                { x: 317, y: 268 },
                { x: 358, y: 213 },
                { x: 363, y: 144 },
                { x: 343, y: 79 },
                { x: 291, y: 34 },
                { x: 222, y: 24 },
                { x: 157, y: 47 },
                { x: 110, y: 97 },
                { x: 88, y: 162 },
                { x: 87, y: 232 },
              ],
            },
            {
              label: "arch over and down to the foot",
              path: [
                { x: 87, y: 232 },
                { x: 91, y: 302 },
                { x: 109, y: 370 },
                { x: 147, y: 430 },
                { x: 199, y: 477 },
                { x: 261, y: 510 },
                { x: 330, y: 526 },
                { x: 400, y: 528 },
                { x: 470, y: 516 },
                { x: 533, y: 486 },
                { x: 584, y: 437 },
                { x: 616, y: 374 },
                { x: 629, y: 305 },
                { x: 630, y: 234 },
                { x: 612, y: 166 },
                { x: 579, y: 104 },
                { x: 531, y: 52 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw ാ clockwise",
              path: [
                { x: 1004, y: 424 },
                { x: 1044, y: 480 },
                { x: 1100, y: 520 },
                { x: 1168, y: 528 },
                { x: 1235, y: 516 },
                { x: 1289, y: 473 },
                { x: 1321, y: 412 },
                { x: 1336, y: 345 },
                { x: 1340, y: 276 },
                { x: 1338, y: 207 },
                { x: 1321, y: 140 },
                { x: 1283, y: 83 },
                { x: 1231, y: 38 },
                { x: 1164, y: 24 },
                { x: 1095, y: 32 },
                { x: 1040, y: 72 },
                { x: 1000, y: 128 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ൊ"),
    },
  ],
  // ോ: Jayasree records it as two strokes: the left sign േ (its top loop,
  // left round the bottom, up round the lower loop), a lift, then ാ clockwise
  // from its upper tip. Noto prints the standalone sign as the cited േ outline,
  // a placeholder dot where the consonant would sit, and the cited ാ outline 788
  // units right, so each run is the cited path, the second shifted with it.
  // The dot is not drawn: see NOTO_PLACEHOLDER_CONTOURS in the stroke-honesty
  // tests.
  [
    "malayalam:ോ",
    {
      script: "malayalam",
      glyph: "ോ",
      strokes: [
        {
          segments: [
            {
              label: "circle the small top loop",
              path: [
                { x: 327, y: 512 },
                { x: 319, y: 441 },
                { x: 332, y: 371 },
                { x: 392, y: 337 },
                { x: 463, y: 342 },
                { x: 511, y: 393 },
                { x: 512, y: 464 },
                { x: 464, y: 516 },
                { x: 395, y: 532 },
                { x: 327, y: 516 },
              ],
            },
            {
              label: "sweep left and round the bottom",
              path: [
                { x: 327, y: 516 },
                { x: 257, y: 518 },
                { x: 192, y: 490 },
                { x: 140, y: 444 },
                { x: 103, y: 384 },
                { x: 87, y: 315 },
                { x: 87, y: 245 },
                { x: 102, y: 176 },
                { x: 139, y: 116 },
                { x: 190, y: 68 },
                { x: 253, y: 37 },
                { x: 323, y: 36 },
              ],
            },
            {
              label: "curl up round the lower loop",
              path: [
                { x: 323, y: 36 },
                { x: 395, y: 24 },
                { x: 465, y: 42 },
                { x: 506, y: 100 },
                { x: 502, y: 174 },
                { x: 445, y: 217 },
                { x: 372, y: 217 },
                { x: 318, y: 169 },
                { x: 311, y: 96 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw ാ clockwise",
              path: [
                { x: 869, y: 424 },
                { x: 909, y: 480 },
                { x: 965, y: 520 },
                { x: 1033, y: 528 },
                { x: 1100, y: 516 },
                { x: 1154, y: 473 },
                { x: 1186, y: 412 },
                { x: 1201, y: 345 },
                { x: 1205, y: 276 },
                { x: 1203, y: 207 },
                { x: 1186, y: 140 },
                { x: 1148, y: 83 },
                { x: 1096, y: 38 },
                { x: 1029, y: 24 },
                { x: 960, y: 32 },
                { x: 905, y: 72 },
                { x: 865, y: 128 },
              ],
            },
          ],
        },
      ],
      source: malayalamMarkSource("ോ"),
    },
  ],
];
