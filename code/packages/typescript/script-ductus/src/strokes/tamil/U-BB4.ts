import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ழ",
  {
    script: "tamil",
    glyph: "ழ",
    strokes: [
      {
        segments: [
          {
            label: "climb the outer left upright",
            path: [
              { x: 130, y: 36 },
              { x: 130, y: 180 },
              { x: 130, y: 360 },
              { x: 130, y: 525 },
            ],
          },
          {
            label: "retrace down the left upright",
            path: [
              { x: 130, y: 525 },
              { x: 130, y: 360 },
              { x: 130, y: 180 },
              { x: 130, y: 36 },
            ],
          },
          {
            label: "carry the low crossbar right",
            path: [
              { x: 130, y: 36 },
              { x: 260, y: 36 },
              { x: 400, y: 36 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "retrace left into the inner upright",
            path: [
              { x: 620, y: 525 },
              { x: 550, y: 525 },
              { x: 485, y: 500 },
              { x: 445, y: 450 },
            ],
          },
          {
            label: "descend and sweep around the broad right bowl",
            path: [
              { x: 445, y: 450 },
              { x: 445, y: 300 },
              { x: 445, y: 150 },
              { x: 445, y: 36 },
              { x: 540, y: 36 },
              { x: 640, y: 55 },
              { x: 715, y: 115 },
              { x: 750, y: 220 },
              { x: 750, y: 340 },
              { x: 715, y: 440 },
              { x: 650, y: 510 },
              { x: 600, y: 525 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "turn through the detached lower hook",
            path: [
              { x: 250, y: -95 },
              { x: 175, y: -120 },
              { x: 110, y: -175 },
              { x: 105, y: -215 },
              { x: 145, y: -260 },
              { x: 235, y: -285 },
              { x: 330, y: -285 },
              { x: 420, y: -255 },
              { x: 490, y: -205 },
              { x: 535, y: -150 },
              { x: 590, y: -130 },
              { x: 680, y: -130 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 7, ழ (University of Texas at Austin), p. 193",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 7 numbers six movements in three pen-down runs: joined movements 1–3 form the left body and bar, joined movements 4–5 retrace into the inner upright and broad right bowl, and separate movement 6 forms the detached lower hook. The bundled Noto Sans Tamil print face simplifies the source's looped left body and high bar into a retraced upright with a low crossbar; the authored path preserves the source's run grouping while fitting that printed outline. Tamil handwriting varies by school; this is one attested three-run order.",
    },
  },
];
