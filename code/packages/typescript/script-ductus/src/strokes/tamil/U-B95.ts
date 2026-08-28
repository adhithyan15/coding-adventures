import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "க",
  {
    script: "tamil",
    glyph: "க",
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
              { x: 580, y: 510 },
              { x: 500, y: 510 },
              { x: 420, y: 510 },
            ],
          },
          {
            label: "drop the inner upright and carry left",
            path: [
              { x: 420, y: 510 },
              { x: 420, y: 410 },
              { x: 420, y: 300 },
              { x: 300, y: 300 },
              { x: 160, y: 300 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "curve down and around the lower left",
            path: [
              { x: 420, y: 300 },
              { x: 420, y: 220 },
              { x: 440, y: 160 },
              { x: 420, y: 100 },
              { x: 380, y: 60 },
              { x: 330, y: 40 },
              { x: 280, y: 35 },
              { x: 200, y: 35 },
            ],
          },
          {
            label: "return up the outer left side",
            path: [
              { x: 200, y: 35 },
              { x: 125, y: 50 },
              { x: 75, y: 100 },
              { x: 65, y: 160 },
              { x: 80, y: 225 },
              { x: 115, y: 275 },
              { x: 160, y: 300 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "turn around the lower right bowl",
            path: [
              { x: 420, y: 300 },
              { x: 520, y: 300 },
              { x: 620, y: 285 },
              { x: 680, y: 245 },
              { x: 710, y: 185 },
              { x: 705, y: 120 },
              { x: 675, y: 70 },
              { x: 620, y: 35 },
              { x: 555, y: 25 },
              { x: 510, y: 40 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 3, க (Univ. of Texas at Austin), p. 191",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
];
