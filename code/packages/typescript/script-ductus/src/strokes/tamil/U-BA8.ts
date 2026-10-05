import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ந",
  {
    script: "tamil",
    glyph: "ந",
    strokes: [
      {
        segments: [
          {
            label: "draw the left upright upward",
            path: [
              { x: 131, y: 43 },
              { x: 131, y: 109 },
              { x: 131, y: 176 },
              { x: 131, y: 242 },
              { x: 131, y: 308 },
              { x: 131, y: 375 },
              { x: 131, y: 441 },
              { x: 135, y: 507 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 135, y: 507 },
              { x: 208, y: 515 },
              { x: 282, y: 519 },
              { x: 356, y: 516 },
              { x: 427, y: 514 },
              { x: 501, y: 518 },
              { x: 575, y: 519 },
            ],
          },
          {
            label: "return left to the middle",
            path: [
              { x: 575, y: 519 },
              { x: 512, y: 519 },
              { x: 449, y: 519 },
              { x: 391, y: 499 },
            ],
          },
          {
            label: "descend the middle upright",
            path: [
              { x: 391, y: 499 },
              { x: 391, y: 434 },
              { x: 391, y: 369 },
              { x: 391, y: 304 },
              { x: 391, y: 238 },
              { x: 391, y: 173 },
              { x: 391, y: 108 },
              { x: 391, y: 43 },
            ],
          },
          {
            label: "back up, round the right bowl",
            path: [
              { x: 391, y: 43 },
              { x: 391, y: 125 },
              { x: 391, y: 206 },
              { x: 399, y: 287 },
              { x: 466, y: 305 },
              { x: 532, y: 323 },
              { x: 601, y: 315 },
              { x: 654, y: 272 },
              { x: 688, y: 212 },
              { x: 698, y: 143 },
              { x: 694, y: 74 },
              { x: 674, y: 8 },
              { x: 630, y: -46 },
              { x: 574, y: -86 },
              { x: 508, y: -108 },
              { x: 439, y: -116 },
              { x: 370, y: -118 },
              { x: 301, y: -121 },
              { x: 231, y: -121 },
            ],
          },
          {
            label: "sweep the low tail left",
            path: [
              { x: 231, y: -121 },
              { x: 153, y: -139 },
              { x: 101, y: -198 },
              { x: 91, y: -277 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 5, ந (University of Texas at Austin), p. 193",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Module 5 identifies ந as the voiced dental nasal and notes that its extended final curve may be omitted. Appendix I Frame 5 numbers six hand-movements. Native writers draw them as one continuous stroke, and this ductus follows the six movements in order without lifting: the pen climbs back up the middle upright to where the right bowl leaves it. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 85% of the 224 stored prototypes of ந are a single pen-down stroke. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
