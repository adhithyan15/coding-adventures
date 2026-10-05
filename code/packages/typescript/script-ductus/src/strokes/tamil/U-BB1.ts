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
            label: "climb and arch to the middle",
            path: [
              { x: 129, y: 43 },
              { x: 129, y: 110 },
              { x: 129, y: 177 },
              { x: 129, y: 244 },
              { x: 129, y: 311 },
              { x: 129, y: 378 },
              { x: 140, y: 444 },
              { x: 182, y: 496 },
              { x: 240, y: 527 },
              { x: 306, y: 520 },
              { x: 355, y: 475 },
              { x: 397, y: 423 },
            ],
          },
          {
            label: "descend the middle upright",
            path: [
              { x: 397, y: 423 },
              { x: 395, y: 347 },
              { x: 393, y: 271 },
              { x: 393, y: 195 },
              { x: 393, y: 119 },
              { x: 393, y: 43 },
            ],
          },
          {
            label: "climb back up the same upright",
            path: [
              { x: 393, y: 43 },
              { x: 393, y: 119 },
              { x: 393, y: 195 },
              { x: 393, y: 271 },
              { x: 393, y: 347 },
              { x: 397, y: 423 },
            ],
          },
          {
            label: "arch over, down the right side",
            path: [
              { x: 397, y: 423 },
              { x: 444, y: 476 },
              { x: 500, y: 518 },
              { x: 569, y: 527 },
              { x: 638, y: 516 },
              { x: 692, y: 472 },
              { x: 733, y: 414 },
              { x: 751, y: 346 },
              { x: 757, y: 276 },
              { x: 761, y: 206 },
              { x: 754, y: 135 },
              { x: 737, y: 67 },
            ],
          },
          {
            label: "sweep left and drop",
            path: [
              { x: 737, y: 67 },
              { x: 708, y: 4 },
              { x: 662, y: -48 },
              { x: 603, y: -84 },
              { x: 537, y: -105 },
              { x: 469, y: -115 },
              { x: 400, y: -117 },
              { x: 330, y: -117 },
              { x: 261, y: -121 },
              { x: 192, y: -126 },
              { x: 132, y: -158 },
              { x: 97, y: -216 },
              { x: 97, y: -285 },
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
        "Tamil handwriting is taught with school-to-school variation; there is no single national stroke-order standard. Appendix I Frame 10 numbers five hand-movements, with movement 3 on the middle upright beside movement 2. Native writers draw them as one continuous stroke, and this ductus follows the five movements in order without lifting. The bundled Noto Sans Tamil outline prints a single shared middle upright, so movement 3 climbs back up it to start the second arch. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 99% of the 378 stored prototypes of ற are a single pen-down stroke.",
    },
  },
];
