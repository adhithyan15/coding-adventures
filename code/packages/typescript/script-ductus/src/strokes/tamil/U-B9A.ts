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
              { x: 160, y: 300 },
              { x: 160, y: 400 },
              { x: 160, y: 510 },
            ],
          },
          {
            label: "carry the top bar to the right",
            path: [
              { x: 160, y: 510 },
              { x: 300, y: 510 },
              { x: 450, y: 510 },
              { x: 680, y: 510 },
              { x: 500, y: 510 },
              { x: 420, y: 510 },
            ],
          },
          {
            label: "drop the inner upright and carry right",
            path: [
              { x: 420, y: 510 },
              { x: 420, y: 410 },
              { x: 420, y: 300 },
              { x: 500, y: 300 },
              { x: 680, y: 300 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "turn around and close the lower-left bowl",
            path: [
              { x: 420, y: 300 },
              { x: 420, y: 220 },
              { x: 440, y: 160 },
              { x: 420, y: 100 },
              { x: 380, y: 60 },
              { x: 330, y: 40 },
              { x: 280, y: 35 },
              { x: 200, y: 35 },
              { x: 125, y: 50 },
              { x: 75, y: 100 },
              { x: 65, y: 160 },
              { x: 80, y: 225 },
              { x: 115, y: 275 },
              { x: 160, y: 300 },
              { x: 290, y: 300 },
              { x: 420, y: 300 },
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
        "Frame 3 numbers the three joined upper-frame movements before the separate fourth movement turns around ச's lower-left bowl. Tamil handwriting varies by school; this is one attested two-run order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
