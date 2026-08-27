// Exact real-corpus evidence owned by the Kannada inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Kannada",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const kannadaHalant = scripts.kannada!.marks!.find(
      (mark) => mark.mark === "್",
    )!;
    expect(kannadaHalant.role).toBe("virama");
    expect(kannadaHalant.compositionOrder).toEqual([
      "write the Kannada consonant carrier first",
      "add the halant to suppress its inherent vowel or prepare the following conjunct",
    ]);
    expect(kannadaHalant.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(kannadaHalant.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.8\.2.*U\+0CCD/i,
    );
    expect(kannadaHalant.compositionSource?.variation).toMatch(
      /horn.*dead consonants.*conjuncts.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const kannadaAnusvara = scripts.kannada!.marks!.find(
      (mark) => mark.mark === "ಂ",
    )!;
    expect(kannadaAnusvara.role).toBe("anusvara");
    expect(kannadaAnusvara.compositionOrder).toEqual([
      "write the Kannada carrier first",
      "add the anusvara to mark consonant nasalization",
    ]);
    expect(kannadaAnusvara.example).toEqual({
      base: "ಅ",
      combined: "ಅಂ",
      sound: "aṃ",
    });
    expect(kannadaAnusvara.compositionSource?.url).toBe(
      "https://www.unicode.org/L2/L2012/12289-index-cnvrt.pdf",
    );
    expect(kannadaAnusvara.compositionSource?.citation).toMatch(
      /Indic Scripts in Unicode.*Kannada.*376.*consonant nasalization sign.*U\+0C82.*KANNADA SIGN ANUSVARA.*2012.*Unicode Standard 17\.0/i,
    );
    expect(kannadaAnusvara.compositionSource?.variation).toMatch(
      /consonant-nasalization role.*not a universal handwriting direction.*pen-lift count.*encoded composition convention.*no standalone ductus claim/i,
    );
    const kannadaA = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಅ",
    )!;
    expect(kannadaA.sound).toBe("a");
    expect(kannadaA.penLifts).toBe(0);
    expect(kannadaA.strokeOrder).toEqual([
      "turn clockwise around the compact left loop",
      "without lifting, descend into the broad lower bowl and sweep up its right side",
      "without lifting, turn counterclockwise around the rounded right loop",
      "without lifting, return left along the inward horizontal bar",
    ]);
    expect(kannadaA.strokeOrderNote).toMatch(
      /four visible movements.*one continuous pen-down run/i,
    );
    expect(kannadaA.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-a.gif",
    );
    expect(kannadaA.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-a\.gif.*independent vowel ಅ.*00:00\.0.?00:03\.4.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaA.strokeOrderSource?.variation).toMatch(
      /35-frame animation.*one uninterrupted run.*left loop.*lower bowl.*right loop.*horizontal bar returning left.*Noto Sans Kannada/i,
    );
    expect(missingByScript.get("kannada.json")?.has("್")).toBe(false);
    expect(affected.get("್") ?? 0).toBe(0);
    expect(missingByScript.get("kannada.json")?.has("ಂ")).toBe(false);
    expect(affected.get("ಂ") ?? 0).toBe(0);
    expect(missingByScript.get("kannada.json")?.has("ಅ")).toBe(false);
    expect(affected.get("ಅ") ?? 0).toBe(0);
    const kannadaI = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಇ",
    )!;
    expect(kannadaI.sound).toBe("i");
    expect(kannadaI.penLifts).toBe(0);
    expect(kannadaI.strokeOrder).toEqual([
      "climb the left upright, turn over the first arch, and descend the middle stem",
      "without lifting, retrace the middle stem upward and turn over the second arch",
      "without lifting, descend through the broad outer curve and turn left along the base",
      "without lifting, close the lower loop and sweep out to the right",
    ]);
    expect(kannadaI.strokeOrderSource?.citation).toMatch(
      /Yogesh.*Animation of hand-writing Kannada character.*ಇ.*98 frames.*4\.6 seconds.*Wikimedia Commons.*26 December 2015/i,
    );
    expect(kannadaI.strokeOrderSource?.variation).toMatch(
      /one uninterrupted run.*first arch.*retrace.*second arch.*outer curve.*lower loop.*Noto Sans Kannada.*zero-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಇ")).toBe(false);
    expect(affected.get("ಇ") ?? 0).toBe(0);
    const kannadaU = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಉ",
    )!;
    expect(kannadaU.sound).toBe("u");
    expect(kannadaU.penLifts).toBe(0);
    expect(kannadaU.strokeOrder).toEqual([
      "turn counterclockwise around the compact upper-left loop",
      "without lifting, descend through the left shoulder and sweep around the broad lower-left bowl",
      "without lifting, climb over the tall middle arch and descend into the lower-right bowl",
      "without lifting, sweep around the outer-right curve and finish at the open upper terminal",
    ]);
    expect(kannadaU.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-u\.gif.*ಉ.*35 frames.*3\.5 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaU.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one uninterrupted run.*upper-left loop.*lower-left bowl.*tall middle arch.*lower-right bowl.*outer-right curve.*open upper terminal.*Noto Sans Kannada.*zero-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಉ")).toBe(false);
    expect(affected.get("ಉ") ?? 0).toBe(0);
    const kannadaE = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಎ",
    )!;
    expect(kannadaE.sound).toBe("e");
    expect(kannadaE.penLifts).toBe(0);
    expect(kannadaE.strokeOrder).toEqual([
      "turn clockwise around the compact left loop",
      "without lifting, sweep through the joined lower-left curve",
      "without lifting, turn around the rounded lower-right bowl and climb its right side",
      "without lifting, carry the tall outer arch over and finish to the left",
    ]);
    expect(kannadaE.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ae\.gif.*ಎ.*30 frames.*3\.0 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaE.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one uninterrupted run.*compact left loop.*joined lower curves.*rounded right side.*tall outer arch.*finish left.*Noto Sans Kannada.*zero-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಎ")).toBe(false);
    expect(affected.get("ಎ") ?? 0).toBe(0);
    const kannadaEe = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಏ",
    )!;
    expect(kannadaEe.sound).toBe("ē");
    expect(kannadaEe.penLifts).toBe(1);
    expect(kannadaEe.strokeOrder).toEqual([
      "turn clockwise around the compact left loop",
      "without lifting, sweep through the joined lower curves and climb the right side",
      "without lifting, carry the tall outer arch over and finish at the upper left",
      "lift, then draw the small upper loop from left to right",
    ]);
    expect(kannadaEe.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-aee\.gif.*ಏ.*31 frames.*3\.1 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaEe.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*compact left loop.*joined lower curves.*tall outer arch.*one lift.*small upper loop.*left to right.*Noto Sans Kannada.*one-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಏ")).toBe(false);
    expect(affected.get("ಏ") ?? 0).toBe(0);
    const kannadaO = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಒ",
    )!;
    expect(kannadaO.sound).toBe("o");
    expect(kannadaO.penLifts).toBe(0);
    expect(kannadaO.strokeOrder).toEqual([
      "turn counterclockwise around the compact upper-left loop",
      "without lifting, descend through the curved middle into the lower-left bowl",
      "without lifting, sweep through the join and around the lower-right bowl",
      "without lifting, climb the right side and curl left at the open terminal",
    ]);
    expect(kannadaO.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-o\.gif.*ಒ.*30 frames.*3\.0 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaO.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one uninterrupted run.*upper-left loop.*curved middle.*lower-left bowl.*lower-right bowl.*curl left.*open terminal.*Noto Sans Kannada.*zero-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಒ")).toBe(false);
    expect(affected.get("ಒ") ?? 0).toBe(0);
    const kannadaAa = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಆ",
    )!;
    expect(kannadaAa.sound).toBe("ā");
    expect(kannadaAa.penLifts).toBe(1);
    expect(kannadaAa.strokeOrder).toEqual([
      "turn clockwise around the compact left loop",
      "without lifting, sweep around the broad lower bowl and finish at the upper right",
      "lift, then turn clockwise around the rounded right loop",
      "without lifting, return left along the inward horizontal bar",
    ]);
    expect(kannadaAa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-aa\.gif.*ಆ.*35 frames.*3\.5 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaAa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*compact left loop.*broad lower bowl.*lift once.*rounded right loop.*horizontal bar.*Noto Sans Kannada.*one-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಆ")).toBe(false);
    expect(affected.get("ಆ") ?? 0).toBe(0);
    expect(
      [...affected.entries()].sort((left, right) => right[1] - left[1])[0],
    ).toEqual(["گ", 2]);
  },
};
