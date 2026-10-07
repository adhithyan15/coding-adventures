import type { DuctusEntry } from "../registry.ts";

// The puḷḷi (U+0BCD) is the dot that removes a consonant's inherent vowel:
// க is "ka", க் is a bare "k". It is one touch of the pen, made after the
// consonant's body is complete, so it is a mark of its own here, like the six
// vowel signs, and `WRITTEN_SIGN_SIDES` places it AFTER its consonant.
//
// The bundled Noto Sans Tamil draws the bare sign as a filled disc of radius
// 67 font units centred at (-278, 745): a zero-width mark that hangs back over
// the letter before it. Every consonant + puḷḷi cluster the font prints is a
// composite of the unchanged consonant glyph and this same disc, shifted
// right, so this path also lies on the dot of every printed cluster.
//
//          ●   <- a short press inside the disc, top to bottom
//
// The path stays well inside the disc (27 units from its centre at most), so
// all of it lies on ink, and every inked point of the disc is within reach of
// it: nothing of the printed sign is left untraced.
export const entry: DuctusEntry = [
  "்",
  {
    script: "tamil",
    glyph: "்",
    strokes: [
      {
        segments: [
          {
            label: "dab the dot",
            path: [
              { x: -278, y: 772 },
              { x: -278, y: 758 },
              { x: -278, y: 745 },
              { x: -278, y: 732 },
              { x: -278, y: 718 },
            ],
          },
        ],
      },
    ],
    source: {
      citation:
        "Abhinaya Rajarajan, Varai (Swift Student Challenge 2026), reference drawings recorded by hand for the 18 Tamil consonants with pulli, க் to ன் (repository commit 952294fa, no licence: facts only)",
      url: "https://github.com/abhinayaRajarajan/varai",
      variation:
        "One writer's recorded reference drawings, stored in the order they were drawn. In all 18 the consonant body is one stroke and the puḷḷi is a separate second stroke, made after the body is complete, so it costs one pen lift. Its centre sits above the body at 0.47 to 0.65 of the body's width (0.54 for க்). An earlier reading of Info-farmer's Writing Tamil animations of the consonants on Wikimedia Commons, which show each consonant with its puḷḷi, also places the dot after the body. Confidence is medium: one recorded writer, and only stroke count, order and the dot's bounding box were read; the dot is about 10 by 13 canvas units, too small to say which way the pen moved, so the short downward dab drawn here is not a claim. The path is fitted to the bundled Noto Sans Tamil disc, which the font centres at 0.43 to 0.60 of the consonant's width, a little left of where this writer put it. Handwriting varies by writer.",
    },
  },
];
