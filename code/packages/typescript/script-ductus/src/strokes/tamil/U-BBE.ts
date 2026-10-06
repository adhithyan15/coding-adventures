import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ா",
  {
    script: "tamil",
    glyph: "ா",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 131, y: 330 },
              { x: 131, y: 250 },
              { x: 131, y: 170 },
              { x: 131, y: 90 },
              { x: 131, y: 20 },
            ],
          },
          {
            label: "back up and along the top",
            path: [
              { x: 131, y: 20 },
              { x: 131, y: 120 },
              { x: 131, y: 220 },
              { x: 131, y: 320 },
              { x: 131, y: 420 },
              { x: 131, y: 518 },
              { x: 230, y: 518 },
              { x: 330, y: 518 },
              { x: 430, y: 518 },
              { x: 530, y: 518 },
              { x: 600, y: 518 },
            ],
          },
          {
            label: "back, then down the stem",
            path: [
              { x: 600, y: 518 },
              { x: 500, y: 518 },
              { x: 410, y: 518 },
              { x: 410, y: 420 },
              { x: 410, y: 320 },
              { x: 410, y: 220 },
              { x: 410, y: 120 },
              { x: 410, y: 20 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 36 (ா, aa sign): stored online-handwriting prototypes from native Tamil writers (MIT licence, 2012)",
      url: "https://lipitk.sourceforge.net/lipi-reco.htm",
      variation:
        "The recognizer stores native Tamil writers' tablet pen traces, each resampled to 60 points. 189 of the 217 stored prototypes of the aa sign (87%) are one pen-down stroke. In 135 of those 189 the pen starts on the left, partway down, and first moves down; 154 end at the foot of the right upright. So the left upright is drawn down, climbed back to the top bar, and the right upright comes last, without lifting. The traces are scaled to a square, so they fix the start, the direction and the pen lifts, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline, whose top bar runs a little past the right upright. Handwriting varies by writer.",
    },
  },
];
