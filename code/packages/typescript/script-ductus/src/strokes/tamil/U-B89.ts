import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "உ",
  {
    script: "tamil",
    glyph: "உ",
    strokes: [
      {
        segments: [
          {
            label: "sweep outward around the compact upper spiral",
            path: [
              { x: 250, y: 400 },
              { x: 285, y: 430 },
              { x: 320, y: 415 },
              { x: 340, y: 375 },
              { x: 325, y: 330 },
              { x: 285, y: 295 },
              { x: 230, y: 275 },
              { x: 165, y: 290 },
              { x: 115, y: 335 },
              { x: 95, y: 400 },
              { x: 120, y: 465 },
              { x: 180, y: 520 },
              { x: 250, y: 535 },
              { x: 320, y: 520 },
            ],
          },
          {
            label:
              "descend through the broad outer curve and turn left onto the baseline",
            path: [
              { x: 320, y: 520 },
              { x: 405, y: 495 },
              { x: 485, y: 445 },
              { x: 550, y: 375 },
              { x: 560, y: 315 },
              { x: 530, y: 255 },
              { x: 455, y: 205 },
              { x: 365, y: 175 },
              { x: 275, y: 160 },
              { x: 190, y: 150 },
              { x: 115, y: 120 },
              { x: 105, y: 80 },
              { x: 145, y: 40 },
            ],
          },
          {
            label: "carry the long baseline straight to the right",
            path: [
              { x: 145, y: 40 },
              { x: 300, y: 36 },
              { x: 500, y: 36 },
              { x: 700, y: 36 },
              { x: 875, y: 36 },
              { x: 1015, y: 36 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 16, உ (University of Texas at Austin), p. 196",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 16 numbers the upper spiral, descending outer curve, and rightward baseline as three joined movements. Tamil handwriting varies by school; this is one attested continuous order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
