// Authored telugu ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import telugu from "../../../../../learning/human-languages/data/scripts/telugu.json";

const teluguLetterSource = (glyph: string): StrokeSource => {
  const letter = [...telugu.letters, ...telugu.independentVowels].find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Telugu letter ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const teluguIndependentVowelSource = teluguLetterSource;

export const entries: DuctusEntry[] = [
  [
    "telugu:ఝ",
    {
      script: "telugu",
      glyph: "ఝ",
      strokes: [
        {
          segments: [
            {
              label: "circle clockwise around the left bowl",
              path: [
                { x: 296, y: 462 }, { x: 215, y: 445 }, { x: 140, y: 400 },
                { x: 90, y: 335 }, { x: 76, y: 255 }, { x: 92, y: 170 },
                { x: 145, y: 95 }, { x: 220, y: 45 }, { x: 296, y: 28 },
                { x: 375, y: 48 }, { x: 445, y: 100 }, { x: 500, y: 175 },
                { x: 517, y: 250 }, { x: 500, y: 330 }, { x: 445, y: 400 },
                { x: 375, y: 445 }, { x: 296, y: 462 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "circle clockwise around the middle bowl",
              path: [
                { x: 648, y: 462 }, { x: 595, y: 425 }, { x: 555, y: 370 },
                { x: 525, y: 300 }, { x: 535, y: 220 }, { x: 575, y: 145 },
                { x: 640, y: 75 }, { x: 710, y: 28 }, { x: 790, y: 45 },
                { x: 850, y: 105 }, { x: 863, y: 190 }, { x: 845, y: 270 },
                { x: 805, y: 345 }, { x: 745, y: 410 }, { x: 648, y: 462 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "circle clockwise around the right bowl",
              path: [
                { x: 995, y: 462 }, { x: 942, y: 425 }, { x: 902, y: 370 },
                { x: 872, y: 300 }, { x: 882, y: 220 }, { x: 922, y: 145 },
                { x: 987, y: 75 }, { x: 1057, y: 28 }, { x: 1137, y: 45 },
                { x: 1197, y: 105 }, { x: 1210, y: 190 }, { x: 1192, y: 270 },
                { x: 1152, y: 345 }, { x: 1092, y: 410 }, { x: 995, y: 462 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 175, y: 520 }, { x: 220, y: 468 }, { x: 275, y: 463 },
                { x: 330, y: 490 }, { x: 380, y: 535 }, { x: 430, y: 590 },
                { x: 485, y: 645 }, { x: 545, y: 685 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and draw the separate downward stem",
              path: [
                { x: 882, y: 10 }, { x: 884, y: -35 }, { x: 886, y: -80 },
                { x: 888, y: -120 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఝ"),
    },
  ],
  [
    "telugu:డ",
    {
      script: "telugu",
      glyph: "డ",
      strokes: [
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 277, y: 500 },
                { x: 220, y: 485 },
                { x: 165, y: 450 },
                { x: 120, y: 410 },
                { x: 75, y: 350 },
                { x: 45, y: 285 },
                { x: 35, y: 195 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 35, y: 195 },
                { x: 55, y: 110 },
                { x: 125, y: 25 },
                { x: 200, y: -12 },
                { x: 275, y: 0 },
                { x: 345, y: 85 },
                { x: 390, y: 130 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right and upward around the lower-right bowl",
              path: [
                { x: 465, y: 35 },
                { x: 535, y: 0 },
                { x: 605, y: 0 },
                { x: 665, y: 35 },
                { x: 705, y: 95 },
                { x: 711, y: 172 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve left around the upper-right shoulder",
              path: [
                { x: 675, y: 385 },
                { x: 620, y: 415 },
                { x: 560, y: 420 },
                { x: 520, y: 395 },
                { x: 520, y: 340 },
                { x: 530, y: 285 },
                { x: 505, y: 230 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl upward through the separate top flourish",
              path: [
                { x: 235, y: 540 },
                { x: 270, y: 565 },
                { x: 310, y: 545 },
                { x: 350, y: 490 },
                { x: 405, y: 505 },
                { x: 465, y: 555 },
                { x: 525, y: 620 },
                { x: 585, y: 675 },
                { x: 645, y: 690 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("డ"),
    },
  ],
  [
    "telugu:ఠ",
    {
      script: "telugu",
      glyph: "ఠ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left and around the broad circular body",
              path: [
                { x: 295, y: 465 },
                { x: 220, y: 455 },
                { x: 145, y: 410 },
                { x: 90, y: 340 },
                { x: 75, y: 255 },
                { x: 90, y: 165 },
                { x: 145, y: 85 },
                { x: 220, y: 35 },
                { x: 300, y: 25 },
                { x: 380, y: 35 },
                { x: 455, y: 85 },
                { x: 515, y: 160 },
                { x: 535, y: 245 },
                { x: 520, y: 325 },
                { x: 480, y: 395 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl upward through the separate top flourish",
              path: [
                { x: 150, y: 550 },
                { x: 180, y: 570 },
                { x: 215, y: 540 },
                { x: 245, y: 485 },
                { x: 285, y: 450 },
                { x: 330, y: 470 },
                { x: 390, y: 525 },
                { x: 445, y: 590 },
                { x: 500, y: 650 },
                { x: 555, y: 685 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "place the separate inner dot",
              path: [
                { x: 270, y: 245 },
                { x: 325, y: 245 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఠ"),
    },
  ],
  [
    "telugu:ట",
    {
      script: "telugu",
      glyph: "ట",
      strokes: [
        {
          segments: [
            {
              label: "curl upward along the inner shoulder",
              path: [
                { x: 400, y: 150 },
                { x: 420, y: 180 },
                { x: 405, y: 210 },
                { x: 360, y: 235 },
                { x: 305, y: 245 },
                { x: 335, y: 270 },
                { x: 365, y: 315 },
                { x: 380, y: 365 },
                { x: 380, y: 415 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 195, y: 485 },
                { x: 135, y: 455 },
                { x: 95, y: 410 },
                { x: 80, y: 355 },
                { x: 95, y: 310 },
                { x: 135, y: 270 },
                { x: 185, y: 245 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 65, y: 150 },
                { x: 95, y: 85 },
                { x: 155, y: 35 },
                { x: 225, y: 5 },
                { x: 305, y: 5 },
                { x: 370, y: 25 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right and upward around the lower-right bowl",
              path: [
                { x: 430, y: 18 },
                { x: 500, y: 0 },
                { x: 570, y: 10 },
                { x: 635, y: 45 },
                { x: 690, y: 105 },
                { x: 720, y: 175 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve upward and left around the outer shoulder",
              path: [
                { x: 690, y: 235 },
                { x: 670, y: 300 },
                { x: 630, y: 360 },
                { x: 585, y: 415 },
                { x: 545, y: 465 },
                { x: 520, y: 490 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate upper stem downward",
              path: [
                { x: 235, y: 620 },
                { x: 235, y: 575 },
                { x: 235, y: 530 },
                { x: 235, y: 485 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ట"),
    },
  ],
  [
    "telugu:ఞ",
    {
      script: "telugu",
      glyph: "ఞ",
      strokes: [
        {
          segments: [
            {
              label: "sweep up around the upper-left loop",
              path: [
                { x: 126, y: 305 },
                { x: 72, y: 330 },
                { x: 62, y: 385 },
                { x: 88, y: 440 },
                { x: 140, y: 468 },
                { x: 200, y: 468 },
                { x: 255, y: 442 },
                { x: 300, y: 398 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right around the upper-right loop",
              path: [
                { x: 338, y: 398 },
                { x: 378, y: 442 },
                { x: 430, y: 468 },
                { x: 490, y: 468 },
                { x: 548, y: 438 },
                { x: 594, y: 390 },
                { x: 620, y: 330 },
                { x: 615, y: 270 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve down and left around the broad lower bowl",
              path: [
                { x: 615, y: 235 },
                { x: 600, y: 175 },
                { x: 565, y: 115 },
                { x: 515, y: 72 },
                { x: 455, y: 42 },
                { x: 390, y: 25 },
                { x: 325, y: 25 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl upward around the inner-left loop",
              path: [
                { x: 145, y: 90 },
                { x: 125, y: 120 },
                { x: 132, y: 150 },
                { x: 160, y: 180 },
                { x: 205, y: 205 },
                { x: 255, y: 220 },
                { x: 310, y: 225 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl down and right around the inner bowl",
              path: [
                { x: 365, y: 186 },
                { x: 415, y: 175 },
                { x: 465, y: 150 },
                { x: 505, y: 120 },
                { x: 535, y: 82 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the short downward tail",
              path: [
                { x: 548, y: 68 },
                { x: 570, y: 35 },
                { x: 582, y: 0 },
                { x: 580, y: -55 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the right horizontal bar",
              path: [
                { x: 700, y: 320 },
                { x: 760, y: 320 },
                { x: 825, y: 320 },
                { x: 900, y: 320 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate upper vertical stem downward",
              path: [
                { x: 780, y: 475 },
                { x: 780, y: 430 },
                { x: 780, y: 385 },
                { x: 780, y: 350 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఞ"),
    },
  ],
  [
    "telugu:ఙ",
    {
      script: "telugu",
      glyph: "ఙ",
      strokes: [
        {
          segments: [
            {
              label: "turn around the compact upper-left lobe",
              path: [
                { x: 128, y: 323 },
                { x: 118, y: 365 },
                { x: 140, y: 410 },
                { x: 175, y: 452 },
                { x: 220, y: 465 },
                { x: 270, y: 445 },
                { x: 310, y: 405 },
                { x: 320, y: 360 },
                { x: 300, y: 305 },
                { x: 260, y: 270 },
              ],
            },
            {
              label: "continue down and around the broad lower bowl",
              path: [
                { x: 260, y: 270 },
                { x: 190, y: 245 },
                { x: 125, y: 215 },
                { x: 85, y: 170 },
                { x: 82, y: 120 },
                { x: 105, y: 75 },
                { x: 150, y: 42 },
                { x: 205, y: 25 },
                { x: 260, y: 35 },
                { x: 315, y: 70 },
                { x: 365, y: 115 },
              ],
            },
            {
              label: "curl upward around the rounded right lobe",
              path: [
                { x: 365, y: 115 },
                { x: 410, y: 75 },
                { x: 465, y: 42 },
                { x: 525, y: 25 },
                { x: 585, y: 35 },
                { x: 635, y: 68 },
                { x: 660, y: 115 },
                { x: 660, y: 160 },
                { x: 640, y: 200 },
                { x: 600, y: 225 },
                { x: 565, y: 220 },
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
                { x: 310, y: 321 },
                { x: 380, y: 321 },
                { x: 450, y: 321 },
                { x: 520, y: 321 },
                { x: 590, y: 321 },
                { x: 660, y: 321 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and draw the short upper headstroke downward",
              path: [
                { x: 499, y: 480 },
                { x: 499, y: 440 },
                { x: 499, y: 400 },
                { x: 499, y: 360 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఙ"),
    },
  ],
  [
    "telugu:జ",
    {
      script: "telugu",
      glyph: "జ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right across the rounded upper-left arch",
              path: [
                { x: 80, y: 360 },
                { x: 95, y: 410 },
                { x: 130, y: 455 },
                { x: 175, y: 475 },
                { x: 220, y: 470 },
                { x: 265, y: 445 },
                { x: 300, y: 405 },
                { x: 320, y: 365 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve down and right around the lower-left bowl",
              path: [
                { x: 215, y: 285 },
                { x: 250, y: 260 },
                { x: 255, y: 238 },
                { x: 225, y: 235 },
                { x: 195, y: 222 },
                { x: 165, y: 205 },
                { x: 138, y: 183 },
                { x: 110, y: 145 },
                { x: 82, y: 110 },
                { x: 95, y: 70 },
                { x: 140, y: 35 },
                { x: 195, y: 18 },
                { x: 245, y: 25 },
                { x: 290, y: 55 },
                { x: 325, y: 105 },
                { x: 336, y: 155 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right and up around the lower-right bowl",
              path: [
                { x: 366, y: 105 },
                { x: 405, y: 62 },
                { x: 455, y: 32 },
                { x: 515, y: 18 },
                { x: 570, y: 28 },
                { x: 620, y: 60 },
                { x: 650, y: 105 },
                { x: 660, y: 155 },
                { x: 650, y: 200 },
                { x: 638, y: 218 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and curl through the upper-right flourish",
              path: [
                { x: 325, y: 350 },
                { x: 350, y: 325 },
                { x: 400, y: 305 },
                { x: 455, y: 298 },
                { x: 515, y: 302 },
                { x: 565, y: 315 },
                { x: 605, y: 340 },
                { x: 625, y: 375 },
                { x: 615, y: 410 },
                { x: 580, y: 435 },
                { x: 530, y: 450 },
                { x: 485, y: 448 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("జ"),
    },
  ],
  [
    "telugu:చ",
    {
      script: "telugu",
      glyph: "చ",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper bar from left to right",
              path: [
                { x: 50, y: 295 },
                { x: 100, y: 295 },
                { x: 150, y: 295 },
                { x: 200, y: 295 },
                { x: 250, y: 295 },
              ],
            },
            {
              label: "continue down and around the left bowl",
              path: [
                { x: 250, y: 295 },
                { x: 220, y: 260 },
                { x: 175, y: 235 },
                { x: 130, y: 205 },
                { x: 105, y: 165 },
                { x: 105, y: 120 },
                { x: 125, y: 80 },
                { x: 165, y: 48 },
                { x: 215, y: 30 },
                { x: 265, y: 35 },
                { x: 310, y: 62 },
                { x: 345, y: 105 },
                { x: 370, y: 155 },
                { x: 385, y: 185 },
              ],
            },
            {
              label: "sweep right and up around the outer bowl",
              path: [
                { x: 385, y: 185 },
                { x: 410, y: 135 },
                { x: 450, y: 92 },
                { x: 505, y: 62 },
                { x: 565, y: 55 },
                { x: 620, y: 78 },
                { x: 660, y: 120 },
                { x: 680, y: 180 },
                { x: 675, y: 245 },
                { x: 650, y: 305 },
                { x: 610, y: 355 },
                { x: 560, y: 392 },
                { x: 505, y: 418 },
                { x: 450, y: 432 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and cup through the upper flourish",
              path: [
                { x: 288, y: 570 },
                { x: 310, y: 520 },
                { x: 340, y: 480 },
                { x: 380, y: 462 },
                { x: 420, y: 465 },
                { x: 460, y: 490 },
                { x: 500, y: 535 },
                { x: 540, y: 585 },
                { x: 585, y: 630 },
                { x: 635, y: 665 },
                { x: 680, y: 683 },
                { x: 710, y: 686 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("చ"),
    },
  ],
  [
    "telugu:ఛ",
    {
      script: "telugu",
      glyph: "ఛ",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper bar from left to right",
              path: [
                { x: 50, y: 295 },
                { x: 100, y: 295 },
                { x: 150, y: 295 },
                { x: 200, y: 295 },
                { x: 250, y: 295 },
              ],
            },
            {
              label: "continue down and around the left bowl",
              path: [
                { x: 250, y: 295 },
                { x: 220, y: 260 },
                { x: 175, y: 235 },
                { x: 130, y: 205 },
                { x: 105, y: 165 },
                { x: 105, y: 120 },
                { x: 125, y: 80 },
                { x: 165, y: 48 },
                { x: 215, y: 30 },
                { x: 265, y: 35 },
                { x: 310, y: 62 },
                { x: 345, y: 105 },
                { x: 370, y: 155 },
                { x: 385, y: 185 },
              ],
            },
            {
              label: "sweep right and up around the outer bowl",
              path: [
                { x: 385, y: 185 },
                { x: 410, y: 135 },
                { x: 450, y: 92 },
                { x: 505, y: 62 },
                { x: 565, y: 55 },
                { x: 620, y: 78 },
                { x: 660, y: 120 },
                { x: 680, y: 180 },
                { x: 675, y: 245 },
                { x: 650, y: 305 },
                { x: 610, y: 355 },
                { x: 560, y: 392 },
                { x: 505, y: 418 },
                { x: 450, y: 432 },
              ],
            },
            {
              label: "continue through the upper flourish",
              path: [
                { x: 450, y: 432 },
                { x: 410, y: 440 },
                { x: 370, y: 460 },
                { x: 335, y: 495 },
                { x: 305, y: 535 },
                { x: 288, y: 570 },
                { x: 325, y: 585 },
                { x: 375, y: 585 },
                { x: 425, y: 570 },
                { x: 470, y: 565 },
                { x: 515, y: 590 },
                { x: 560, y: 625 },
                { x: 610, y: 655 },
                { x: 660, y: 678 },
                { x: 710, y: 686 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and draw the separate downward stem",
              path: [
                { x: 385, y: 12 },
                { x: 385, y: -35 },
                { x: 385, y: -80 },
                { x: 385, y: -120 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఛ"),
    },
  ],
  [
    "telugu:ఘ",
    {
      script: "telugu",
      glyph: "ఘ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left around the upper-left shoulder",
              path: [
                { x: 300, y: 145 },
                { x: 280, y: 195 },
                { x: 245, y: 230 },
                { x: 200, y: 250 },
                { x: 150, y: 248 },
                { x: 110, y: 230 },
                { x: 80, y: 200 },
                { x: 65, y: 165 },
              ],
            },
            {
              label: "continue down and right around the lower-left bowl",
              path: [
                { x: 65, y: 165 },
                { x: 65, y: 125 },
                { x: 75, y: 85 },
                { x: 105, y: 50 },
                { x: 150, y: 28 },
                { x: 200, y: 25 },
                { x: 245, y: 45 },
                { x: 275, y: 80 },
                { x: 300, y: 125 },
                { x: 300, y: 145 },
              ],
            },
            {
              label: "turn upward around the broad middle arch",
              path: [
                { x: 300, y: 145 },
                { x: 330, y: 105 },
                { x: 370, y: 70 },
                { x: 420, y: 45 },
                { x: 480, y: 30 },
                { x: 535, y: 50 },
                { x: 575, y: 90 },
                { x: 600, y: 145 },
                { x: 607, y: 205 },
                { x: 595, y: 270 },
                { x: 565, y: 325 },
                { x: 520, y: 370 },
                { x: 470, y: 405 },
                { x: 420, y: 430 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right and up around the outer arch",
              path: [
                { x: 650, y: 145 },
                { x: 690, y: 95 },
                { x: 745, y: 55 },
                { x: 810, y: 30 },
                { x: 870, y: 45 },
                { x: 915, y: 85 },
                { x: 945, y: 145 },
                { x: 950, y: 210 },
                { x: 935, y: 275 },
                { x: 900, y: 330 },
                { x: 855, y: 375 },
                { x: 805, y: 410 },
                { x: 765, y: 430 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and cup through the upper flourish",
              path: [
                { x: 98, y: 570 },
                { x: 120, y: 520 },
                { x: 150, y: 480 },
                { x: 190, y: 462 },
                { x: 230, y: 465 },
                { x: 270, y: 490 },
                { x: 310, y: 535 },
                { x: 350, y: 585 },
                { x: 395, y: 630 },
                { x: 445, y: 665 },
                { x: 490, y: 683 },
                { x: 520, y: 686 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and draw the separate downward stem",
              path: [
                { x: 325, y: 15 },
                { x: 325, y: -48 },
                { x: 325, y: -118 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఘ"),
    },
  ],
  [
    "telugu:గ",
    {
      script: "telugu",
      glyph: "గ",
      strokes: [
        {
          segments: [
            {
              label: "sweep up and over the broad lower arch",
              path: [
                { x: 120, y: 12 },
                { x: 88, y: 70 },
                { x: 68, y: 145 },
                { x: 80, y: 230 },
                { x: 105, y: 320 },
                { x: 155, y: 405 },
                { x: 220, y: 455 },
                { x: 295, y: 465 },
                { x: 370, y: 445 },
                { x: 430, y: 398 },
                { x: 472, y: 330 },
                { x: 500, y: 250 },
                { x: 493, y: 170 },
                { x: 470, y: 95 },
                { x: 430, y: 25 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "cup through the separate upper flourish",
              path: [
                { x: 165, y: 560 },
                { x: 190, y: 505 },
                { x: 235, y: 465 },
                { x: 275, y: 468 },
                { x: 320, y: 495 },
                { x: 365, y: 540 },
                { x: 415, y: 595 },
                { x: 470, y: 645 },
                { x: 525, y: 680 },
                { x: 580, y: 685 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("గ"),
    },
  ],
  [
    "telugu:ఖ",
    {
      script: "telugu",
      glyph: "ఖ",
      strokes: [
        {
          segments: [
            {
              label: "circle up around the upper-left bowl",
              path: [
                { x: 285, y: 404 },
                { x: 252, y: 446 },
                { x: 198, y: 468 },
                { x: 135, y: 455 },
                { x: 82, y: 416 },
                { x: 65, y: 365 },
                { x: 86, y: 315 },
                { x: 132, y: 275 },
                { x: 188, y: 270 },
                { x: 240, y: 298 },
                { x: 276, y: 344 },
                { x: 285, y: 404 },
              ],
            },
            {
              label: "descend through the central curve",
              path: [
                { x: 285, y: 404 },
                { x: 325, y: 350 },
                { x: 350, y: 290 },
                { x: 360, y: 225 },
                { x: 350, y: 165 },
                { x: 320, y: 112 },
              ],
            },
            {
              label: "turn up around the left shoulder",
              path: [
                { x: 320, y: 112 },
                { x: 270, y: 78 },
                { x: 210, y: 58 },
                { x: 150, y: 58 },
                { x: 100, y: 78 },
                { x: 68, y: 108 },
                { x: 62, y: 140 },
                { x: 78, y: 170 },
                { x: 112, y: 188 },
                { x: 158, y: 190 },
              ],
            },
            {
              label: "sweep right and up around the broad outer bowl",
              path: [
                { x: 158, y: 190 },
                { x: 220, y: 178 },
                { x: 285, y: 145 },
                { x: 350, y: 105 },
                { x: 420, y: 70 },
                { x: 500, y: 42 },
                { x: 570, y: 48 },
                { x: 625, y: 82 },
                { x: 665, y: 140 },
                { x: 675, y: 210 },
                { x: 658, y: 278 },
                { x: 620, y: 338 },
                { x: 565, y: 392 },
                { x: 500, y: 438 },
              ],
            },
            {
              label: "return left along the crown",
              path: [
                { x: 500, y: 438 },
                { x: 455, y: 460 },
                { x: 410, y: 468 },
                { x: 365, y: 456 },
                { x: 325, y: 432 },
                { x: 285, y: 404 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate downward stem",
              path: [
                { x: 343, y: 12 },
                { x: 343, y: -48 },
                { x: 343, y: -118 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఖ"),
    },
  ],
  [
    "telugu:క",
    {
      script: "telugu",
      glyph: "క",
      strokes: [
        {
          segments: [
            {
              label: "turn down and left around the upper bowl",
              path: [
                { x: 442, y: 357 },
                { x: 410, y: 415 },
                { x: 350, y: 452 },
                { x: 270, y: 466 },
                { x: 190, y: 458 },
                { x: 125, y: 425 },
                { x: 87, y: 375 },
                { x: 78, y: 325 },
                { x: 100, y: 292 },
              ],
            },
            {
              label: "continue right through the middle shoulder",
              path: [
                { x: 100, y: 292 },
                { x: 165, y: 268 },
                { x: 235, y: 255 },
                { x: 310, y: 251 },
                { x: 385, y: 240 },
                { x: 442, y: 218 },
              ],
            },
            {
              label: "curve down and left around the lower bowl",
              path: [
                { x: 442, y: 218 },
                { x: 455, y: 165 },
                { x: 430, y: 112 },
                { x: 380, y: 72 },
                { x: 315, y: 45 },
                { x: 245, y: 31 },
                { x: 175, y: 44 },
                { x: 115, y: 78 },
                { x: 77, y: 125 },
              ],
            },
            {
              label: "finish upward along the left tail",
              path: [
                { x: 77, y: 125 },
                { x: 62, y: 155 },
                { x: 57, y: 190 },
                { x: 62, y: 225 },
                { x: 78, y: 250 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep up through the separate headstroke",
              path: [
                { x: 132, y: 548 },
                { x: 165, y: 505 },
                { x: 215, y: 470 },
                { x: 260, y: 468 },
                { x: 315, y: 492 },
                { x: 365, y: 535 },
                { x: 410, y: 585 },
                { x: 455, y: 635 },
                { x: 510, y: 676 },
                { x: 548, y: 684 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("క"),
    },
  ],
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
    "telugu:ఒ",
    {
      script: "telugu",
      glyph: "ఒ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right across the upper arch",
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
              label: "sweep right around the broad lower bowl",
              path: [
                { x: 350, y: 115 },
                { x: 390, y: 65 },
                { x: 440, y: 40 },
                { x: 500, y: 30 },
                { x: 560, y: 35 },
                { x: 610, y: 65 },
                { x: 650, y: 110 },
                { x: 665, y: 155 },
                { x: 650, y: 205 },
                { x: 615, y: 250 },
                { x: 575, y: 285 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఒ"),
    },
  ],
  [
    "telugu:ఐ",
    {
      script: "telugu",
      glyph: "ఐ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left across the compact upper arch",
              path: [
                { x: 390, y: 370 },
                { x: 365, y: 405 },
                { x: 335, y: 430 },
                { x: 300, y: 450 },
                { x: 260, y: 460 },
                { x: 220, y: 455 },
                { x: 185, y: 425 },
                { x: 230, y: 410 },
                { x: 270, y: 380 },
                { x: 295, y: 340 },
                { x: 290, y: 300 },
                { x: 260, y: 270 },
                { x: 220, y: 245 },
                { x: 180, y: 230 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curve down around the left bowl",
              path: [
                { x: 52, y: 302 },
                { x: 42, y: 250 },
                { x: 55, y: 170 },
                { x: 80, y: 95 },
                { x: 120, y: 45 },
                { x: 180, y: 22 },
                { x: 240, y: 30 },
                { x: 290, y: 70 },
                { x: 325, y: 115 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right around the broad lower bowl",
              path: [
                { x: 285, y: 140 },
                { x: 330, y: 90 },
                { x: 380, y: 50 },
                { x: 440, y: 25 },
                { x: 500, y: 25 },
                { x: 555, y: 45 },
                { x: 610, y: 80 },
                { x: 650, y: 135 },
                { x: 668, y: 200 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep left across the upper-right arch",
              path: [
                { x: 650, y: 300 },
                { x: 630, y: 365 },
                { x: 590, y: 420 },
                { x: 535, y: 455 },
                { x: 480, y: 465 },
                { x: 425, y: 450 },
                { x: 375, y: 420 },
                { x: 340, y: 390 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep left across the upper-left arch",
              path: [
                { x: 323, y: 396 },
                { x: 295, y: 435 },
                { x: 255, y: 465 },
                { x: 205, y: 478 },
                { x: 155, y: 472 },
                { x: 110, y: 450 },
                { x: 75, y: 415 },
                { x: 55, y: 375 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఐ"),
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
