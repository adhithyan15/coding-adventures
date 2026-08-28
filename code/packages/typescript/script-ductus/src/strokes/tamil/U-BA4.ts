import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "த",
  {
    script: "tamil",
    glyph: "த",
    strokes: [
      {
        segments: [
          {
            label: "climb the short left upright",
            path: [
              { x: 160, y: 286 },
              { x: 160, y: 420 },
              { x: 160, y: 548 },
            ],
          },
          {
            label: "carry the top bar to the right",
            path: [
              { x: 160, y: 548 },
              { x: 285, y: 518 },
              { x: 412, y: 518 },
              { x: 540, y: 518 },
              { x: 680, y: 518 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "carry the short upper bar right",
            path: [
              { x: 412, y: 518 },
              { x: 540, y: 518 },
              { x: 680, y: 518 },
            ],
          },
          {
            label: "curve down around the broad right bowl",
            path: [
              { x: 680, y: 518 },
              { x: 540, y: 518 },
              { x: 412, y: 518 },
              { x: 412, y: 360 },
              { x: 412, y: 300 },
              { x: 530, y: 290 },
              { x: 630, y: 255 },
              { x: 690, y: 190 },
              { x: 700, y: 90 },
              { x: 675, y: 5 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "turn around the compact left loop",
            path: [
              { x: 160, y: 286 },
              { x: 105, y: 260 },
              { x: 80, y: 205 },
              { x: 90, y: 125 },
              { x: 140, y: 65 },
              { x: 240, y: 25 },
              { x: 340, y: 40 },
              { x: 400, y: 105 },
            ],
          },
          {
            label: "curl back to the central crossing",
            path: [
              { x: 400, y: 105 },
              { x: 430, y: 135 },
              { x: 440, y: 170 },
              { x: 430, y: 200 },
              { x: 412, y: 205 },
              { x: 412, y: 300 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "sweep the low tail left",
            path: [
              { x: 675, y: 5 },
              { x: 610, y: -55 },
              { x: 510, y: -95 },
              { x: 370, y: -115 },
              { x: 230, y: -115 },
              { x: 130, y: -135 },
              { x: 80, y: -205 },
              { x: 90, y: -315 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 3, த (University of Texas at Austin), p. 192",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Module 3 identifies த as the dental stop and asks learners to write it. Appendix I's final Frame 3 row numbers four separate pen-down runs: joined movements 1–2 for the upper frame, joined 3–4 for the right bowl, joined 5–6 for the left loop, and separate movement 7 for the leftward tail. Tamil handwriting varies by school; this is one attested four-run order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
