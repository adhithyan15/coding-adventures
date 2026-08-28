import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ப",
  {
    script: "tamil",
    glyph: "ப",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 110, y: 548 },
              { x: 104, y: 120 },
              { x: 104, y: 40 },
            ],
          },
          {
            label: "along the bottom",
            path: [
              { x: 104, y: 40 },
              { x: 240, y: 24 },
              { x: 410, y: 22 },
              { x: 570, y: 28 },
              { x: 664, y: 42 },
            ],
          },
          {
            label: "up the right upright",
            path: [
              { x: 664, y: 42 },
              { x: 674, y: 180 },
              { x: 672, y: 430 },
              { x: 672, y: 548 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Frame 1, ப (University of Texas at Austin, 2009)",
      url: "https://sites.la.utexas.edu/tamilscript/category/3-moduals/module-01",
      variation:
        "Frame 1 teaches the usual left-to-right and top-to-bottom movement and directly presents ப for copying. Tamil handwriting varies by school; this is one attested continuous order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
