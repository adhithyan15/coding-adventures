import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ய",
  {
    script: "tamil",
    glyph: "ய",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 130, y: 548 },
              { x: 130, y: 280 },
              { x: 130, y: 145 },
            ],
          },
          {
            label: "around the curved foot into the center",
            path: [
              { x: 130, y: 145 },
              { x: 145, y: 90 },
              { x: 185, y: 55 },
              { x: 240, y: 40 },
              { x: 295, y: 55 },
              { x: 335, y: 90 },
              { x: 354, y: 130 },
            ],
          },
          {
            label: "up the central upright",
            path: [
              { x: 354, y: 130 },
              { x: 354, y: 330 },
              { x: 354, y: 548 },
            ],
          },
          {
            label: "retrace down the central upright",
            path: [
              { x: 354, y: 548 },
              { x: 354, y: 300 },
              { x: 354, y: 40 },
            ],
          },
          {
            label: "along the bottom",
            path: [
              { x: 354, y: 40 },
              { x: 500, y: 30 },
              { x: 675, y: 30 },
              { x: 832, y: 40 },
            ],
          },
          {
            label: "up the right upright",
            path: [
              { x: 832, y: 40 },
              { x: 832, y: 285 },
              { x: 832, y: 548 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 1, ய (University of Texas at Austin), p. 190",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Frame 1 numbers six joined movements for ய: down the left, around its foot, up and back down the central upright, across the bottom, and up the right. Tamil handwriting varies by school; this is one attested continuous order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
