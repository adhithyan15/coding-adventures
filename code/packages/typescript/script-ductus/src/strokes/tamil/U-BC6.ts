import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ெ",
  {
    script: "tamil",
    glyph: "ெ",
    strokes: [
      {
        segments: [
          {
            label: "circle the small loop",
            path: [
              { x: 355, y: 40 },
              { x: 310, y: 100 },
              { x: 296, y: 180 },
              { x: 330, y: 260 },
              { x: 380, y: 300 },
              { x: 431, y: 313 },
              { x: 500, y: 295 },
              { x: 550, y: 245 },
              { x: 562, y: 173 },
              { x: 545, y: 100 },
              { x: 490, y: 45 },
              { x: 420, y: 25 },
            ],
          },
          {
            label: "climb the outer curve",
            path: [
              { x: 420, y: 25 },
              { x: 340, y: 30 },
              { x: 250, y: 70 },
              { x: 170, y: 140 },
              { x: 120, y: 240 },
              { x: 100, y: 330 },
              { x: 100, y: 420 },
              { x: 115, y: 520 },
              { x: 160, y: 620 },
              { x: 240, y: 710 },
              { x: 350, y: 780 },
              { x: 475, y: 805 },
            ],
          },
          {
            label: "over and down the right upright",
            path: [
              { x: 475, y: 805 },
              { x: 580, y: 795 },
              { x: 660, y: 760 },
              { x: 720, y: 690 },
              { x: 755, y: 600 },
              { x: 768, y: 500 },
              { x: 770, y: 400 },
              { x: 770, y: 250 },
              { x: 770, y: 100 },
              { x: 770, y: 20 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 41 (ெ, e sign): stored online-handwriting prototypes from native Tamil writers (MIT licence, 2012)",
      url: "https://lipitk.sourceforge.net/lipi-reco.htm",
      variation:
        "The recognizer stores native Tamil writers' tablet pen traces, each resampled to 60 points. 131 of the 132 stored prototypes of the e sign are one pen-down stroke. All 131 turn clockwise: the pen starts inside the small curl (110 of 131), circles it, climbs the outer curve and comes down the right upright, where 115 end. The traces are scaled to a square, so they fix the start, the direction and the pen lifts, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline, which tucks the end of the small curl into the bottom of the outer curve, so the fitted path starts at that tucked end. Handwriting varies by writer.",
    },
  },
];
