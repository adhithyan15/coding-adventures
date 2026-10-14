// Gujarati filmstrip policy: cited mark order and font substitutions.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

const FONT =
  "Noto Sans Gujarati Version 2.106, bundled as learning/human-languages/_fonts/NotoSansGujarati-Static.ttf";
const FONT_URL = "https://github.com/notofonts/gujarati";

export const gujaratiFigureTargetPolicy: FigureTargetPolicy = {
  script: "gujarati",
  tracks: ["gujarati"],
  separateLetters: true,
  writtenSignSides: {
    "\u0ABE": "after",
    "\u0ABF": "after",
    "\u0AC0": "after",
    "\u0AC1": "after",
    "\u0AC2": "after",
    "\u0AC7": "after",
    "\u0AC8": "after",
    "\u0ACB": "after",
    "\u0ACC": "after",
    "\u0A82": "after",
    "\u0A83": "after",
  },
  fusedSignPairSources: [
    {
      citation:
        `${FONT}, GSUB 'blws' contextual lookup 90: before ુ or ૂ the consonants ` +
        "ખ ગ ઘ ચ ઞ ણ ત થ ધ ન પ ફ બ ભ મ ય લ ળ વ શ ષ સ take a stem form that neither the " +
        "letter's ductus nor the sign's draws",
      url: FONT_URL,
      pairs: [..."ખગઘચઞણતથધનપફબભમયલળવશષસ"].flatMap((consonant) => [
        `${consonant}\u0AC1`,
        `${consonant}\u0AC2`,
      ]),
    },
    {
      citation:
        `${FONT}, GSUB 'blws' ligature lookup 89: ર with ુ, ર with ૂ, and ણ with ુ ` +
        "each become one glyph of their own",
      url: FONT_URL,
      pairs: ["\u0AB0\u0AC1", "\u0AB0\u0AC2", "\u0AA3\u0AC1"],
    },
    {
      citation:
        `${FONT}, GSUB 'psts' lookups 94 to 96 and 106: after જ or ૹ, ો and ૌ ` +
        "are split into ા plus ે or ૈ, and ા and ી then join the consonant into one glyph",
      url: FONT_URL,
      pairs: ["\u0A9C", "\u0AF9"].flatMap((consonant) =>
        ["\u0ABE", "\u0AC0", "\u0ACB", "\u0ACC"].map((sign) => `${consonant}${sign}`),
      ),
    },
  ],
};
