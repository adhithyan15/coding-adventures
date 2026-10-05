import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ஒ",
  {
    script: "tamil",
    glyph: "ஒ",
    strokes: [
      {
        segments: [
          {
            label: "circle the small loop into the crown",
            path: [
              { x: 150, y: 160 },
              { x: 220, y: 105 },
              { x: 305, y: 120 },
              { x: 345, y: 180 },
              { x: 350, y: 255 },
              { x: 345, y: 310 },
              { x: 290, y: 350 },
              { x: 215, y: 350 },
              { x: 140, y: 315 },
              { x: 95, y: 255 },
              { x: 95, y: 190 },
              { x: 135, y: 125 },
            ],
          },
          {
            label: "sweep the large loop and curl in",
            path: [
              { x: 135, y: 125 },
              { x: 80, y: 300 },
              { x: 180, y: 430 },
              { x: 360, y: 515 },
              { x: 500, y: 530 },
              { x: 675, y: 505 },
              { x: 810, y: 435 },
              { x: 885, y: 330 },
              { x: 895, y: 225 },
              { x: 860, y: 135 },
              { x: 790, y: 100 },
              { x: 720, y: 105 },
              { x: 670, y: 150 },
              { x: 660, y: 220 },
              { x: 680, y: 285 },
              { x: 655, y: 325 },
              { x: 600, y: 275 },
              { x: 550, y: 210 },
              { x: 520, y: 140 },
              { x: 520, y: 60 },
              { x: 600, y: -20 },
              { x: 720, y: -65 },
              { x: 850, y: -70 },
            ],
          },
          {
            label: "back, then round the lower bowl",
            path: [
              { x: 850, y: -70 },
              { x: 773, y: -72 },
              { x: 705, y: -69 },
              { x: 655, y: -104 },
              { x: 622, y: -162 },
              { x: 579, y: -214 },
              { x: 528, y: -257 },
              { x: 464, y: -279 },
              { x: 397, y: -281 },
              { x: 331, y: -270 },
              { x: 279, y: -228 },
              { x: 253, y: -167 },
              { x: 256, y: -100 },
              { x: 280, y: -37 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Module 14, ஒ, with Appendix I: Hand-movements, Frame 14 (University of Texas at Austin), p. 195",
      url: "https://sites.la.utexas.edu/tamilscript/category/3-moduals/module-14",
      variation:
        "Module 14 identifies ஒ as short o. Appendix I Frame 14 numbers three movements: the small left loop, the large right loop, and the lower bowl. Native writers draw them as one continuous stroke, and this ductus follows the three movements in order without lifting: the large loop ends at the tip of the tail, and the bundled Noto Sans Tamil outline joins the top of the lower bowl to that tail, so the pen comes back along the tail before drawing the bowl down and around. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 99% of the 115 stored prototypes of ஒ are a single pen-down stroke. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
