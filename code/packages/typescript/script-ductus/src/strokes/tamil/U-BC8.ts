import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ை",
  {
    script: "tamil",
    glyph: "ை",
    strokes: [
      {
        segments: [
          {
            label: "circle the small loop on the left",
            path: [
              { x: 100, y: 216 },
              { x: 150, y: 265 },
              { x: 254, y: 293 },
              { x: 330, y: 260 },
              { x: 381, y: 160 },
              { x: 340, y: 60 },
              { x: 252, y: 22 },
              { x: 170, y: 40 },
              { x: 122, y: 90 },
            ],
          },
          {
            label: "climb and arch over to the right",
            path: [
              { x: 122, y: 90 },
              { x: 100, y: 160 },
              { x: 97, y: 240 },
              { x: 100, y: 330 },
              { x: 140, y: 420 },
              { x: 230, y: 505 },
              { x: 320, y: 540 },
              { x: 403, y: 529 },
              { x: 500, y: 520 },
              { x: 600, y: 490 },
              { x: 690, y: 430 },
              { x: 760, y: 340 },
              { x: 785, y: 250 },
              { x: 780, y: 170 },
              { x: 750, y: 90 },
              { x: 700, y: 40 },
              { x: 661, y: 22 },
            ],
          },
          {
            label: "up the middle, over the second arch",
            path: [
              { x: 661, y: 22 },
              { x: 610, y: 40 },
              { x: 560, y: 100 },
              { x: 539, y: 188 },
              { x: 550, y: 290 },
              { x: 580, y: 380 },
              { x: 630, y: 450 },
              { x: 690, y: 500 },
              { x: 760, y: 525 },
              { x: 825, y: 529 },
              { x: 900, y: 520 },
              { x: 980, y: 470 },
              { x: 1035, y: 390 },
              { x: 1052, y: 300 },
              { x: 1045, y: 200 },
              { x: 1015, y: 110 },
              { x: 969, y: 15 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 43 (ை, ai sign): stored online-handwriting prototypes from native Tamil writers (MIT licence, 2012)",
      url: "https://lipitk.sourceforge.net/lipi-reco.htm",
      variation:
        "The recognizer stores native Tamil writers' tablet pen traces, each resampled to 60 points. 189 of the 190 stored prototypes of the ai sign are one pen-down stroke. All 189 turn clockwise: the pen starts on the left (181 of 189), circles the small left loop, arches over to the right, rises through the middle into the second arch, and comes down on the right, where 146 end. The traces are scaled to a square, so they fix the start, the direction and the pen lifts, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline. Handwriting varies by writer.",
    },
  },
];
