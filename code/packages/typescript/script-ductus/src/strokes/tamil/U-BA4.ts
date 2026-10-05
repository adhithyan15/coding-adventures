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
              { x: 203, y: 291 },
              { x: 199, y: 366 },
              { x: 199, y: 440 },
              { x: 199, y: 515 },
            ],
          },
          {
            label: "carry the top bar to the middle",
            path: [
              { x: 199, y: 515 },
              { x: 260, y: 515 },
              { x: 322, y: 519 },
              { x: 383, y: 519 },
              { x: 443, y: 507 },
            ],
          },
          {
            label: "carry the short upper bar right",
            path: [
              { x: 443, y: 507 },
              { x: 511, y: 515 },
              { x: 579, y: 519 },
              { x: 647, y: 519 },
            ],
          },
          {
            label: "back and down the right bowl",
            path: [
              { x: 647, y: 519 },
              { x: 579, y: 519 },
              { x: 510, y: 518 },
              { x: 443, y: 507 },
              { x: 451, y: 437 },
              { x: 450, y: 366 },
              { x: 455, y: 297 },
              { x: 525, y: 291 },
              { x: 594, y: 279 },
              { x: 659, y: 250 },
              { x: 709, y: 199 },
              { x: 731, y: 133 },
              { x: 731, y: 61 },
              { x: 707, y: -5 },
            ],
          },
          {
            label: "back up, round the left loop",
            path: [
              { x: 707, y: -5 },
              { x: 730, y: 63 },
              { x: 731, y: 135 },
              { x: 707, y: 202 },
              { x: 655, y: 252 },
              { x: 589, y: 279 },
              { x: 518, y: 291 },
              { x: 447, y: 299 },
              { x: 387, y: 291 },
              { x: 326, y: 291 },
              { x: 264, y: 291 },
              { x: 203, y: 291 },
              { x: 146, y: 255 },
              { x: 103, y: 204 },
              { x: 99, y: 136 },
              { x: 123, y: 75 },
              { x: 180, y: 39 },
              { x: 246, y: 27 },
              { x: 314, y: 28 },
              { x: 378, y: 49 },
              { x: 427, y: 95 },
            ],
          },
          {
            label: "curl back to the centre",
            path: [
              { x: 427, y: 95 },
              { x: 449, y: 160 },
              { x: 447, y: 230 },
              { x: 447, y: 299 },
            ],
          },
          {
            label: "the bowl again, then the tail",
            path: [
              { x: 447, y: 299 },
              { x: 515, y: 291 },
              { x: 584, y: 281 },
              { x: 650, y: 257 },
              { x: 700, y: 209 },
              { x: 730, y: 147 },
              { x: 731, y: 77 },
              { x: 715, y: 10 },
              { x: 669, y: -43 },
              { x: 615, y: -86 },
              { x: 548, y: -106 },
              { x: 479, y: -115 },
              { x: 409, y: -117 },
              { x: 339, y: -117 },
              { x: 269, y: -117 },
              { x: 200, y: -125 },
              { x: 135, y: -150 },
              { x: 97, y: -208 },
              { x: 91, y: -277 },
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
        "Module 3 identifies த as the dental stop and asks learners to write it. Appendix I's final Frame 3 row numbers seven hand-movements: 1–2 for the upper frame, 3–4 for the broad right bowl, 5–6 for the compact left loop, and 7 for the leftward tail. Native writers draw த as one continuous stroke, and this ductus follows the seven movements in order without lifting. Because the manual numbers the right bowl before the left loop, the pen retraces the right bowl to reach the loop and again to reach the tail. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 88% of the 266 stored prototypes of த are a single pen-down stroke. Tamil handwriting varies by school; this is one attested order fitted to the bundled Noto Sans Tamil outline.",
    },
  },
];
