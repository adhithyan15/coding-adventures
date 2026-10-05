import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ர",
  {
    script: "tamil",
    glyph: "ர",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 124, y: 520 },
              { x: 132, y: 453 },
              { x: 132, y: 384 },
              { x: 132, y: 315 },
              { x: 132, y: 246 },
              { x: 132, y: 178 },
              { x: 132, y: 109 },
              { x: 132, y: 40 },
            ],
          },
          {
            label: "back up and along the top",
            path: [
              { x: 132, y: 40 },
              { x: 132, y: 109 },
              { x: 132, y: 178 },
              { x: 132, y: 246 },
              { x: 132, y: 315 },
              { x: 132, y: 384 },
              { x: 132, y: 453 },
              { x: 124, y: 520 },
              { x: 200, y: 520 },
              { x: 275, y: 520 },
              { x: 351, y: 520 },
              { x: 426, y: 511 },
              { x: 501, y: 516 },
              { x: 576, y: 520 },
            ],
          },
          {
            label: "back, then down the stem",
            path: [
              { x: 576, y: 520 },
              { x: 492, y: 520 },
              { x: 412, y: 500 },
              { x: 412, y: 431 },
              { x: 412, y: 363 },
              { x: 412, y: 294 },
              { x: 412, y: 225 },
              { x: 412, y: 157 },
              { x: 412, y: 88 },
              { x: 408, y: 20 },
            ],
          },
          {
            label: "into the angled tail",
            path: [
              { x: 408, y: 20 },
              { x: 362, y: -34 },
              { x: 310, y: -82 },
              { x: 260, y: -132 },
              { x: 200, y: -168 },
              { x: 221, y: -179 },
              { x: 232, y: -200 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 3, ர (University of Texas at Austin), p. 191",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Frame 3 identifies ர as the three-movement ஈ frame plus a slightly angular short fourth movement. Native writers draw the four movements as one continuous stroke, and this ductus follows them in order without lifting: the first movement runs down the left upright, so the pen climbs back up it to begin the top bar and returns along the bar to the central upright. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 90% of the 189 stored prototypes of ர are a single pen-down stroke. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
