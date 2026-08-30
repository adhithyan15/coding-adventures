import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ஐ",
  {
    script: "tamil",
    glyph: "ஐ",
    strokes: [
      {
        segments: [
          {
            label: "curl inward around the upper-left spiral",
            path: [
              { x: 400, y: 490 },
              { x: 320, y: 535 },
              { x: 220, y: 510 },
              { x: 155, y: 445 },
              { x: 125, y: 350 },
              { x: 155, y: 260 },
              { x: 225, y: 215 },
              { x: 310, y: 220 },
              { x: 375, y: 275 },
              { x: 390, y: 350 },
              { x: 355, y: 415 },
              { x: 285, y: 440 },
              { x: 225, y: 415 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "draw the central upright upward",
            path: [
              { x: 555, y: 210 },
              { x: 555, y: 250 },
              { x: 555, y: 290 },
              { x: 555, y: 315 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "sweep around the upper-right loop and return left across the middle",
            path: [
              { x: 555, y: 315 },
              { x: 545, y: 400 },
              { x: 570, y: 470 },
              { x: 650, y: 525 },
              { x: 760, y: 530 },
              { x: 860, y: 470 },
              { x: 915, y: 380 },
              { x: 915, y: 285 },
              { x: 875, y: 200 },
              { x: 800, y: 135 },
              { x: 700, y: 95 },
              { x: 575, y: 95 },
              { x: 400, y: 95 },
              { x: 225, y: 100 },
              { x: 160, y: 85 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "circle the lower-left bowl back toward the centre",
            path: [
              { x: 160, y: 85 },
              { x: 105, y: 10 },
              { x: 105, y: -100 },
              { x: 165, y: -215 },
              { x: 275, y: -285 },
              { x: 370, y: -285 },
              { x: 445, y: -290 },
              { x: 500, y: -255 },
              { x: 535, y: -190 },
              { x: 550, y: -100 },
              { x: 555, y: -20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "descend and circle through the lower-right bowl",
            path: [
              { x: 555, y: 80 },
              { x: 555, y: -45 },
              { x: 585, y: -170 },
              { x: 650, y: -255 },
              { x: 750, y: -285 },
              { x: 850, y: -245 },
              { x: 915, y: -165 },
              { x: 920, y: -70 },
              { x: 890, y: 20 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Info-farmer, Writing Tamil 10.gif, ஐ stroke-order animation (Wikimedia Commons, CC BY-SA 3.0, 2008), cross-checked with Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I Frame 11 (University of Texas at Austin), p. 194",
      url: "https://commons.wikimedia.org/wiki/File:Writing_Tamil_10.gif",
      variation:
        "The 13-frame animation writes five separate runs in order: upper-left spiral, upward central upright, upper-right loop returning left across the middle, lower-left bowl, then lower-right bowl. Radhakrishnan's Appendix I Frame 11 independently numbers the same shape as seven movements. This five-run learner path fits those movements to the bundled Noto Sans Tamil outline; Tamil handwriting varies by school.",
    },
  },
];
