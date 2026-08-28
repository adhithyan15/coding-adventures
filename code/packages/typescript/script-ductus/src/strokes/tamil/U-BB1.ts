import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ற",
  {
    script: "tamil",
    glyph: "ற",
    strokes: [
      {
        segments: [
          {
            label: "climb the left and arch to the middle",
            path: [
              { x: 105, y: 40 },
              { x: 105, y: 200 },
              { x: 105, y: 400 },
              { x: 135, y: 485 },
              { x: 215, y: 535 },
              { x: 300, y: 530 },
              { x: 370, y: 485 },
              { x: 405, y: 430 },
            ],
          },
          {
            label: "descend the first middle upright",
            path: [
              { x: 405, y: 430 },
              { x: 405, y: 300 },
              { x: 405, y: 150 },
              { x: 405, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend the second middle upright",
            path: [
              { x: 405, y: 430 },
              { x: 405, y: 300 },
              { x: 405, y: 150 },
              { x: 405, y: 35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "arch over and descend the right side",
            path: [
              { x: 420, y: 450 },
              { x: 455, y: 510 },
              { x: 540, y: 540 },
              { x: 640, y: 520 },
              { x: 710, y: 460 },
              { x: 755, y: 380 },
              { x: 760, y: 260 },
              { x: 760, y: 130 },
              { x: 750, y: 50 },
            ],
          },
          {
            label: "sweep left below the baseline and down",
            path: [
              { x: 750, y: 50 },
              { x: 730, y: 0 },
              { x: 690, y: -40 },
              { x: 620, y: -75 },
              { x: 520, y: -100 },
              { x: 400, y: -115 },
              { x: 280, y: -120 },
              { x: 160, y: -125 },
              { x: 105, y: -155 },
              { x: 105, y: -230 },
              { x: 105, y: -315 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 10, ற (Univ. of Texas at Austin), p. 194",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
];
