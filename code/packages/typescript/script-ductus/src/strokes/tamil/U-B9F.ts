import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ட",
  {
    script: "tamil",
    glyph: "ட",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 131, y: 554 },
              { x: 131, y: 300 },
              { x: 131, y: 36 },
            ],
          },
          {
            label: "along the long rightward foot",
            path: [
              { x: 131, y: 36 },
              { x: 330, y: 36 },
              { x: 520, y: 36 },
              { x: 676, y: 36 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 1, ட (University of Texas at Austin), p. 190",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Frame 1 numbers ட's left descent and rightward foot as two joined movements, while Module 1 identifies the letter and teaches the usual top-to-bottom and left-to-right motion. Tamil handwriting varies by school; this is one attested continuous order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
