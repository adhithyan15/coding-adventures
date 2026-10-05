import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ள",
  {
    script: "tamil",
    glyph: "ள",
    strokes: [
      {
        segments: [
          {
            label: "curl round the bowl and up the outer loop",
            path: [
              { x: 105, y: 264 },
              { x: 175, y: 273 },
              { x: 244, y: 288 },
              { x: 313, y: 277 },
              { x: 364, y: 227 },
              { x: 381, y: 159 },
              { x: 365, y: 90 },
              { x: 314, y: 39 },
              { x: 246, y: 24 },
              { x: 178, y: 42 },
              { x: 129, y: 92 },
              { x: 105, y: 159 },
              { x: 105, y: 231 },
              { x: 109, y: 302 },
              { x: 121, y: 372 },
            ],
          },
          {
            label: "curve over the top into the junction",
            path: [
              { x: 121, y: 372 },
              { x: 159, y: 436 },
              { x: 214, y: 487 },
              { x: 281, y: 518 },
              { x: 355, y: 528 },
              { x: 429, y: 521 },
              { x: 498, y: 492 },
              { x: 565, y: 460 },
            ],
          },
          {
            label: "draw the adjoining stem down",
            path: [
              { x: 565, y: 460 },
              { x: 577, y: 391 },
              { x: 577, y: 321 },
              { x: 581, y: 251 },
              { x: 581, y: 181 },
              { x: 581, y: 110 },
              { x: 581, y: 40 },
            ],
          },
          {
            label: "rise back up the same stem",
            path: [
              { x: 581, y: 40 },
              { x: 581, y: 108 },
              { x: 581, y: 176 },
              { x: 581, y: 244 },
              { x: 581, y: 312 },
              { x: 581, y: 380 },
              { x: 581, y: 448 },
              { x: 581, y: 516 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 581, y: 516 },
              { x: 653, y: 516 },
              { x: 724, y: 516 },
              { x: 796, y: 516 },
              { x: 866, y: 513 },
              { x: 937, y: 516 },
              { x: 1009, y: 516 },
            ],
          },
          {
            label: "back, then down the right upright",
            path: [
              { x: 1009, y: 516 },
              { x: 939, y: 516 },
              { x: 869, y: 515 },
              { x: 841, y: 459 },
              { x: 841, y: 389 },
              { x: 841, y: 319 },
              { x: 841, y: 250 },
              { x: 841, y: 180 },
              { x: 841, y: 110 },
              { x: 841, y: 40 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 12, ள (University of Texas at Austin), p. 195",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Module 12 identifies ள as the retroflex lateral, contrasts it with ல, and directs learners to Appendix I. Frame 12 numbers six hand-movements. Native writers draw them as one continuous stroke, and this ductus follows the six movements in order without lifting. The bundled Noto Sans Tamil outline joins the loop to the middle upright only at the top and prints the adjoining stem and the rising middle upright as one stem, so the pen starts inside the loop, curls round its inner bowl before climbing the outer loop, and climbs back up that shared stem. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 90% of the 264 stored prototypes of ள are a single pen-down stroke. Tamil handwriting varies by school; this is one attested order.",
    },
  },
];
