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
              { x: 131, y: 518 },
              { x: 131, y: 300 },
              { x: 131, y: 40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "across the top bar",
            path: [
              { x: 131, y: 518 },
              { x: 280, y: 518 },
              { x: 450, y: 518 },
              { x: 580, y: 518 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "down the central upright",
            path: [
              { x: 410, y: 518 },
              { x: 410, y: 300 },
              { x: 410, y: 40 },
            ],
          },
          {
            label: "around the short angular tail",
            path: [
              { x: 410, y: 40 },
              { x: 330, y: -40 },
              { x: 235, y: -135 },
              { x: 200, y: -170 },
              { x: 231, y: -210 },
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
        "Frame 3 identifies ர as the three-movement ஈ frame plus a slightly angular short fourth movement. Tamil handwriting varies by school; this is one attested three-run order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
