// Malayalam filmstrip policy: cited suffix marks and one apart cluster.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const malayalamFigureTargetPolicy: FigureTargetPolicy = {
  script: "malayalam",
  tracks: ["malayalam"],
  separateLetters: true,
  writtenSignSides: {
    "\u0D02": "after",
    "\u0D3E": "after",
    "\u0D4D": "after",
  },
  apartClusterSources: [
    {
      citation:
        "Noto Sans Malayalam Version 2.104, bundled as learning/human-languages/_fonts/NotoSansMalayalam-Static.ttf, " +
        "shaped with HarfBuzz 14.6.0 under the font's 'mlm2' script, the one fontspec's Script=Malayalam selects: " +
        "സ, ് and ക (സ്ക) print as the glyphs samlym, viramamlym and kamlym, the ones each prints alone, " +
        "unmoved; the 'mlm2' 'akhn' ligature lookup 35 joins സ and ് only with ല, സ, ഥ and റ്റ",
      url: "https://github.com/notofonts/malayalam",
      clusters: ["\u0D38\u0D4D\u0D15"],
    },
  ],
};
