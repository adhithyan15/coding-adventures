import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "அ",
  {
    script: "tamil",
    glyph: "அ",
    strokes: [
      {
        segments: [
          {
            label: "curl around the upper loop",
            path: [
              { x: 500, y: 510 },
              { x: 440, y: 530 },
              { x: 380, y: 500 },
              { x: 340, y: 445 },
              { x: 335, y: 385 },
              { x: 360, y: 330 },
              { x: 420, y: 285 },
              { x: 490, y: 280 },
              { x: 550, y: 310 },
              { x: 590, y: 365 },
              { x: 595, y: 430 },
            ],
          },
          {
            label: "sweep down the outer curve",
            path: [
              { x: 595, y: 430 },
              { x: 650, y: 500 },
              { x: 710, y: 460 },
              { x: 755, y: 390 },
              { x: 775, y: 300 },
              { x: 775, y: 215 },
              { x: 755, y: 130 },
              { x: 710, y: 55 },
            ],
          },
          {
            label: "turn around the lower loop",
            path: [
              { x: 710, y: 55 },
              { x: 650, y: -5 },
              { x: 570, y: -50 },
              { x: 470, y: -80 },
              { x: 350, y: -90 },
              { x: 240, y: -80 },
              { x: 150, y: -50 },
              { x: 90, y: 0 },
              { x: 70, y: 55 },
              { x: 90, y: 105 },
              { x: 145, y: 140 },
              { x: 215, y: 140 },
            ],
          },
          {
            label: "carry the horizontal to the right",
            path: [
              { x: 215, y: 140 },
              { x: 350, y: 140 },
              { x: 520, y: 140 },
              { x: 700, y: 140 },
              { x: 850, y: 140 },
              { x: 950, y: 140 },
            ],
          },
          {
            label: "climb the right upright to its top",
            path: [
              { x: 950, y: 140 },
              { x: 990, y: 165 },
              { x: 990, y: 290 },
              { x: 990, y: 410 },
              { x: 990, y: 530 },
            ],
          },
          {
            label: "draw the right upright down",
            path: [
              { x: 990, y: 530 },
              { x: 990, y: 390 },
              { x: 990, y: 230 },
              { x: 990, y: 70 },
              { x: 990, y: -130 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 4, அ (Univ. of Texas at Austin), p. 192; drawn in one stroke after HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 0 (அ): stored online-handwriting prototypes from native writers (MIT licence, 2012)",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 4 numbers அ's movements and gives the right upright a number of its own after the horizontal; this ductus used to read that as a pen lift, but a numbered movement is not evidence of one. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 104 of the 114 stored prototypes of அ (91%) are one pen-down stroke. All 104 start in the upper half, at the curl. In 95 of them (91%) the pen runs on from the horizontal up the right upright to its top and then draws it down, and 94 (90%) end at the bottom right. So this ductus overrides the old lift: it follows Frame 4's movements in order without lifting, climbs the upright from the end of the horizontal and draws it straight down over that ink. The traces are scaled to a square, so they fix the stroke count, the start, the order and the direction, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline. The underlying HP Labs data is licensed for research use only, so only counts and shares are cited here, and no trace was copied. Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard.",
    },
  },
];
