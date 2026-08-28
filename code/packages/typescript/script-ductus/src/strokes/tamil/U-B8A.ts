import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ஊ",
  {
    script: "tamil",
    glyph: "ஊ",
    strokes: [
      {
        segments: [
          {
            label: "sweep outward around the compact upper spiral",
            path: [
              { x: 320, y: 520 },
              { x: 250, y: 535 },
              { x: 180, y: 520 },
              { x: 120, y: 465 },
              { x: 95, y: 400 },
              { x: 115, y: 335 },
              { x: 165, y: 290 },
              { x: 230, y: 275 },
              { x: 285, y: 295 },
              { x: 325, y: 330 },
              { x: 340, y: 375 },
              { x: 320, y: 415 },
              { x: 320, y: 425 },
            ],
          },
          {
            label:
              "descend through the broad outer curve and turn left onto the baseline",
            path: [
              { x: 320, y: 425 },
              { x: 390, y: 500 },
              { x: 510, y: 515 },
              { x: 555, y: 450 },
              { x: 585, y: 375 },
              { x: 575, y: 300 },
              { x: 540, y: 225 },
              { x: 480, y: 175 },
              { x: 380, y: 175 },
              { x: 280, y: 175 },
              { x: 200, y: 140 },
              { x: 140, y: 80 },
              { x: 115, y: 35 },
            ],
          },
          {
            label: "carry the long baseline straight to the right",
            path: [
              { x: 115, y: 35 },
              { x: 350, y: 35 },
              { x: 700, y: 35 },
              { x: 1050, y: 35 },
              { x: 1300, y: 35 },
              { x: 1480, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "curl around the added left loop",
            path: [
              { x: 780, y: 175 },
              { x: 730, y: 275 },
              { x: 730, y: 425 },
              { x: 820, y: 510 },
              { x: 950, y: 520 },
            ],
          },
          {
            label: "turn inward through the added loop",
            path: [
              { x: 950, y: 520 },
              { x: 1060, y: 470 },
              { x: 1110, y: 375 },
              { x: 1110, y: 275 },
              { x: 1080, y: 190 },
              { x: 960, y: 175 },
            ],
          },
          {
            label: "descend around the added inner curl",
            path: [
              { x: 960, y: 175 },
              { x: 940, y: 230 },
              { x: 960, y: 290 },
              { x: 960, y: 340 },
              { x: 920, y: 375 },
              { x: 840, y: 375 },
              { x: 780, y: 325 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "rise on the added adjoining upright",
            path: [
              { x: 1135, y: 35 },
              { x: 1135, y: 210 },
              { x: 1135, y: 390 },
              { x: 1135, y: 530 },
            ],
          },
          {
            label: "carry the added top bar right",
            path: [
              { x: 1135, y: 530 },
              { x: 1250, y: 530 },
              { x: 1375, y: 530 },
              { x: 1490, y: 530 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the added separate right upright",
            path: [
              { x: 1340, y: 530 },
              { x: 1340, y: 400 },
              { x: 1340, y: 270 },
              { x: 1340, y: 150 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Module 17, ஊ construction, with Appendix I: Hand-movements, Frames 17, 16, and 12 (University of Texas at Austin), pp. 195–196",
      url: "https://sites.la.utexas.edu/tamilscript/frame-17/92",
      variation:
        "Module 17 identifies ஊ as long ū and explicitly constructs it from two familiar letters: write உ first, then write ள over it. Appendix I Frame 16 keeps உ's three movements joined, while Frame 12 groups ள's six movements into three pen-down runs. The resulting four-run learner order is fitted to the bundled Noto Sans Tamil ஊ outline; Tamil handwriting varies by school.",
    },
  },
];
