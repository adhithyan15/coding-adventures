// Tamil filmstrip policy: written mark order and the font's fused forms.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const tamilFigureTargetPolicy: FigureTargetPolicy = {
  script: "tamil",
  tracks: ["tamil"],
  separateLetters: 30,
  writtenSignSides: {
    "\u0BC6": "before", // ெ e
    "\u0BC7": "before", // ே ē
    "\u0BC8": "before", // ை ai
    "\u0BBE": "after", // ா ā
    "\u0BBF": "after", // ி i
    "\u0BC0": "after", // ீ ī
    "\u0BCD": "after", // ் puḷḷi
  },
  fusedSignPairSources: [
    {
      citation:
        "The Unicode Standard, Version 17.0, §12.6.3 Tamil Ligatures, Ligatures with Vowel i and " +
        "Figure 12-21: ட with ி and ீ, and ல with ீ, join into ligatures of their own (2025)",
      url: "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
      pairs: ["\u0B9F\u0BBF", "\u0B9F\u0BC0", "\u0BB2\u0BC0"],
    },
  ],
  fusedLetterSequenceSources: [
    {
      citation:
        "Noto Sans Tamil Version 2.004, bundled as learning/human-languages/_fonts/NotoSansTamil-Static.ttf: " +
        "GSUB 'akhn' ligature lookup 2 joins க, ் and ஷ into one glyph; GSUB 'abvs' ligature lookup 1 joins " +
        "ஸ or ஶ with ், ர and ீ into one glyph",
      url: "https://github.com/notofonts/tamil",
      sequences: ["\u0B95\u0BCD\u0BB7", "\u0BB8\u0BCD\u0BB0\u0BC0", "\u0BB6\u0BCD\u0BB0\u0BC0"],
    },
  ],
};
