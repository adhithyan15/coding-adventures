import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "எ",
  {
    script: "tamil",
    glyph: "எ",
    strokes: [
      {
        segments: [
          {
            label: "climb the outer left side",
            path: [
              { x: 100, y: 220 },
              { x: 100, y: 310 },
              { x: 130, y: 410 },
              { x: 200, y: 480 },
            ],
          },
          {
            label: "carry the top bar to the right",
            path: [
              { x: 200, y: 480 },
              { x: 320, y: 520 },
              { x: 480, y: 520 },
              { x: 620, y: 520 },
              { x: 760, y: 520 },
            ],
          },
          {
            label: "retrace left and drop the inner upright",
            path: [
              { x: 760, y: 520 },
              { x: 650, y: 520 },
              { x: 520, y: 520 },
              { x: 380, y: 515 },
              { x: 250, y: 500 },
              { x: 190, y: 455 },
              { x: 150, y: 405 },
              { x: 125, y: 350 },
              { x: 105, y: 300 },
              { x: 100, y: 250 },
            ],
          },
          {
            label: "turn left into the inner spiral",
            path: [
              { x: 100, y: 250 },
              { x: 135, y: 275 },
              { x: 210, y: 285 },
              { x: 300, y: 280 },
              { x: 360, y: 260 },
              { x: 385, y: 220 },
              { x: 385, y: 170 },
              { x: 375, y: 120 },
              { x: 355, y: 80 },
            ],
          },
          {
            label: "sweep around the broad outer curve",
            path: [
              { x: 355, y: 80 },
              { x: 320, y: 55 },
              { x: 275, y: 35 },
              { x: 225, y: 25 },
              { x: 175, y: 30 },
              { x: 130, y: 55 },
              { x: 100, y: 90 },
              { x: 80, y: 135 },
              { x: 75, y: 180 },
              { x: 85, y: 220 },
              { x: 105, y: 250 },
              { x: 135, y: 270 },
              { x: 120, y: 225 },
              { x: 95, y: 175 },
              { x: 90, y: 125 },
              { x: 110, y: 80 },
              { x: 145, y: 45 },
              { x: 190, y: 15 },
            ],
          },
          {
            label: "carry the lower foot right",
            path: [
              { x: 190, y: 15 },
              { x: 230, y: 10 },
              { x: 275, y: 10 },
              { x: 315, y: 10 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "draw the separate right upright up",
            path: [
              { x: 570, y: 10 },
              { x: 570, y: 150 },
              { x: 570, y: 300 },
              { x: 570, y: 450 },
              { x: 570, y: 520 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 5, எ (University of Texas at Austin), p. 193",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 5 numbers எ's first six movements as one connected body and its upward right upright as movement 7 after one lift. Tamil handwriting varies by school; this is one attested two-run order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
