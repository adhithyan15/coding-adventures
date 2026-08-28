import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ம",
  {
    script: "tamil",
    glyph: "ம",
    strokes: [
      {
        segments: [
          {
            label: "down the left upright",
            path: [
              { x: 110, y: 548 },
              { x: 104, y: 120 },
              { x: 104, y: 40 },
            ],
          },
          {
            label: "along the bottom",
            path: [
              { x: 104, y: 40 },
              { x: 250, y: 26 },
              { x: 430, y: 24 },
              { x: 600, y: 30 },
              { x: 715, y: 44 },
            ],
          },
          {
            label: "up the right side",
            path: [
              { x: 715, y: 44 },
              { x: 726, y: 180 },
              { x: 724, y: 430 },
            ],
          },
          {
            label: "over the top",
            path: [
              { x: 724, y: 430 },
              { x: 690, y: 510 },
              { x: 600, y: 548 },
              { x: 500, y: 548 },
              { x: 452, y: 512 },
              { x: 434, y: 430 },
            ],
          },
          {
            label: "down the middle",
            path: [
              { x: 434, y: 430 },
              { x: 430, y: 180 },
              { x: 430, y: 44 },
            ],
          },
        ],
      },
    ],
    // Five movements, matching the numbered arrows in the cited primer's
    // Frame 1 (down the left · along the bottom · up the right · over the top ·
    // down the middle) — one unbroken pen path, exactly as authored.
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 1 (Univ. of Texas at Austin)",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. This is one attested order.",
    },
  },
];
