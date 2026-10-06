import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ீ",
  {
    script: "tamil",
    glyph: "ீ",
    strokes: [
      {
        segments: [
          {
            label: "curl up from the tail's foot",
            path: [
              { x: -366, y: 535 },
              { x: -380, y: 580 },
              { x: -388, y: 630 },
              { x: -375, y: 690 },
              { x: -340, y: 740 },
              { x: -280, y: 785 },
            ],
          },
          {
            label: "over the top to the right",
            path: [
              { x: -280, y: 785 },
              { x: -200, y: 810 },
              { x: -120, y: 812 },
              { x: -50, y: 790 },
              { x: -20, y: 750 },
              { x: -27, y: 700 },
            ],
          },
          {
            label: "curl down and back into the small loop",
            path: [
              { x: -27, y: 700 },
              { x: -50, y: 660 },
              { x: -90, y: 632 },
              { x: -123, y: 625 },
              { x: -170, y: 635 },
              { x: -205, y: 665 },
              { x: -218, y: 710 },
              { x: -205, y: 760 },
              { x: -182, y: 795 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 38 (ீ, ii sign): stored online-handwriting prototypes from native Tamil writers (MIT licence, 2012)",
      url: "https://lipitk.sourceforge.net/lipi-reco.htm",
      variation:
        "The recognizer stores native Tamil writers' tablet pen traces, each resampled to 60 points. 225 of the 229 stored prototypes of the ii sign (98%) are one pen-down stroke. In 213 of those 225 the pen starts low, at the foot of the tail, and 168 first move up; 224 turn clockwise, over the top to the right, and 190 end in the upper half, inside the closing curl. The traces are scaled to a square, so they fix the start, the direction and the pen lifts, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline of the sign on its own. Handwriting varies by writer.",
    },
  },
];
