import type { DuctusEntry } from "../registry.ts";

export const entry: DuctusEntry = [
  "எ",
  {
    script: "tamil",
    glyph: "எ",
    strokes: [
      {
        segments: [
          {
            label: "curl round the inner bowl",
            path: [
              { x: 109, y: 256 },
              { x: 179, y: 275 },
              { x: 249, y: 288 },
              { x: 318, y: 274 },
              { x: 368, y: 221 },
              { x: 381, y: 150 },
              { x: 359, y: 83 },
              { x: 305, y: 34 },
              { x: 234, y: 24 },
              { x: 167, y: 48 },
              { x: 121, y: 104 },
            ],
          },
          {
            label: "climb the outer left side",
            path: [
              { x: 121, y: 104 },
              { x: 102, y: 167 },
              { x: 102, y: 233 },
              { x: 105, y: 299 },
              { x: 122, y: 362 },
              { x: 158, y: 417 },
              { x: 205, y: 464 },
            ],
          },
          {
            label: "carry the top bar to its right end",
            path: [
              { x: 205, y: 464 },
              { x: 267, y: 494 },
              { x: 335, y: 510 },
              { x: 404, y: 516 },
              { x: 473, y: 520 },
              { x: 542, y: 515 },
              { x: 611, y: 515 },
              { x: 680, y: 516 },
              { x: 749, y: 520 },
            ],
          },
          {
            label: "back to the upright, then down it",
            path: [
              { x: 749, y: 520 },
              { x: 689, y: 520 },
              { x: 629, y: 519 },
              { x: 569, y: 520 },
              { x: 569, y: 448 },
              { x: 569, y: 376 },
              { x: 569, y: 304 },
              { x: 569, y: 232 },
              { x: 569, y: 160 },
              { x: 569, y: 88 },
              { x: 569, y: 16 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Sankaran Radhakrishnan, Tamil Script Learners Manual, Appendix I: Hand-movements, Frame 5, எ (University of Texas at Austin), p. 193; reordered, and drawn in one stroke, after HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 6 (எ): stored online-handwriting prototypes from native writers (MIT licence, 2012)",
      url: "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      variation:
        "Appendix I Frame 5 numbers seven movements for எ: it starts on the outer left side and draws the right upright last, upward from its foot, after a lift. Native writers draw எ in one stroke and in another order. In HP Labs India's online Tamil handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained on hpl-tamil-iso-char), 81 of the 88 stored prototypes of எ (92%) are one pen-down stroke. 67 of the 81 (83%) start inside the bowl, all 81 turn it clockwise first, and 79 (98%) then reach the top bar and draw the right upright downward to the end; 61 (75%) end at the bottom right. So this ductus overrides Frame 5's order and its lift: it starts at the inner end of the curl, goes clockwise round the bowl, up the outer left side and along the top bar, then draws the upright down. The bundled Noto Sans Tamil outline carries the top bar past the upright, so the pen runs to the bar's right end and comes back along it to the upright. The traces are scaled to a square, so they fix the stroke count, the start, the order and the direction, not the proportions: the path is fitted to the bundled Noto Sans Tamil outline. The underlying HP Labs data is licensed for research use only, so only counts and shares are cited here, and no trace was copied. Tamil handwriting varies by school.",
    },
  },
];
