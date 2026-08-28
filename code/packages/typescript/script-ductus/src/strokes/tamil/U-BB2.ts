import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ல",
  {
    script: "tamil",
    glyph: "ல",
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
            label: "arch over and descend through the middle",
            path: [
              { x: 180, y: 450 },
              { x: 260, y: 520 },
              { x: 370, y: 560 },
              { x: 470, y: 530 },
              { x: 530, y: 480 },
              { x: 560, y: 430 },
              { x: 570, y: 350 },
              { x: 570, y: 250 },
              { x: 580, y: 160 },
              { x: 590, y: 100 },
            ],
          },
          {
            label: "turn around the deep right-hand curve",
            path: [
              { x: 590, y: 100 },
              { x: 620, y: 60 },
              { x: 690, y: 30 },
              { x: 770, y: 25 },
              { x: 840, y: 60 },
              { x: 875, y: 110 },
            ],
          },
          {
            label: "rise to the open right tip",
            path: [
              { x: 875, y: 110 },
              { x: 900, y: 200 },
              { x: 900, y: 350 },
              { x: 875, y: 450 },
              { x: 825, y: 540 },
              { x: 790, y: 580 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 9, ல (Univ. of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
];
