import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "ங",
  {
    script: "tamil",
    glyph: "ங",
    strokes: [
      {
        segments: [
          {
            label: "draw the left upright down",
            path: [
              { x: 132, y: 504 },
              { x: 132, y: 436 },
              { x: 132, y: 368 },
              { x: 132, y: 300 },
              { x: 132, y: 232 },
              { x: 132, y: 164 },
              { x: 132, y: 96 },
              { x: 132, y: 28 },
            ],
          },
          {
            label: "climb back up it, then the top bar",
            path: [
              { x: 132, y: 28 },
              { x: 132, y: 98 },
              { x: 132, y: 167 },
              { x: 132, y: 237 },
              { x: 132, y: 307 },
              { x: 132, y: 377 },
              { x: 132, y: 446 },
              { x: 132, y: 516 },
              { x: 207, y: 516 },
              { x: 283, y: 516 },
              { x: 358, y: 516 },
              { x: 433, y: 516 },
              { x: 509, y: 516 },
              { x: 584, y: 516 },
            ],
          },
          {
            label: "back along the bar, down the stem",
            path: [
              { x: 584, y: 516 },
              { x: 519, y: 516 },
              { x: 453, y: 516 },
              { x: 388, y: 516 },
              { x: 388, y: 446 },
              { x: 388, y: 375 },
              { x: 388, y: 305 },
              { x: 388, y: 234 },
              { x: 388, y: 164 },
            ],
          },
          {
            label: "back up, then round the bowl",
            path: [
              { x: 388, y: 164 },
              { x: 388, y: 235 },
              { x: 417, y: 289 },
              { x: 484, y: 312 },
              { x: 553, y: 324 },
              { x: 619, y: 306 },
              { x: 665, y: 254 },
              { x: 672, y: 184 },
              { x: 660, y: 115 },
              { x: 640, y: 48 },
            ],
          },
          {
            label: "left along the low bar to its end",
            path: [
              { x: 640, y: 48 },
              { x: 572, y: 36 },
              { x: 503, y: 36 },
              { x: 433, y: 36 },
              { x: 364, y: 36 },
            ],
          },
          {
            label: "back along it and up the right upright",
            path: [
              { x: 364, y: 36 },
              { x: 432, y: 36 },
              { x: 501, y: 36 },
              { x: 570, y: 36 },
              { x: 638, y: 36 },
              { x: 706, y: 36 },
              { x: 775, y: 36 },
              { x: 844, y: 36 },
              { x: 912, y: 36 },
              { x: 912, y: 105 },
              { x: 912, y: 174 },
              { x: 912, y: 243 },
              { x: 912, y: 313 },
              { x: 912, y: 382 },
              { x: 912, y: 451 },
              { x: 912, y: 520 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 2, ங (University of Texas at Austin), p. 191; reordered, and drawn in one stroke, after HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 13 (ங): stored online-handwriting prototypes from native writers (MIT licence, 2012)",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 2 draws ங's right upright first, downward, as a detached part, and then the body after a lift. Native writers draw ங in one stroke and draw that upright last. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 92 of the 108 stored prototypes of ங (85%) are one pen-down stroke. 75 of the 92 (82%) start at the upper left, and 69 (75%) go down the left upright and climb back up it before moving right; 81 (88%) draw the right upright upward at the end, and 73 (79%) end at the top right. So this ductus overrides Frame 2's order and its lift. The bundled Noto Sans Tamil outline joins the right upright to the low bar at its foot, carries the top bar past the inner stem, hangs the bowl from the stem above the stem's free foot, and carries the low bar left under the stem, so the pen goes down the left upright and back up it, out along the top bar and back to the inner stem, down the stem to its foot and back up into the bowl, round the bowl to the low bar, out to the low bar's left end and back, and up the right upright. The traces are scaled to a square, so they fix the stroke count, the start, the order and the direction, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline. The underlying HP Labs data is licensed for research use only, so only counts and shares are cited here, and no trace was copied. Tamil handwriting varies by school.",
    },
  },
];
