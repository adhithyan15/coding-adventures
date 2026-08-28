import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ங",
  {
    script: "tamil",
    glyph: "ங",
    strokes: [
      {
        segments: [
          {
            label: "draw the detached upright down",
            path: [
              { x: 915, y: 520 },
              { x: 915, y: 400 },
              { x: 915, y: 270 },
              { x: 915, y: 140 },
              { x: 915, y: 40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "climb the tall left body",
            path: [
              { x: 130, y: 40 },
              { x: 130, y: 170 },
              { x: 130, y: 320 },
              { x: 130, y: 500 },
            ],
          },
          {
            label: "carry the top bar right and return inward",
            path: [
              { x: 130, y: 500 },
              { x: 280, y: 510 },
              { x: 430, y: 510 },
              { x: 555, y: 510 },
              { x: 385, y: 510 },
            ],
          },
          {
            label: "descend into the rounded inner turn",
            path: [
              { x: 385, y: 510 },
              { x: 385, y: 420 },
              { x: 385, y: 320 },
              { x: 410, y: 285 },
              { x: 475, y: 315 },
              { x: 545, y: 325 },
              { x: 610, y: 295 },
              { x: 655, y: 245 },
              { x: 675, y: 190 },
              { x: 665, y: 135 },
              { x: 640, y: 85 },
              { x: 610, y: 40 },
            ],
          },
          {
            label: "carry the low bar to the right",
            path: [
              { x: 610, y: 40 },
              { x: 690, y: 36 },
              { x: 770, y: 36 },
            ],
          },
          {
            label: "return left and finish up the inner stem",
            path: [
              { x: 770, y: 36 },
              { x: 660, y: 36 },
              { x: 540, y: 36 },
              { x: 390, y: 36 },
              { x: 385, y: 140 },
              { x: 385, y: 250 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 2, ங (University of Texas at Austin), p. 191",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 2 numbers a detached descending upright before five joined movements build ங's framed and returning body. The bundled Noto Sans Tamil face places that detached upright on the right and squares the handwritten lower return; Tamil handwriting varies by school, so this is one attested two-run order fitted to the font rather than a national standard.",
    },
  },
];
