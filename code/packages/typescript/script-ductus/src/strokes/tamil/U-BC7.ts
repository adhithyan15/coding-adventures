import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ே",
  {
    script: "tamil",
    glyph: "ே",
    strokes: [
      {
        segments: [
          {
            label: "circle the small upper loop",
            path: [
              { x: 455, y: 783 },
              { x: 420, y: 740 },
              { x: 403, y: 660 },
              { x: 430, y: 590 },
              { x: 480, y: 550 },
              { x: 525, y: 539 },
              { x: 580, y: 560 },
              { x: 630, y: 610 },
              { x: 652, y: 667 },
              { x: 640, y: 730 },
              { x: 600, y: 780 },
              { x: 540, y: 805 },
              { x: 475, y: 807 },
            ],
          },
          {
            label: "sweep left and down the big curve",
            path: [
              { x: 475, y: 807 },
              { x: 380, y: 790 },
              { x: 280, y: 750 },
              { x: 190, y: 680 },
              { x: 130, y: 580 },
              { x: 100, y: 470 },
              { x: 100, y: 370 },
              { x: 125, y: 270 },
              { x: 175, y: 180 },
              { x: 250, y: 110 },
              { x: 340, y: 55 },
              { x: 458, y: 23 },
            ],
          },
          {
            label: "circle up into the lower loop",
            path: [
              { x: 458, y: 23 },
              { x: 540, y: 35 },
              { x: 600, y: 80 },
              { x: 635, y: 140 },
              { x: 638, y: 200 },
              { x: 610, y: 260 },
              { x: 560, y: 300 },
              { x: 510, y: 311 },
              { x: 450, y: 295 },
              { x: 400, y: 250 },
              { x: 380, y: 178 },
              { x: 395, y: 110 },
              { x: 420, y: 60 },
              { x: 437, y: 38 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 42 (ே, ee sign): stored online-handwriting prototypes from native Tamil writers (MIT licence, 2012)",
      url: "https://lipitk.sourceforge.net/lipi-reco.htm",
      variation:
        "The recognizer stores native Tamil writers' tablet pen traces, each resampled to 60 points. 105 of the 108 stored prototypes of the ee sign (97%) are one pen-down stroke. 104 of those 105 turn counterclockwise: the pen starts in the upper half (98), inside the small upper curl, circles it, sweeps left and down the big curve, and curls up into the lower loop, ending in the lower half (102). The traces are scaled to a square, so they fix the start, the direction and the pen lifts, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline. Handwriting varies by writer.",
    },
  },
];
