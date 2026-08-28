import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ஞ",
  {
    script: "tamil",
    glyph: "ஞ",
    strokes: [
      {
        segments: [
          {
            label: "curve down into the compact left inner turn",
            path: [
              { x: 650, y: 520 },
              { x: 520, y: 510 },
              { x: 405, y: 455 },
              { x: 330, y: 350 },
              { x: 315, y: 225 },
              { x: 350, y: 150 },
            ],
          },
          {
            label: "circle the left inner loop and return upward",
            path: [
              { x: 350, y: 150 },
              { x: 380, y: 75 },
              { x: 450, y: 25 },
              { x: 535, y: 35 },
              { x: 595, y: 95 },
              { x: 610, y: 175 },
              { x: 580, y: 245 },
              { x: 500, y: 275 },
              { x: 415, y: 265 },
              { x: 360, y: 220 },
              { x: 350, y: 150 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "carry the long top bar straight to the right",
            path: [
              { x: 650, y: 520 },
              { x: 800, y: 520 },
              { x: 1005, y: 520 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "curve left from the upper-right shoulder",
            path: [
              { x: 815, y: 520 },
              { x: 815, y: 390 },
              { x: 815, y: 255 },
            ],
          },
          {
            label: "descend the central upright",
            path: [
              { x: 815, y: 255 },
              { x: 815, y: 120 },
              { x: 815, y: 15 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "sweep around the broad outer-right curve",
            path: [
              { x: 815, y: 150 },
              { x: 845, y: 225 },
              { x: 920, y: 270 },
              { x: 1000, y: 250 },
              { x: 1070, y: 175 },
              { x: 1100, y: 30 },
            ],
          },
          {
            label: "continue around the broad bottom bowl",
            path: [
              { x: 1100, y: 30 },
              { x: 1090, y: -115 },
              { x: 980, y: -225 },
              { x: 800, y: -285 },
              { x: 560, y: -300 },
              { x: 330, y: -260 },
              { x: 185, y: -155 },
            ],
          },
          {
            label: "return up the outer-left side",
            path: [
              { x: 185, y: -155 },
              { x: 105, y: -25 },
              { x: 95, y: 145 },
              { x: 125, y: 315 },
              { x: 200, y: 480 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 8, ஞ (University of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 8 numbers eight movements: 1–2 build the left inner loop, 3 draws the top bar, 4–5 form the shoulder and central descent, and 6–8 sweep around the broad outer bowl. Tamil handwriting varies by school; this is one attested four-run order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
