// Authored telugu ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import telugu from "../../../../../learning/human-languages/data/scripts/telugu.json";

const teluguIndependentVowelSource = (glyph: string): StrokeSource => {
  const letter = telugu.independentVowels.find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Telugu independent vowel ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  [
    "telugu:అ",
    {
      script: "telugu",
      glyph: "అ",
      strokes: [
        {
          segments: [
            {
              label: "turn around the left lobe",
              path: [
                { x: 142, y: 258 },
                { x: 200, y: 270 },
                { x: 260, y: 310 },
                { x: 315, y: 370 },
                { x: 310, y: 425 },
                { x: 265, y: 468 },
                { x: 220, y: 468 },
                { x: 155, y: 450 },
                { x: 100, y: 395 },
                { x: 72, y: 325 },
                { x: 74, y: 245 },
                { x: 90, y: 180 },
              ],
            },
            {
              label: "sweep around the broad lower bowl",
              path: [
                { x: 90, y: 180 },
                { x: 125, y: 105 },
                { x: 215, y: 48 },
                { x: 315, y: 24 },
                { x: 420, y: 24 },
                { x: 535, y: 50 },
                { x: 630, y: 105 },
                { x: 700, y: 185 },
                { x: 724, y: 270 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "turn around the right lobe",
              path: [
                { x: 610, y: 220 },
                { x: 610, y: 190 },
                { x: 665, y: 170 },
                { x: 690, y: 220 },
                { x: 700, y: 270 },
                { x: 710, y: 300 },
                { x: 715, y: 360 },
                { x: 680, y: 420 },
                { x: 620, y: 465 },
                { x: 575, y: 465 },
                { x: 530, y: 465 },
                { x: 500, y: 435 },
                { x: 478, y: 395 },
                { x: 478, y: 360 },
                { x: 500, y: 320 },
                { x: 550, y: 285 },
                { x: 575, y: 270 },
                { x: 610, y: 255 },
                { x: 610, y: 220 },
              ],
            },
            {
              label: "return left along the inner bar",
              path: [
                { x: 610, y: 220 },
                { x: 520, y: 220 },
                { x: 420, y: 220 },
                { x: 305, y: 220 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("అ"),
    },
  ],
  [
    "telugu:ఆ",
    {
      script: "telugu",
      glyph: "ఆ",
      strokes: [
        {
          segments: [
            {
              label:
                "turn around the hooked left lobe and sweep through the broad lower bowl",
              path: [
                { x: 142, y: 258 },
                { x: 200, y: 270 },
                { x: 260, y: 310 },
                { x: 315, y: 370 },
                { x: 310, y: 425 },
                { x: 265, y: 468 },
                { x: 220, y: 468 },
                { x: 155, y: 450 },
                { x: 100, y: 395 },
                { x: 72, y: 325 },
                { x: 74, y: 245 },
                { x: 90, y: 180 },
                { x: 125, y: 105 },
                { x: 215, y: 48 },
                { x: 315, y: 24 },
                { x: 420, y: 24 },
                { x: 535, y: 50 },
                { x: 630, y: 105 },
                { x: 700, y: 185 },
                { x: 724, y: 270 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "turn around the rounded right lobe and return left along the inner bar",
              path: [
                { x: 610, y: 220 },
                { x: 610, y: 190 },
                { x: 650, y: 205 },
                { x: 665, y: 205 },
                { x: 675, y: 190 },
                { x: 680, y: 175 },
                { x: 690, y: 220 },
                { x: 700, y: 270 },
                { x: 710, y: 300 },
                { x: 715, y: 360 },
                { x: 680, y: 420 },
                { x: 620, y: 465 },
                { x: 575, y: 465 },
                { x: 530, y: 465 },
                { x: 500, y: 435 },
                { x: 478, y: 395 },
                { x: 478, y: 360 },
                { x: 500, y: 320 },
                { x: 550, y: 285 },
                { x: 575, y: 270 },
                { x: 610, y: 255 },
                { x: 610, y: 220 },
                { x: 520, y: 220 },
                { x: 420, y: 220 },
                { x: 305, y: 220 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఆ"),
    },
  ],
  [
    "telugu:ఇ",
    {
      script: "telugu",
      glyph: "ఇ",
      strokes: [
        {
          segments: [
            {
              label: "turn around the broad outer bowl",
              path: [
                { x: 460, y: 220 },
                { x: 430, y: 220 },
                { x: 330, y: 220 },
                { x: 230, y: 235 },
                { x: 155, y: 210 },
                { x: 112, y: 170 },
                { x: 100, y: 125 },
                { x: 118, y: 80 },
                { x: 175, y: 48 },
                { x: 250, y: 28 },
                { x: 340, y: 25 },
                { x: 430, y: 42 },
                { x: 505, y: 78 },
                { x: 555, y: 125 },
                { x: 585, y: 180 },
                { x: 600, y: 240 },
                { x: 610, y: 300 },
                { x: 615, y: 360 },
                { x: 620, y: 300 },
                { x: 620, y: 220 },
                { x: 590, y: 160 },
                { x: 560, y: 100 },
                { x: 560, y: 40 },
                { x: 570, y: -20 },
                { x: 555, y: -60 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "form the compact upper-left lobe",
              path: [
                { x: 320, y: 390 },
                { x: 275, y: 440 },
                { x: 220, y: 468 },
                { x: 155, y: 465 },
                { x: 100, y: 440 },
                { x: 70, y: 400 },
                { x: 70, y: 355 },
                { x: 92, y: 320 },
                { x: 120, y: 310 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "form the angled upper-right shoulder",
              path: [
                { x: 335, y: 390 },
                { x: 380, y: 438 },
                { x: 430, y: 468 },
                { x: 485, y: 468 },
                { x: 535, y: 445 },
                { x: 575, y: 410 },
                { x: 600, y: 365 },
                { x: 610, y: 315 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఇ"),
    },
  ],
  [
    "telugu:ఉ",
    {
      script: "telugu",
      glyph: "ఉ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left across the rounded upper arch",
              path: [
                { x: 610, y: 410 },
                { x: 550, y: 440 },
                { x: 470, y: 465 },
                { x: 380, y: 468 },
                { x: 285, y: 445 },
                { x: 205, y: 405 },
                { x: 145, y: 350 },
                { x: 110, y: 285 },
              ],
            },
            {
              label: "continue down and around the broad lower bowl",
              path: [
                { x: 110, y: 285 },
                { x: 78, y: 215 },
                { x: 78, y: 145 },
                { x: 105, y: 85 },
                { x: 155, y: 45 },
                { x: 215, y: 25 },
                { x: 275, y: 35 },
                { x: 325, y: 70 },
                { x: 365, y: 115 },
                { x: 405, y: 75 },
                { x: 455, y: 42 },
              ],
            },
            {
              label:
                "curl upward around the rounded right lobe without lifting",
              path: [
                { x: 455, y: 42 },
                { x: 520, y: 20 },
                { x: 585, y: 25 },
                { x: 645, y: 55 },
                { x: 685, y: 105 },
                { x: 690, y: 155 },
                { x: 665, y: 205 },
                { x: 620, y: 225 },
                { x: 575, y: 220 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift and draw the inner horizontal bar from left to right",
              path: [
                { x: 95, y: 282 },
                { x: 205, y: 282 },
                { x: 320, y: 282 },
                { x: 440, y: 282 },
                { x: 560, y: 282 },
                { x: 680, y: 282 },
                { x: 750, y: 282 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and draw the short upper headstroke downward",
              path: [
                { x: 378, y: 610 },
                { x: 378, y: 570 },
                { x: 378, y: 525 },
                { x: 378, y: 490 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఉ"),
    },
  ],
  [
    "telugu:ఎ",
    {
      script: "telugu",
      glyph: "ఎ",
      strokes: [
        {
          segments: [
            {
              label: "turn down and left around the compact lower loop",
              path: [
                { x: 275, y: 141 },
                { x: 255, y: 195 },
                { x: 215, y: 235 },
                { x: 170, y: 245 },
                { x: 120, y: 225 },
                { x: 80, y: 180 },
                { x: 68, y: 125 },
                { x: 78, y: 82 },
              ],
            },
            {
              label:
                "continue around its base and return to the central junction",
              path: [
                { x: 78, y: 82 },
                { x: 105, y: 42 },
                { x: 155, y: 24 },
                { x: 205, y: 30 },
                { x: 245, y: 72 },
                { x: 275, y: 141 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "restart at the junction and sweep up through the broad outer arch",
              path: [
                { x: 275, y: 141 },
                { x: 325, y: 95 },
                { x: 390, y: 52 },
                { x: 460, y: 28 },
                { x: 525, y: 45 },
                { x: 585, y: 100 },
                { x: 615, y: 180 },
                { x: 610, y: 270 },
                { x: 575, y: 370 },
                { x: 515, y: 465 },
                { x: 435, y: 545 },
                { x: 345, y: 610 },
                { x: 260, y: 655 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఎ"),
    },
  ],
  [
    "telugu:ఏ",
    {
      script: "telugu",
      glyph: "ఏ",
      strokes: [
        {
          segments: [
            {
              label: "turn down and left around the compact lower loop",
              path: [
                { x: 275, y: 141 },
                { x: 255, y: 195 },
                { x: 215, y: 235 },
                { x: 170, y: 245 },
                { x: 120, y: 225 },
                { x: 80, y: 180 },
                { x: 68, y: 125 },
                { x: 78, y: 82 },
              ],
            },
            {
              label:
                "continue around its base and return to the central junction",
              path: [
                { x: 78, y: 82 },
                { x: 105, y: 42 },
                { x: 155, y: 24 },
                { x: 205, y: 30 },
                { x: 245, y: 72 },
                { x: 275, y: 141 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "restart at the lower-right tail and sweep up through the broad outer arch",
              path: [
                { x: 260, y: 655 },
                { x: 345, y: 610 },
                { x: 435, y: 545 },
                { x: 515, y: 465 },
                { x: 575, y: 370 },
                { x: 610, y: 270 },
                { x: 615, y: 180 },
                { x: 585, y: 100 },
                { x: 525, y: 45 },
                { x: 460, y: 28 },
                { x: 390, y: 52 },
                { x: 325, y: 95 },
                { x: 275, y: 141 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "restart below the upper-left hook and sweep upward to its tip",
              path: [
                { x: 210, y: 535 },
                { x: 175, y: 585 },
                { x: 155, y: 650 },
                { x: 155, y: 705 },
                { x: 175, y: 755 },
                { x: 215, y: 790 },
                { x: 260, y: 795 },
                { x: 295, y: 785 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఏ"),
    },
  ],
  [
    "telugu:ఋ",
    {
      script: "telugu",
      glyph: "ఋ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right across the upper shoulder",
              path: [
                { x: 85, y: 365 },
                { x: 90, y: 405 },
                { x: 125, y: 445 },
                { x: 175, y: 463 },
                { x: 225, y: 460 },
                { x: 275, y: 438 },
                { x: 315, y: 400 },
                { x: 320, y: 360 },
                { x: 300, y: 320 },
                { x: 270, y: 280 },
                { x: 230, y: 250 },
                { x: 190, y: 225 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve down around the left bowl",
              path: [
                { x: 90, y: 300 },
                { x: 80, y: 245 },
                { x: 80, y: 180 },
                { x: 95, y: 115 },
                { x: 130, y: 70 },
                { x: 180, y: 40 },
                { x: 235, y: 35 },
                { x: 285, y: 50 },
                { x: 325, y: 85 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right around the lower bowl",
              path: [
                { x: 350, y: 115 },
                { x: 390, y: 65 },
                { x: 440, y: 40 },
                { x: 500, y: 30 },
                { x: 560, y: 35 },
                { x: 610, y: 65 },
                { x: 650, y: 110 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up around the first right lobe",
              path: [
                { x: 410, y: 125 },
                { x: 450, y: 75 },
                { x: 510, y: 40 },
                { x: 575, y: 45 },
                { x: 630, y: 85 },
                { x: 670, y: 150 },
                { x: 675, y: 225 },
                { x: 650, y: 300 },
                { x: 605, y: 360 },
                { x: 550, y: 410 },
                { x: 475, y: 450 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up around the middle lobe",
              path: [
                { x: 690, y: 125 },
                { x: 755, y: 70 },
                { x: 825, y: 40 },
                { x: 890, y: 45 },
                { x: 945, y: 85 },
                { x: 990, y: 150 },
                { x: 995, y: 225 },
                { x: 970, y: 300 },
                { x: 925, y: 360 },
                { x: 870, y: 410 },
                { x: 815, y: 450 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up around the final lobe",
              path: [
                { x: 1035, y: 125 },
                { x: 1100, y: 70 },
                { x: 1170, y: 40 },
                { x: 1235, y: 45 },
                { x: 1290, y: 85 },
                { x: 1335, y: 150 },
                { x: 1340, y: 225 },
                { x: 1315, y: 300 },
                { x: 1270, y: 360 },
                { x: 1215, y: 410 },
                { x: 1160, y: 450 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఋ"),
    },
  ],
];
