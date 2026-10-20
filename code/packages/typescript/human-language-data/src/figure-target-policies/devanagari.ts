// Devanagari filmstrip policy: shared-headline word composition.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const devanagariFigureTargetPolicy: FigureTargetPolicy = {
  script: "devanagari",
  tracks: ["hindi", "marathi", "sanskrit", "marwadi"],
  headlineWord: {
    letter: /^\p{Script=Devanagari}$/u,
    signs: new Set(["\u093E"]),
    consonant: /^[\u0915-\u0939]$/u,
    splitLetterSources: [
      {
        citation:
          "Noto Sans Devanagari Version 2.006, bundled as learning/human-languages/_fonts/" +
          "NotoSansDevanagari-Static.ttf: GSUB 'abvs' multiple-substitution lookup 179 replaces ई " +
          "with इ plus a reph-shaped mark (glyph uni0930094D) and ऐ with ए plus े, each placed by GPOS",
        url: "https://github.com/notofonts/devanagari",
        letters: ["\u0908", "\u0910"],
      },
    ],
  },
};
