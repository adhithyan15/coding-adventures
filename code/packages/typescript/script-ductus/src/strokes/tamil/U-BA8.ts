import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ந",
  {
    script: "tamil",
    glyph: "ந",
    strokes: [
      {
        segments: [
          {
            label: "draw the left upright upward",
            path: [
              { x: 130, y: 25 },
              { x: 130, y: 220 },
              { x: 130, y: 380 },
              { x: 130, y: 518 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 130, y: 518 },
              { x: 250, y: 518 },
              { x: 390, y: 518 },
              { x: 500, y: 518 },
              { x: 605, y: 518 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "return left to the middle upright",
            path: [
              { x: 605, y: 518 },
              { x: 540, y: 518 },
              { x: 465, y: 518 },
              { x: 390, y: 518 },
            ],
          },
          {
            label: "descend the middle upright",
            path: [
              { x: 390, y: 518 },
              { x: 390, y: 350 },
              { x: 390, y: 180 },
              { x: 390, y: 25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend around the right bowl",
            path: [
              { x: 540, y: 325 },
              { x: 620, y: 300 },
              { x: 675, y: 235 },
              { x: 700, y: 155 },
              { x: 690, y: 75 },
              { x: 645, y: -10 },
              { x: 575, y: -70 },
              { x: 470, y: -105 },
              { x: 360, y: -120 },
              { x: 250, y: -120 },
            ],
          },
          {
            label: "sweep left into the below-baseline tail",
            path: [
              { x: 250, y: -120 },
              { x: 160, y: -140 },
              { x: 110, y: -185 },
              { x: 92, y: -235 },
              { x: 92, y: -300 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 5, ந (University of Texas at Austin), p. 193",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Module 5 identifies ந as the voiced dental nasal and notes that its extended final curve may be omitted. Appendix I Frame 5 numbers six movements in three pen-down runs: joined movements 1–2, joined movements 3–4, and joined movements 5–6. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
