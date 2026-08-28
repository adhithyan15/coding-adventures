import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "வ",
  {
    script: "tamil",
    glyph: "வ",
    strokes: [
      {
        segments: [
          {
            label: "curl outward and climb the left",
            path: [
              { x: 300, y: 35 },
              { x: 310, y: 55 },
              { x: 350, y: 90 },
              { x: 380, y: 145 },
              { x: 380, y: 205 },
              { x: 340, y: 260 },
              { x: 285, y: 292 },
              { x: 225, y: 290 },
              { x: 170, y: 275 },
              { x: 120, y: 275 },
              { x: 95, y: 270 },
              { x: 95, y: 180 },
              { x: 115, y: 105 },
              { x: 175, y: 45 },
              { x: 250, y: 25 },
              { x: 175, y: 45 },
              { x: 115, y: 105 },
              { x: 95, y: 180 },
              { x: 95, y: 270 },
              { x: 120, y: 360 },
              { x: 180, y: 450 },
            ],
          },
          {
            label: "arch over the top and down the right",
            path: [
              { x: 180, y: 450 },
              { x: 270, y: 525 },
              { x: 370, y: 530 },
              { x: 470, y: 500 },
              { x: 545, y: 430 },
              { x: 600, y: 340 },
              { x: 605, y: 255 },
              { x: 585, y: 175 },
              { x: 545, y: 105 },
            ],
          },
          {
            label: "turn down to the baseline",
            path: [
              { x: 545, y: 105 },
              { x: 550, y: 80 },
              { x: 555, y: 55 },
              { x: 555, y: 35 },
              { x: 515, y: 35 },
            ],
          },
          {
            label: "carry the bottom bar right",
            path: [
              { x: 515, y: 35 },
              { x: 650, y: 35 },
              { x: 780, y: 35 },
              { x: 913, y: 35 },
            ],
          },
          {
            label: "rise up the right upright",
            path: [
              { x: 913, y: 35 },
              { x: 913, y: 180 },
              { x: 913, y: 350 },
              { x: 913, y: 515 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 9, வ (Univ. of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
];
