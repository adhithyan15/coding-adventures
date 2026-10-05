import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ச",
  {
    script: "tamil",
    glyph: "ச",
    strokes: [
      {
        segments: [
          {
            label: "climb the left upright",
            path: [
              { x: 205, y: 292 },
              { x: 201, y: 364 },
              { x: 201, y: 436 },
              { x: 205, y: 508 },
            ],
          },
          {
            label: "along the top and back",
            path: [
              { x: 205, y: 508 },
              { x: 278, y: 516 },
              { x: 353, y: 519 },
              { x: 426, y: 515 },
              { x: 500, y: 516 },
              { x: 575, y: 516 },
              { x: 649, y: 520 },
              { x: 582, y: 520 },
              { x: 515, y: 520 },
              { x: 453, y: 500 },
            ],
          },
          {
            label: "down the stem, out right",
            path: [
              { x: 453, y: 500 },
              { x: 453, y: 431 },
              { x: 453, y: 361 },
              { x: 453, y: 292 },
              { x: 518, y: 292 },
              { x: 584, y: 292 },
              { x: 649, y: 292 },
            ],
          },
          {
            label: "back, round the bowl",
            path: [
              { x: 649, y: 292 },
              { x: 584, y: 292 },
              { x: 518, y: 292 },
              { x: 453, y: 292 },
              { x: 453, y: 221 },
              { x: 448, y: 151 },
              { x: 421, y: 86 },
              { x: 366, y: 43 },
              { x: 298, y: 28 },
              { x: 227, y: 26 },
              { x: 160, y: 48 },
              { x: 110, y: 97 },
              { x: 97, y: 164 },
              { x: 119, y: 229 },
              { x: 176, y: 271 },
              { x: 241, y: 292 },
              { x: 312, y: 292 },
              { x: 382, y: 292 },
              { x: 453, y: 292 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 3, ச (University of Texas at Austin), p. 191",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Frame 3 numbers three upper-frame movements and a fourth that turns around ச's lower-left bowl. Native writers draw the four movements as one continuous stroke, and this ductus follows them in order without lifting: movement 3 ends at the tip of the middle bar, so the pen comes back along the bar to the inner crossing before it turns around the bowl. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 88% of the 176 stored prototypes of ச are a single pen-down stroke. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
