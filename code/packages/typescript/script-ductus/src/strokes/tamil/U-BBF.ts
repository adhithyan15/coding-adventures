import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ி",
  {
    script: "tamil",
    glyph: "ி",
    strokes: [
      {
        segments: [
          {
            label: "curl up from the hook's tip",
            path: [
              { x: -228, y: 520 },
              { x: -245, y: 560 },
              { x: -258, y: 630 },
              { x: -250, y: 690 },
              { x: -215, y: 745 },
              { x: -160, y: 785 },
            ],
          },
          {
            label: "over the top to the right",
            path: [
              { x: -160, y: 785 },
              { x: -90, y: 805 },
              { x: -20, y: 795 },
              { x: 45, y: 765 },
              { x: 95, y: 705 },
              { x: 122, y: 630 },
            ],
          },
          {
            label: "down the stem to the line",
            path: [
              { x: 122, y: 630 },
              { x: 131, y: 540 },
              { x: 131, y: 440 },
              { x: 131, y: 330 },
              { x: 131, y: 220 },
              { x: 131, y: 110 },
              { x: 131, y: 20 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 37 (ி, i sign): stored online-handwriting prototypes from native Tamil writers (MIT licence, 2012)",
      url: "https://lipitk.sourceforge.net/lipi-reco.htm",
      variation:
        "The recognizer stores native Tamil writers' tablet pen traces, each resampled to 60 points. 205 of the 206 stored prototypes of the i sign are one pen-down stroke. In 161 of those 205 the pen starts at the tip of the hook and first moves up; 200 turn clockwise, over the top to the right; 152 end at the foot of the stem. The traces are scaled to a square, so they fix the start, the direction and the pen lifts, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline of the sign on its own. Handwriting varies by writer.",
    },
  },
];
