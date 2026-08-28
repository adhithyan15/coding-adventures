import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ள",
  {
    script: "tamil",
    glyph: "ள",
    strokes: [
      {
        segments: [
          {
            label: "curl around the large left loop",
            path: [
              { x: 250, y: 25 },
              { x: 175, y: 35 },
              { x: 115, y: 95 },
              { x: 95, y: 180 },
              { x: 95, y: 270 },
              { x: 120, y: 360 },
              { x: 180, y: 450 },
              { x: 260, y: 520 },
              { x: 355, y: 530 },
            ],
          },
          {
            label: "turn inward through the loop",
            path: [
              { x: 355, y: 530 },
              { x: 300, y: 510 },
              { x: 240, y: 480 },
              { x: 175, y: 450 },
              { x: 145, y: 390 },
              { x: 120, y: 330 },
              { x: 110, y: 300 },
              { x: 180, y: 270 },
              { x: 280, y: 270 },
              { x: 350, y: 270 },
            ],
          },
          {
            label: "descend around the inner curl",
            path: [
              { x: 350, y: 270 },
              { x: 355, y: 240 },
              { x: 370, y: 210 },
              { x: 380, y: 150 },
              { x: 375, y: 90 },
              { x: 340, y: 60 },
              { x: 300, y: 40 },
              { x: 250, y: 30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "rise on the adjoining upright",
            path: [
              { x: 580, y: 25 },
              { x: 580, y: 180 },
              { x: 580, y: 350 },
              { x: 580, y: 515 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 580, y: 515 },
              { x: 680, y: 515 },
              { x: 780, y: 515 },
              { x: 900, y: 515 },
              { x: 1010, y: 515 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the separate right upright",
            path: [
              { x: 840, y: 515 },
              { x: 840, y: 350 },
              { x: 840, y: 180 },
              { x: 840, y: 25 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 12, ள (University of Texas at Austin), p. 195",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Module 12 identifies ள as the retroflex lateral, contrasts it with ல, and directs learners to Appendix I. Frame 12 numbers six movements in three pen-down runs: joined movements 1–3, joined movements 4–5, and separate movement 6. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
