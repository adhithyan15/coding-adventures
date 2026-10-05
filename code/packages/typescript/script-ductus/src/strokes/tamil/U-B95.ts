import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "க",
  {
    script: "tamil",
    glyph: "க",
    strokes: [
      {
        segments: [
          {
            label: "climb the left upright",
            path: [
              { x: 205, y: 292 },
              { x: 201, y: 364 },
              { x: 197, y: 436 },
              { x: 205, y: 508 },
            ],
          },
          {
            label: "carry the top bar right and back",
            path: [
              { x: 205, y: 508 },
              { x: 270, y: 516 },
              { x: 337, y: 516 },
              { x: 403, y: 516 },
              { x: 469, y: 511 },
              { x: 535, y: 516 },
              { x: 601, y: 516 },
              { x: 525, y: 516 },
              { x: 453, y: 500 },
            ],
          },
          {
            label: "drop the inner upright",
            path: [
              { x: 453, y: 500 },
              { x: 453, y: 431 },
              { x: 453, y: 361 },
              { x: 453, y: 292 },
            ],
          },
          {
            label: "round the lower-left bowl",
            path: [
              { x: 453, y: 292 },
              { x: 453, y: 216 },
              { x: 443, y: 142 },
              { x: 411, y: 75 },
              { x: 347, y: 36 },
              { x: 272, y: 28 },
              { x: 197, y: 32 },
            ],
          },
          {
            label: "return up the outer left side",
            path: [
              { x: 197, y: 32 },
              { x: 133, y: 65 },
              { x: 96, y: 125 },
              { x: 99, y: 197 },
              { x: 144, y: 253 },
              { x: 205, y: 292 },
            ],
          },
          {
            label: "cross into the right bowl",
            path: [
              { x: 205, y: 292 },
              { x: 277, y: 292 },
              { x: 349, y: 292 },
              { x: 421, y: 292 },
              { x: 493, y: 292 },
              { x: 566, y: 292 },
              { x: 636, y: 279 },
              { x: 698, y: 243 },
              { x: 729, y: 180 },
              { x: 726, y: 108 },
              { x: 684, y: 51 },
              { x: 617, y: 29 },
              { x: 545, y: 32 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 3, க (Univ. of Texas at Austin), p. 191",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. Appendix I Frame 3 numbers six hand-movements for க. Native writers draw them as one continuous stroke, and this ductus follows the six movements in order without lifting: the pen returns up the lower-left bowl's outer side to the middle left and crosses the middle bar into the lower-right bowl. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 88% of the 203 stored prototypes of க are a single pen-down stroke.",
    },
  },
];
