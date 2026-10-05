import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ஊ",
  {
    script: "tamil",
    glyph: "ஊ",
    strokes: [
      {
        segments: [
          {
            label: "sweep outward around the spiral",
            path: [
              { x: 321, y: 460 },
              { x: 341, y: 399 },
              { x: 326, y: 335 },
              { x: 277, y: 290 },
              { x: 212, y: 280 },
              { x: 151, y: 302 },
              { x: 107, y: 352 },
              { x: 98, y: 418 },
              { x: 129, y: 476 },
              { x: 182, y: 515 },
              { x: 247, y: 528 },
              { x: 313, y: 520 },
            ],
          },
          {
            label: "down the outer curve onto the baseline",
            path: [
              { x: 313, y: 520 },
              { x: 381, y: 524 },
              { x: 446, y: 507 },
              { x: 504, y: 472 },
              { x: 543, y: 418 },
              { x: 553, y: 351 },
              { x: 547, y: 284 },
              { x: 502, y: 233 },
              { x: 445, y: 199 },
              { x: 379, y: 182 },
              { x: 312, y: 172 },
              { x: 244, y: 165 },
              { x: 177, y: 155 },
              { x: 119, y: 122 },
              { x: 117, y: 60 },
            ],
          },
          {
            label: "carry the baseline to the right",
            path: [
              { x: 117, y: 60 },
              { x: 307, y: 36 },
              { x: 503, y: 36 },
              { x: 698, y: 36 },
              { x: 894, y: 36 },
              { x: 1090, y: 36 },
              { x: 1285, y: 36 },
              { x: 1481, y: 36 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "curl round the bowl and up the outer loop",
            path: [
              { x: 737, y: 352 },
              { x: 804, y: 351 },
              { x: 871, y: 360 },
              { x: 932, y: 333 },
              { x: 957, y: 271 },
              { x: 939, y: 207 },
              { x: 883, y: 169 },
              { x: 815, y: 172 },
              { x: 762, y: 216 },
              { x: 741, y: 280 },
              { x: 737, y: 349 },
              { x: 749, y: 416 },
            ],
          },
          {
            label: "curve over the top into the junction",
            path: [
              { x: 749, y: 416 },
              { x: 787, y: 474 },
              { x: 844, y: 512 },
              { x: 912, y: 528 },
              { x: 981, y: 527 },
              { x: 1046, y: 503 },
              { x: 1109, y: 476 },
            ],
          },
          {
            label: "draw the adjoining stem down",
            path: [
              { x: 1109, y: 476 },
              { x: 1117, y: 374 },
              { x: 1117, y: 271 },
              { x: 1117, y: 168 },
            ],
          },
          {
            label: "rise back up the same stem",
            path: [
              { x: 1117, y: 168 },
              { x: 1117, y: 256 },
              { x: 1117, y: 344 },
              { x: 1117, y: 432 },
              { x: 1117, y: 520 },
            ],
          },
          {
            label: "carry the top bar right",
            path: [
              { x: 1117, y: 520 },
              { x: 1189, y: 520 },
              { x: 1261, y: 520 },
              { x: 1333, y: 520 },
              { x: 1405, y: 520 },
              { x: 1477, y: 520 },
            ],
          },
          {
            label: "back, then down the right upright",
            path: [
              { x: 1477, y: 520 },
              { x: 1407, y: 520 },
              { x: 1337, y: 520 },
              { x: 1337, y: 450 },
              { x: 1337, y: 379 },
              { x: 1337, y: 309 },
              { x: 1337, y: 238 },
              { x: 1337, y: 168 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Module 17, ஊ construction, with Appendix I: Hand-movements, Frames 17, 16, and 12 (University of Texas at Austin), pp. 195–196",
      url: "https://sites.la.utexas.edu/tamilscript/frame-17/92",
      variation:
        "Module 17 identifies ஊ as long ū and explicitly constructs it from two familiar letters: write உ first, then write ள over it. Appendix I Frame 16 numbers உ's three movements and Frame 12 numbers ள's six. Native writers draw each part as one continuous stroke and lift once between them, so this ductus writes உ without lifting, lifts once, and writes ள without lifting, starting inside its loop and climbing back up the shared middle stem as ள itself does. The bundled Noto Sans Tamil ஊ outline does not join the ள part to உ's baseline. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 93% of the 73 stored prototypes of ஊ are two pen-down strokes. This two-run learner order is fitted to the bundled Noto Sans Tamil ஊ outline; Tamil handwriting varies by school.",
    },
  },
];
