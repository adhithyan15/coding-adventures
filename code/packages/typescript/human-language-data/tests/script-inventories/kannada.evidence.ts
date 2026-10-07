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
    // The standalone ring now carries its own cited ductus, from Chimple's
    // tracing lessons: one anticlockwise run, drawn last, after its carrier.
    // The composition record above still makes no handwriting claim.
    expect(kannadaAnusvara.strokeOrder).toEqual([
      "circle anticlockwise from the left",
    ]);
    expect(kannadaAnusvara.penLifts).toBe(0);
    expect(kannadaAnusvara.strokeOrderSource?.url).toBe(
      "https://github.com/chimple/chimple-zips/blob/5b137ab1bbd8516f9b4813f23c0e8e9c99f26156/LIDO_kn4800.zip",
    );
    expect(kannadaAnusvara.strokeOrderSource?.variation).toMatch(
      /all 34 .* drawn last.*anticlockwise.*one source, not two.*no licence.*confidence is medium/,
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
    expect(missingByScript.get("kannada.json")?.has("್") ?? false).toBe(false);
    expect(affected.get("್") ?? 0).toBe(0);
    expect(missingByScript.get("kannada.json")?.has("ಂ") ?? false).toBe(false);
    expect(affected.get("ಂ") ?? 0).toBe(0);
    expect(missingByScript.get("kannada.json")?.has("ಅ") ?? false).toBe(false);
    expect(affected.get("ಅ") ?? 0).toBe(0);
    const kannadaVisarga = scripts.kannada!.marks!.find(
      (entry) => entry.mark === "ಃ",
    )!;
    expect(kannadaVisarga.sound).toBe("ḥ");
    expect(kannadaVisarga.penLifts).toBe(1);
    expect(kannadaVisarga.strokeOrder).toEqual([
      "circle the upper dot anticlockwise",
      "lift, then circle the lower dot",
    ]);
    expect(kannadaVisarga.strokeOrderSource?.variation).toMatch(
      /Chimple.*all 35.*upper dot comes first.*anticlockwise.*drew the lower dot first while calling it the upper one/,
    );
    expect(kannadaVisarga.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-Alphabet-Aha\.gif.*Kannada visarga ಃ.*569 frames.*22\.76 seconds.*Wikimedia Commons.*2 June 2016/i,
    );
    expect(kannadaVisarga.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*ಅಃ.*carrier first.*upper and lower visarga dots.*standalone U\+0C83.*excludes that carrier.*one intervening lift.*Noto Sans Kannada/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಃ") ?? false).toBe(false);
    expect(affected.get("ಃ") ?? 0).toBe(0);
    // The digits ೧-೯ cite Chimple's LIDO digit lesson: one run each, read for
    // order, start and direction only (the repository has no licence). ೦ has
    // no source (Chimple's ೧೦ re-uses the ೧ image), so it stays undrawn.
    const digitMovements: Record<string, number> = {
      "೧": 3, "೨": 3, "೩": 4, "೪": 5, "೫": 6, "೬": 3, "೭": 3, "೮": 4, "೯": 3,
    };
    for (const digit of scripts.kannada!.digits!) {
      const movements = digitMovements[digit.glyph];
      if (movements === undefined) {
        expect(digit.glyph).toBe("೦");
        expect(digit.strokeOrder).toEqual([]);
        expect(digit.penLifts).toBeUndefined();
        expect(digit.strokeOrderSource).toBeUndefined();
        continue;
      }
      expect(digit.role, digit.glyph).toBe("digit");
      expect(digit.penLifts, digit.glyph).toBe(0);
      expect(digit.strokeOrder, digit.glyph).toHaveLength(movements);
      expect(
        digit.strokeOrder.slice(1).every((step) => step.startsWith("without lifting, ")),
        digit.glyph,
      ).toBe(true);
      expect(digit.strokeOrderNote, digit.glyph).toMatch(/movements in one continuous pen-down run/);
      expect(digit.strokeOrderSource?.url, digit.glyph).toBe(
        "https://github.com/chimple/chimple-zips/blob/5b137ab1bbd8516f9b4813f23c0e8e9c99f26156/LIDO_kn2_0318.zip",
      );
      expect(digit.strokeOrderSource?.citation, digit.glyph).toMatch(
        new RegExp(`^Chimple \\(Sutara Learning Foundation, Bangalore\\), LIDO tracing lessons LIDO_kn2_0318 and LIDO_kn2_0319, .* for ${digit.glyph}, trace image `),
      );
      expect(digit.strokeOrderSource?.variation, digit.glyph).toMatch(
        /^Chimple draws .* as one path .*sha256 cc49279b364bdf6f.*no licence, so only facts are cited.*one source.*confidence is medium.*Noto Sans Kannada/,
      );
    }
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
    expect(missingByScript.get("kannada.json")?.has("ಇ") ?? false).toBe(false);
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
    expect(missingByScript.get("kannada.json")?.has("ಉ") ?? false).toBe(false);
    expect(affected.get("ಉ") ?? 0).toBe(0);
    const kannadaUu = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಊ",
    )!;
    expect(kannadaUu.sound).toBe("ū");
    expect(kannadaUu.penLifts).toBe(0);
    expect(kannadaUu.strokeOrder).toEqual([
      "turn counterclockwise around the compact upper-left spiral",
      "without lifting, descend through the left shoulder and sweep around the broad lower-left bowl",
      "without lifting, climb over the first tall arch, descend through the middle trough, and climb over the second arch",
      "without lifting, descend the outer-right curve and curl around the small lower-right spiral",
    ]);
    expect(kannadaUu.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-uu\.gif.*ಊ.*34 frames.*3\.4 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaUu.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one uninterrupted run.*upper-left spiral.*lower-left bowl.*two joined tall arches.*outer-right curve.*lower-right spiral.*Noto Sans Kannada.*zero-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಊ") ?? false).toBe(false);
    expect(affected.get("ಊ") ?? 0).toBe(0);
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
    expect(missingByScript.get("kannada.json")?.has("ಎ") ?? false).toBe(false);
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
    expect(missingByScript.get("kannada.json")?.has("ಏ") ?? false).toBe(false);
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
    expect(missingByScript.get("kannada.json")?.has("ಒ") ?? false).toBe(false);
    expect(affected.get("ಒ") ?? 0).toBe(0);
    const kannadaOo = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಓ",
    )!;
    expect(kannadaOo.sound).toBe("ō");
    expect(kannadaOo.penLifts).toBe(1);
    expect(kannadaOo.strokeOrder).toEqual([
      "turn counterclockwise around the compact upper-left loop",
      "without lifting, descend through the curved middle into the lower-left bowl",
      "without lifting, sweep through the join and around the lower-right bowl",
      "without lifting, climb the right side and curl left at the open terminal",
      "lift, then sweep left and curl upward through the small upper flourish",
    ]);
    expect(kannadaOo.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-oo\.gif.*ಓ.*35 frames.*3\.5 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaOo.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*upper-left loop.*curved middle.*joined lower bowls.*open terminal.*one lift.*small upper flourish.*Noto Sans Kannada.*one-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಓ") ?? false).toBe(false);
    expect(affected.get("ಓ") ?? 0).toBe(0);
    const kannadaAi = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಐ",
    )!;
    expect(kannadaAi.sound).toBe("ai");
    expect(kannadaAi.penLifts).toBe(0);
    expect(kannadaAi.strokeOrder).toEqual([
      "turn clockwise through the compact left spiral and around its lower bowl",
      "without lifting, sweep through the join and around the broad right loop",
      "without lifting, carry the high arch leftward and finish at the open upper-left terminal",
    ]);
    expect(kannadaAi.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ai\.gif.*ಐ.*28 frames.*2\.8 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaAi.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one uninterrupted run.*left spiral.*lower bowl.*broad right loop.*high arch.*open upper-left terminal.*Noto Sans Kannada.*zero-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಐ") ?? false).toBe(false);
    expect(affected.get("ಐ") ?? 0).toBe(0);
    const kannadaVocalicR = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಋ",
    )!;
    expect(kannadaVocalicR.sound).toBe("r̥");
    expect(kannadaVocalicR.penLifts).toBe(2);
    expect(kannadaVocalicR.strokeOrder).toEqual([
      "turn clockwise around the compact upper-left spiral, descend through the outer curve, curl around the lower-left spiral, and sweep through the join around the rounded middle bowl",
      "lift, draw the inward bar from left to right, then curl upward into the high hook",
      "lift, sweep rightward around the lower bowl, climb its outer side, and finish at the open upper terminal",
    ]);
    expect(kannadaVocalicR.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ru\.gif.*ಋ.*59 frames.*5\.9 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaVocalicR.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*three pen-down runs.*upper-left spiral.*lower-left spiral.*rounded middle bowl.*lift.*inward bar.*high hook.*second lift.*right bowl.*open upper terminal.*Noto Sans Kannada.*two-lift order/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಋ") ?? false).toBe(false);
    expect(affected.get("ಋ") ?? 0).toBe(0);
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
    expect(missingByScript.get("kannada.json")?.has("ಆ") ?? false).toBe(false);
    expect(affected.get("ಆ") ?? 0).toBe(0);
    const kannadaLongI = scripts.kannada!.independentVowels!.find(
      (entry) => entry.glyph === "ಈ",
    )!;
    expect(kannadaLongI.sound).toBe("ī");
    expect(kannadaLongI.penLifts).toBe(1);
    expect(kannadaLongI.strokeOrder).toEqual([
      "draw the broad rounded body and return to its upper-right join",
      "without lifting, sweep the upper bar left, retrace it right, and curl upward",
      "lift, then draw the horizontal crossbar from left to right",
      "without lifting, turn around the small right loop and descend into the lower hook",
    ]);
    expect(kannadaLongI.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ee\.gif.*ಈ.*44 frames.*4\.4 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaLongI.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*rounded body.*retrace.*curl.*one lift.*crossbar.*right loop.*lower hook.*Noto Sans Kannada/i,
    );
    expect(missingByScript.get("kannada.json")?.has("ಈ") ?? false).toBe(false);
    expect(affected.get("ಈ") ?? 0).toBe(0);
    // Base consonants ನ, ತ, ದ, ರ, ಕ and ಗ are bare-consonant letter rows
    // (role "syllable"). Each cites one Gopala Krishna A animation; ತ and ದ
    // are filed under the uploader's slugs "tha" and "dha", which the
    // variation must say so a later edit cannot quietly cite ಟ's or ಡ's file.
    const kannadaConsonantNa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ನ",
    )!;
    expect(kannadaConsonantNa.role).toBe("syllable");
    expect(kannadaConsonantNa.penLifts).toBe(1);
    expect(kannadaConsonantNa.strokeOrder).toEqual([
      "rise from the lower tail around the left bowl",
      "without lifting, slant down into the right bowl",
      "without lifting, climb the right side to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantNa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-na.gif",
    );
    expect(kannadaConsonantNa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-na\.gif.*consonant ನ.*34 frames.*3\.4 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantNa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*lower tail.*left bowl.*right bowl.*one lift.*top bar.*hook.*mirror copy.*239×215.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantTa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ತ",
    )!;
    expect(kannadaConsonantTa.role).toBe("syllable");
    expect(kannadaConsonantTa.penLifts).toBe(1);
    expect(kannadaConsonantTa.strokeOrder).toEqual([
      "start at the upper-left end and sweep down around the broad bowl and up its right side",
      "without lifting, turn left over the top and curl down into the inner loop",
      "without lifting, close the small loop and rise to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantTa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-tha.gif",
    );
    expect(kannadaConsonantTa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-tha\.gif.*consonant ತ.*43 frames.*4\.3 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantTa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "tha".*dental ತ \(U\+0CA4\).*ಟ and ಠ.*two pen-down runs.*broad bowl.*inner loop.*one lift.*top bar.*hook.*mirror copy.*209×186.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantDa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ದ",
    )!;
    expect(kannadaConsonantDa.role).toBe("syllable");
    expect(kannadaConsonantDa.penLifts).toBe(1);
    expect(kannadaConsonantDa.strokeOrder).toEqual([
      "start at the upper left and go down the left side into the left lobe, rising into the middle point",
      "without lifting, drop from the point around the right lobe and climb the right side",
      "without lifting, close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantDa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-dha.gif",
    );
    expect(kannadaConsonantDa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-dha\.gif.*consonant ದ.*39 frames.*3\.9 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantDa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "dha".*dental ದ \(U\+0CA6\).*ಡ and ಢ.*two pen-down runs.*left lobe.*middle point.*right lobe.*one lift.*top bar.*hook.*mirror copy.*198×143.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantRa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ರ",
    )!;
    expect(kannadaConsonantRa.role).toBe("syllable");
    expect(kannadaConsonantRa.penLifts).toBe(1);
    expect(kannadaConsonantRa.strokeOrder).toEqual([
      "start at the upper left and go down the left side and round the base",
      "without lifting, climb the right side and close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantRa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ra.gif",
    );
    expect(kannadaConsonantRa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ra\.gif.*consonant ರ.*36 frames.*3\.6 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantRa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*upper left.*base.*right side.*across the top.*one lift.*top bar.*hook.*mirror copy.*230×183.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantKa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಕ",
    )!;
    expect(kannadaConsonantKa.role).toBe("syllable");
    expect(kannadaConsonantKa.penLifts).toBe(3);
    expect(kannadaConsonantKa.strokeOrder).toEqual([
      "start at the upper left and go down the left side and round the base",
      "without lifting, climb the right side and close the bowl leftward along the top",
      "lift, then draw the lower bar from left to right",
      "lift, then draw the short link up from the lower bar",
      "lift, then draw the upper bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantKa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ka.gif",
    );
    expect(kannadaConsonantKa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ka\.gif.*consonant ಕ.*44 frames.*4\.4 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantKa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*four pen-down runs.*round bowl.*lower bar.*second lift.*short link.*third lift.*upper bar.*hook.*mirror copy.*245×192.*Noto Sans Kannada.*straight waist.*three-lift order/i,
    );
    const kannadaConsonantGa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಗ",
    )!;
    expect(kannadaConsonantGa.role).toBe("syllable");
    expect(kannadaConsonantGa.penLifts).toBe(1);
    expect(kannadaConsonantGa.strokeOrder).toEqual([
      "climb the left leg from its foot into the arch",
      "without lifting, arch over and go down the right leg to its foot",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantGa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ga.gif",
    );
    expect(kannadaConsonantGa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ga\.gif.*consonant ಗ.*37 frames.*3\.7 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantGa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*left leg.*arch.*right leg.*one lift.*top bar.*hook.*mirror copy.*227×224.*Noto Sans Kannada.*one-lift order/i,
    );
    // Base consonants ಬ, ಳ, ಯ, ಡ, ಹ and ಸ follow the same pattern. ಡ is filed
    // under the uploader's slug "da" (dental ದ is "dha"), which the variation
    // must say so a later edit cannot quietly cite ದ's or ಢ's file.
    const kannadaConsonantBa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಬ",
    )!;
    expect(kannadaConsonantBa.role).toBe("syllable");
    expect(kannadaConsonantBa.penLifts).toBe(0);
    expect(kannadaConsonantBa.strokeOrder).toEqual([
      "start at the curled tip inside the head and loop up over it clockwise",
      "without lifting, slant down to the left and round the left lobe, rising into the middle point",
      "without lifting, drop from the point round the right lobe's base",
      "without lifting, climb the right side to its tip",
    ]);
    expect(kannadaConsonantBa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ba.gif",
    );
    expect(kannadaConsonantBa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ba\.gif.*consonant ಬ.*29 frames.*2\.9 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantBa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one pen-down run.*curled tip.*head.*left lobe.*middle point.*right lobe.*no top bar.*mirror copy.*214×177.*Noto Sans Kannada.*one-run order/i,
    );
    const kannadaConsonantLla = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಳ",
    )!;
    expect(kannadaConsonantLla.role).toBe("syllable");
    expect(kannadaConsonantLla.penLifts).toBe(1);
    expect(kannadaConsonantLla.strokeOrder).toEqual([
      "start at the left of the small loop and close it counterclockwise",
      "without lifting, sweep down the outer left side and along the base",
      "without lifting, go clockwise round the lower loop and back across its top",
      "without lifting, climb the right bowl to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantLla.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-lla.gif",
    );
    expect(kannadaConsonantLla.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-lla\.gif.*consonant ಳ.*35 frames.*3\.5 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantLla.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*small upper-left loop.*outer left side.*lower loop.*right bowl.*one lift.*top bar.*hook.*mirror copy.*271×224.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantYa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಯ",
    )!;
    expect(kannadaConsonantYa.role).toBe("syllable");
    expect(kannadaConsonantYa.penLifts).toBe(2);
    expect(kannadaConsonantYa.strokeOrder).toEqual([
      "start at the upper left and go down the left side and round the base",
      "without lifting, climb the right side and close the bowl over the top",
      "lift, then dip from the foot of the bowl and climb the middle arm to the top bar",
      "without lifting, run back left along the top bar",
      "without lifting, draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then round the small right bowl from its foot",
      "without lifting, climb its right side and curl in at the top",
    ]);
    expect(kannadaConsonantYa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ya.gif",
    );
    expect(kannadaConsonantYa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ya\.gif.*consonant ಯ.*35 frames.*3\.5 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantYa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*four pen-down runs.*round bowl.*middle arm.*second lift.*top bar.*hook.*third lift.*small right bowl.*mirror copy.*260×189.*Noto Sans Kannada.*arm ends at the bar\./i,
    );
    const kannadaConsonantDda = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಡ",
    )!;
    expect(kannadaConsonantDda.role).toBe("syllable");
    expect(kannadaConsonantDda.penLifts).toBe(1);
    expect(kannadaConsonantDda.strokeOrder).toEqual([
      "start at the upper left and go down the left side into the left lobe, rising into the middle point",
      "without lifting, drop from the point around the right lobe and climb the right side",
      "without lifting, curl left into the small inner loop and go round it counterclockwise",
      "without lifting, rise out of the loop and close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantDda.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-da.gif",
    );
    expect(kannadaConsonantDda.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-da\.gif.*consonant ಡ.*49 frames.*4\.9 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantDda.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "da".*retroflex ಡ \(U\+0CA1\).*dental ದ is "dha".*"dda" animates ಢ.*two pen-down runs.*left lobe.*middle point.*right lobe.*inner loop.*one lift.*top bar.*hook.*mirror copy.*230×164.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantHa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಹ",
    )!;
    expect(kannadaConsonantHa.role).toBe("syllable");
    expect(kannadaConsonantHa.penLifts).toBe(1);
    expect(kannadaConsonantHa.strokeOrder).toEqual([
      "start at the upper right of the left ring and close it counterclockwise",
      "without lifting, arch over to the right and go down the right ring's outer side",
      "without lifting, round the right ring's base and climb its inner side to the waist",
      "without lifting, rise up the neck to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantHa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ha.gif",
    );
    expect(kannadaConsonantHa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ha\.gif.*consonant ಹ.*40 frames.*4\.0 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantHa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*left ring.*counterclockwise.*right ring.*neck.*one lift.*top bar.*hook.*mirror copy.*235×217.*Noto Sans Kannada.*share one upright.*one-lift order/i,
    );
    const kannadaConsonantSa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಸ",
    )!;
    expect(kannadaConsonantSa.role).toBe("syllable");
    expect(kannadaConsonantSa.penLifts).toBe(2);
    expect(kannadaConsonantSa.strokeOrder).toEqual([
      "start at the tail and climb round the left curve",
      "without lifting, slant down to the right into the base",
      "without lifting, climb the right side and curve in at the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then set the dot in the middle",
    ]);
    expect(kannadaConsonantSa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-sa.gif",
    );
    expect(kannadaConsonantSa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-sa\.gif.*consonant ಸ.*40 frames.*4\.0 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantSa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*three pen-down runs.*tail.*left curve.*right side.*after a lift.*top bar.*hook.*second lift.*dot.*mirror copy.*234×190.*Noto Sans Kannada.*closed loop.*two-lift order/i,
    );
    // Base consonants ಚ, ಪ, ಝ, ಥ, ಮ, ಲ, ವ and ಜ follow the same pattern. ಥ is
    // filed under the uploader's slug "thha" ("tha" is dental ತ and "tta" is
    // ಠ), which the variation must say so a later edit cannot quietly cite
    // ತ's or ಠ's file. ಚ and ಝ had no listed Commons size to compare the
    // mirror with, and their variations must say so rather than claim a match.
    const kannadaConsonantCa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಚ",
    )!;
    expect(kannadaConsonantCa.role).toBe("syllable");
    expect(kannadaConsonantCa.penLifts).toBe(1);
    expect(kannadaConsonantCa.strokeOrder).toEqual([
      "start at the curled tip inside the head and loop up over it clockwise",
      "without lifting, slant down to the left and round the left lobe, rising into the middle point",
      "without lifting, drop from the point round the right lobe's base",
      "without lifting, climb the right side to the lower bar",
      "without lifting, run back left along the lower bar",
      "without lifting, draw the lower bar from left to right",
      "without lifting, come back along the bar to the short link",
      "without lifting, draw the short link up from the lower bar",
      "lift, then draw the upper bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantCa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-cha.gif",
    );
    expect(kannadaConsonantCa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-cha\.gif.*consonant ಚ.*52 frames.*5\.2 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantCa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*four pen-down runs.*curled tip.*head.*left lobe.*middle point.*right lobe.*lower bar.*second lift.*short link.*third lift.*upper bar.*hook.*mirror copy \(206×180, 52 frames\).*no listed size.*Noto Sans Kannada.*climb ends at the bar\./i,
    );
    // ಚ and ಯ once lifted the pen after every run of their Commons
    // animations (three lifts each), more often than even Omniglot's
    // copyists do. No native Kannada pen data was reachable; Omniglot's
    // copyists are non-native and over-count against native writers, so
    // their counts are cited only as a ceiling, as counts and shares, never
    // as native evidence. ಚ now lifts once, before the upper bar; ಯ lifts
    // twice, and its record must say why one lift was not used.
    const omniglotCeiling: ReadonlyArray<
      readonly [string, number, string, string, string]
    > = [
      [
        "ಚ",
        1,
        "The source shows eight movements in four pen-down runs; Omniglot's non-native copyists most often draw ಚ in two strokes (55%, 11 of 20).",
        "Even under that ceiling, 55% of them (11 of 20) draw ಚ in two strokes and 15% (3 of 20) in one; 10% (2 of 20) use the animation's four.",
        "so the pen lifts once, before the upper bar.",
      ],
      [
        "ಯ",
        2,
        "The source shows seven movements in four pen-down runs; Omniglot's non-native copyists draw ಯ in three strokes (45%, 9 of 20) or two (40%, 8 of 20).",
        "Even under that ceiling, 45% of them (9 of 20) draw ಯ in three strokes and 40% (8 of 20) in two; 10% (2 of 20) use the animation's four.",
        "One lift would mean retracing more than half the round bowl to reach the arm's foot",
      ],
    ];
    for (const [glyph, lifts, note, shares, join] of omniglotCeiling) {
      const letter = scripts.kannada!.letters.find(
        (entry) => entry.glyph === glyph,
      )!;
      expect(letter.penLifts, glyph).toBe(lifts);
      expect(letter.strokeOrderNote, glyph).toContain(note);
      for (const phrase of [
        "they are not a count of how often fluent writers lift the pen",
        "no native writers' pen data for Kannada was available",
        "Omniglot (Lake, Salakhutdinov & Tenenbaum, Science 350:1332, 2015; https://github.com/brendenlake/omniglot, MIT licence)",
        "Amazon Mechanical Turk worker copying a printed exemplar",
        "They are non-native copyists, not native writers",
        "read here only as a ceiling on native lifts, and only counts and shares are cited",
        shares,
        "Those counts say how many strokes, not where they break",
        join,
      ]) {
        expect(letter.strokeOrderSource?.variation, glyph).toContain(phrase);
      }
      expect(
        (letter.strokeOrder ?? []).filter((step) => step.startsWith("lift")),
        glyph,
      ).toHaveLength(lifts);
    }
    const kannadaConsonantPa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಪ",
    )!;
    expect(kannadaConsonantPa.role).toBe("syllable");
    expect(kannadaConsonantPa.penLifts).toBe(2);
    expect(kannadaConsonantPa.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind up and round it to the left",
      "without lifting, run along the base and rise into the middle point",
      "without lifting, drop round the right lobe and climb the right side to its tip",
      "lift, then set the dot in the middle",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantPa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-pa.gif",
    );
    expect(kannadaConsonantPa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-pa\.gif.*consonant ಪ.*32 frames.*3\.2 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantPa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*three pen-down runs.*inner tip of the curl.*middle point.*right lobe.*after a lift.*dot.*second lift.*top bar.*hook.*mirror copy.*277×184.*Noto Sans Kannada.*closed loop.*two-lift order/i,
    );
    const kannadaConsonantJha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಝ",
    )!;
    expect(kannadaConsonantJha.role).toBe("syllable");
    expect(kannadaConsonantJha.penLifts).toBe(4);
    expect(kannadaConsonantJha.strokeOrder).toEqual([
      "start at the upper left and go down the left side and round the base",
      "without lifting, climb the right side and close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then round the middle arm from its foot",
      "without lifting, climb its right side and curl in",
      "lift, then round the right arm from its foot",
      "without lifting, climb its right side and curl in",
      "lift, then draw the tail downward",
    ]);
    expect(kannadaConsonantJha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-jha.gif",
    );
    expect(kannadaConsonantJha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-jha\.gif.*consonant ಝ.*53 frames.*5\.3 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantJha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*five pen-down runs.*round bowl.*after a lift.*top bar.*hook.*second lift.*middle arm.*third lift.*right arm.*fourth lift.*tail.*mirror copy \(225×196, 53 frames\).*no listed size.*Noto Sans Kannada.*four-lift order/i,
    );
    const kannadaConsonantTha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಥ",
    )!;
    expect(kannadaConsonantTha.role).toBe("syllable");
    expect(kannadaConsonantTha.penLifts).toBe(3);
    expect(kannadaConsonantTha.strokeOrder).toEqual([
      "start at the upper left and go down the left side into the left lobe, rising into the middle point",
      "without lifting, drop from the point around the right lobe and climb the right side",
      "without lifting, close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then draw the tail downward",
      "lift, then set the dot in the middle",
    ]);
    expect(kannadaConsonantTha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-thha.gif",
    );
    expect(kannadaConsonantTha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-thha\.gif.*consonant ಥ.*43 frames.*4\.3 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantTha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "thha".*dental ಥ \(U\+0CA5\).*"tha" animates dental ತ.*"tta" retroflex ಠ.*four pen-down runs.*left lobe.*middle point.*right lobe.*after a lift.*top bar.*hook.*second lift.*tail.*third lift.*dot.*mirror copy.*215×174.*frame count.*Noto Sans Kannada.*three-lift order/i,
    );
    const kannadaConsonantMa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಮ",
    )!;
    expect(kannadaConsonantMa.role).toBe("syllable");
    expect(kannadaConsonantMa.penLifts).toBe(2);
    expect(kannadaConsonantMa.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind up and round it to the left",
      "without lifting, run along the base and rise into the middle point",
      "without lifting, drop round the right lobe and climb the right side to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then round the right bowl from its foot",
      "without lifting, climb its right side and curl in at the top",
    ]);
    expect(kannadaConsonantMa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ma.gif",
    );
    expect(kannadaConsonantMa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ma\.gif.*consonant ಮ.*42 frames.*4\.2 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantMa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*three pen-down runs.*inner tip of the curl.*middle point.*right lobe.*after a lift.*top bar.*hook.*second lift.*right bowl.*mirror copy.*244×236.*Noto Sans Kannada.*two-lift order/i,
    );
    const kannadaConsonantLa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಲ",
    )!;
    expect(kannadaConsonantLa.role).toBe("syllable");
    expect(kannadaConsonantLa.penLifts).toBe(0);
    expect(kannadaConsonantLa.strokeOrder).toEqual([
      "start at the left of the small loop and close it counterclockwise",
      "without lifting, sweep down the outer left side and round the base",
      "without lifting, climb the right side and curl in to its tip",
    ]);
    expect(kannadaConsonantLa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-la.gif",
    );
    expect(kannadaConsonantLa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-la\.gif.*consonant ಲ.*30 frames.*3\.0 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantLa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one pen-down run.*small upper-left loop.*outer left side.*base.*right side.*no top bar.*mirror copy.*230×210.*Noto Sans Kannada.*one-run order/i,
    );
    const kannadaConsonantVa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ವ",
    )!;
    expect(kannadaConsonantVa.role).toBe("syllable");
    expect(kannadaConsonantVa.penLifts).toBe(1);
    expect(kannadaConsonantVa.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind up and round it to the left",
      "without lifting, run along the base and rise into the middle point",
      "without lifting, drop round the right lobe and climb the right side to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantVa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-va.gif",
    );
    expect(kannadaConsonantVa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-va\.gif.*consonant ವ.*32 frames.*3\.2 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantVa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*inner tip of the curl.*middle point.*right lobe.*one lift.*top bar.*hook.*mirror copy.*191×183.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantJa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಜ",
    )!;
    expect(kannadaConsonantJa.role).toBe("syllable");
    expect(kannadaConsonantJa.penLifts).toBe(1);
    expect(kannadaConsonantJa.strokeOrder).toEqual([
      "start at the curled tip inside the head and loop up over it clockwise",
      "without lifting, slant down to the left and round the left lobe, rising into the middle point",
      "without lifting, drop from the point round the right lobe's base",
      "without lifting, climb the right side and curl in to its tip",
      "lift, then sweep the upper arc from the head out to its upturned tip",
    ]);
    expect(kannadaConsonantJa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ja.gif",
    );
    expect(kannadaConsonantJa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ja\.gif.*consonant ಜ.*37 frames.*3\.7 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantJa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*curled tip.*head.*left lobe.*middle point.*right lobe.*one lift.*upper arc.*mirror copy.*234×195.*Noto Sans Kannada.*one-lift order/i,
    );
    // Base consonants ಟ, ಣ, ಶ, ಷ, ಧ, ಭ, ಫ, ಖ, ಘ and ಢ follow the same pattern.
    // ಟ, ಧ and ಢ are filed under the uploader's slugs "ta", "dhha" and "dda"
    // ("tha" is ತ, "dha" is ದ and "da" is ಡ), which their variations must
    // say so a later edit cannot quietly cite another letter's file. ಭ had no
    // listed Commons size, and ಖ and ಘ only a pixel size, so their variations
    // must say what was and was not compared rather than claim a match.
    const kannadaConsonantTta = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಟ",
    )!;
    expect(kannadaConsonantTta.role).toBe("syllable");
    expect(kannadaConsonantTta.penLifts).toBe(0);
    expect(kannadaConsonantTta.strokeOrder).toEqual([
      "start at the tip of the top curl and curve round to the left and down into the waist",
      "without lifting, go round the small loop counterclockwise and close it at the waist",
      "without lifting, go down the left side and round the left lobe, rising into the middle point",
      "without lifting, drop round the right lobe and climb the right side",
      "without lifting, curl in to the short tick in the middle",
      "without lifting, come back and round the upper bowl to its tip",
    ]);
    expect(kannadaConsonantTta.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ta.gif",
    );
    expect(kannadaConsonantTta.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-ta\.gif.*consonant ಟ.*34 frames.*3\.4 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantTta.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "ta".*retroflex ಟ \(U\+0C9F\).*dental ತ is "tha".*"tta" animates ಠ.*one pen-down run.*top curl.*small loop.*left lobe.*middle point.*right lobe.*tick.*upper bowl.*no top bar.*mirror copy.*229×199.*Noto Sans Kannada.*one-run order/i,
    );
    const kannadaConsonantNna = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಣ",
    )!;
    expect(kannadaConsonantNna.role).toBe("syllable");
    expect(kannadaConsonantNna.penLifts).toBe(0);
    expect(kannadaConsonantNna.strokeOrder).toEqual([
      "start at the foot of the lower curl and round it up to the short tick at the waist",
      "without lifting, come back and round the upper curl into the middle point",
      "without lifting, arch over to the right and come down the right side",
      "without lifting, round the base and spiral in to the centre",
    ]);
    expect(kannadaConsonantNna.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-nna.gif",
    );
    expect(kannadaConsonantNna.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-nna\.gif.*consonant ಣ.*45 frames.*4\.5 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantNna.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one pen-down run.*lower curl.*upper curl.*middle point.*arches over.*right side.*spirals in.*no top bar.*mirror copy.*252×200.*Noto Sans Kannada.*one-run order/i,
    );
    const kannadaConsonantSha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಶ",
    )!;
    expect(kannadaConsonantSha.role).toBe("syllable");
    expect(kannadaConsonantSha.penLifts).toBe(1);
    expect(kannadaConsonantSha.strokeOrder).toEqual([
      "start at the head and rise to run right along the top",
      "without lifting, curve down and sweep across to the lower left",
      "without lifting, round the base and climb the right side to the top bar",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
    ]);
    expect(kannadaConsonantSha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-sha.gif",
    );
    expect(kannadaConsonantSha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-sha\.gif.*consonant ಶ.*31 frames.*3\.1 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantSha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*two pen-down runs.*head.*along the top.*lower left.*base.*right side.*one lift.*top bar.*hook.*mirror copy.*259×211.*Noto Sans Kannada.*one-lift order/i,
    );
    const kannadaConsonantSsa = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಷ",
    )!;
    expect(kannadaConsonantSsa.role).toBe("syllable");
    expect(kannadaConsonantSsa.penLifts).toBe(3);
    expect(kannadaConsonantSsa.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind up and round it to the left",
      "without lifting, run along the base and rise into the middle point",
      "without lifting, drop round the right lobe and climb the right side to its tip",
      "lift, then set the dot in the middle",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then draw the slanting stroke down to the right",
    ]);
    expect(kannadaConsonantSsa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-shha.gif",
    );
    expect(kannadaConsonantSsa.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-shha\.gif.*consonant ಷ.*40 frames.*4\.0 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantSsa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*four pen-down runs.*inner tip of the curl.*middle point.*right lobe.*after a lift.*dot.*second lift.*top bar.*hook.*third lift.*slanting stroke.*mirror copy.*268×227.*Noto Sans Kannada.*three-lift order/i,
    );
    const kannadaConsonantDha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಧ",
    )!;
    expect(kannadaConsonantDha.role).toBe("syllable");
    expect(kannadaConsonantDha.penLifts).toBe(2);
    expect(kannadaConsonantDha.strokeOrder).toEqual([
      "start at the upper left and go down the left side into the left lobe, rising into the middle point",
      "without lifting, drop from the point around the right lobe and climb the right side",
      "without lifting, close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then draw the tail downward",
    ]);
    expect(kannadaConsonantDha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-dhha.gif",
    );
    expect(kannadaConsonantDha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-dhha\.gif.*consonant ಧ.*42 frames.*4\.2 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantDha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "dhha".*dental ಧ \(U\+0CA7\).*"dha" animates dental ದ.*three pen-down runs.*left lobe.*middle point.*right lobe.*after a lift.*top bar.*hook.*second lift.*tail.*mirror copy.*211×193.*Noto Sans Kannada.*two-lift order/i,
    );
    const kannadaConsonantBha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಭ",
    )!;
    expect(kannadaConsonantBha.role).toBe("syllable");
    expect(kannadaConsonantBha.penLifts).toBe(1);
    expect(kannadaConsonantBha.strokeOrder).toEqual([
      "start at the curled tip inside the head and loop up over it clockwise",
      "without lifting, slant down to the left and round the left lobe, rising into the middle point",
      "without lifting, drop round the right lobe and climb the right side to the bar",
      "without lifting, run back left along the bar",
      "without lifting, draw the bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then draw the tail downward",
    ]);
    expect(kannadaConsonantBha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-bha.gif",
    );
    expect(kannadaConsonantBha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-bha\.gif.*consonant ಭ.*33 frames.*3\.3 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantBha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*three pen-down runs.*curled tip.*head.*left lobe.*middle point.*right lobe.*after a lift.*top bar.*hook.*second lift.*tail.*mirror copy \(234×189, 33 frames\).*no listed size.*Noto Sans Kannada.*climb ends at the bar\./i,
    );
    const kannadaConsonantPha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಫ",
    )!;
    expect(kannadaConsonantPha.role).toBe("syllable");
    expect(kannadaConsonantPha.penLifts).toBe(3);
    expect(kannadaConsonantPha.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind up and round it to the left",
      "without lifting, run along the base and rise into the middle point",
      "without lifting, drop round the right lobe and climb the right side to its tip",
      "lift, then set the dot in the middle",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then draw the tail downward",
    ]);
    expect(kannadaConsonantPha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-pha.gif",
    );
    expect(kannadaConsonantPha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-pha\.gif.*consonant ಫ.*43 frames.*4\.3 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantPha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*four pen-down runs.*inner tip of the curl.*middle point.*right lobe.*after a lift.*dot.*second lift.*top bar.*hook.*third lift.*tail.*mirror copy.*240×177.*Noto Sans Kannada.*three-lift order/i,
    );
    const kannadaConsonantKha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಖ",
    )!;
    expect(kannadaConsonantKha.role).toBe("syllable");
    expect(kannadaConsonantKha.penLifts).toBe(0);
    expect(kannadaConsonantKha.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind out of it, up and over to the right",
      "without lifting, come down through the waist to the base",
      "without lifting, round the lower loop and cross back through the waist",
      "without lifting, run along the base and climb the right side, curving over to its tip",
    ]);
    expect(kannadaConsonantKha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-kha.gif",
    );
    expect(kannadaConsonantKha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-kha\.gif.*consonant ಖ.*37 frames.*3\.7 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantKha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*one pen-down run.*inner tip of the curl.*waist.*lower loop.*right side.*no top bar.*mirror copy.*listed pixel size \(209×192\).*Noto Sans Kannada.*one-run order/i,
    );
    const kannadaConsonantGha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಘ",
    )!;
    expect(kannadaConsonantGha.role).toBe("syllable");
    expect(kannadaConsonantGha.penLifts).toBe(3);
    expect(kannadaConsonantGha.strokeOrder).toEqual([
      "start at the inner tip of the curl and wind up and round it to the left",
      "without lifting, run along the base and rise into the middle point",
      "without lifting, drop round the right lobe and climb the right side",
      "without lifting, carry on up the middle arm to its tip",
      "without lifting, come back down the middle arm to its foot",
      "without lifting, round the right arm and curl in at its top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then set the dot in the middle",
      "lift, then draw the tail downward",
    ]);
    expect(kannadaConsonantGha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-gha.gif",
    );
    expect(kannadaConsonantGha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-gha\.gif.*consonant ಘ.*61 frames.*6\.1 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantGha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*five pen-down runs.*inner tip of the curl.*middle point.*right lobe.*middle arm.*after a lift.*right arm.*second lift.*top bar.*hook.*third lift.*dot.*fourth lift.*tail.*mirror copy.*listed pixel size \(225×210\).*Noto Sans Kannada.*tapered bar\./i,
    );
    const kannadaConsonantDdha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಢ",
    )!;
    expect(kannadaConsonantDdha.role).toBe("syllable");
    expect(kannadaConsonantDdha.penLifts).toBe(2);
    expect(kannadaConsonantDdha.strokeOrder).toEqual([
      "start at the upper left and go down the left side into the left lobe, rising into the middle point",
      "without lifting, drop from the point around the right lobe and climb the right side",
      "without lifting, curl left into the small inner loop and go round it counterclockwise",
      "without lifting, rise out of the loop and close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then draw the tail downward",
    ]);
    expect(kannadaConsonantDdha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-dda.gif",
    );
    expect(kannadaConsonantDdha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-dda\.gif.*consonant ಢ.*52 frames.*5\.2 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantDdha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "dda".*retroflex ಢ \(U\+0CA2\).*"da" animates ಡ.*three pen-down runs.*left lobe.*middle point.*right lobe.*inner loop.*after a lift.*top bar.*hook.*second lift.*tail.*mirror copy.*244×173.*Noto Sans Kannada.*two-lift order/i,
    );
    // ಭ and ಘ animate one more run than even Omniglot's non-native copyists
    // most often draw (three and five against two and four). As for ಚ and
    // ಯ, their counts are cited only as a ceiling, and each path joins one
    // pair of runs along ink it is about to draw or has drawn: ಭ runs back
    // along its bar (one lift), ಘ comes back down its middle arm (three).
    const omniglotCeilingLater: ReadonlyArray<
      readonly [string, number, string, string, string]
    > = [
      [
        "ಭ",
        1,
        "The source shows six movements in three pen-down runs; Omniglot's non-native copyists most often draw ಭ in two strokes (55%, 11 of 20).",
        "Even under that ceiling, 55% of them (11 of 20) draw ಭ in two strokes; 30% (6 of 20) use the animation's three.",
        "so the pen lifts once, before the tail.",
      ],
      [
        "ಘ",
        3,
        "The source shows nine movements in five pen-down runs; Omniglot's non-native copyists most often draw ಘ in four strokes (65%, 13 of 20).",
        "Even under that ceiling, 65% of them (13 of 20) draw ಘ in four strokes; 15% (3 of 20) use the animation's five.",
        "coming back down the middle arm to its foot, so the pen lifts three times.",
      ],
    ];
    for (const [glyph, lifts, note, shares, join] of omniglotCeilingLater) {
      const letter = scripts.kannada!.letters.find(
        (entry) => entry.glyph === glyph,
      )!;
      expect(letter.penLifts, glyph).toBe(lifts);
      expect(letter.strokeOrderNote, glyph).toContain(note);
      for (const phrase of [
        "they are not a count of how often fluent writers lift the pen",
        "no native writers' pen data for Kannada was available",
        "Omniglot (Lake, Salakhutdinov & Tenenbaum, Science 350:1332, 2015; https://github.com/brendenlake/omniglot, MIT licence)",
        "Amazon Mechanical Turk worker copying a printed exemplar",
        "They are non-native copyists, not native writers",
        "read here only as a ceiling on native lifts, and only counts and shares are cited",
        shares,
        "Those counts say how many strokes, not where they break",
        join,
      ]) {
        expect(letter.strokeOrderSource?.variation, glyph).toContain(phrase);
      }
      expect(
        (letter.strokeOrder ?? []).filter((step) => step.startsWith("lift")),
        glyph,
      ).toHaveLength(lifts);
    }
    // ಠ, the last Kannada consonant to gain a stroke order, follows the same
    // pattern. Its animation is filed under the slug "tta" ("ta" is ಟ and
    // "thha" is ಥ), which its variation must say so a later edit cannot
    // quietly cite another letter's file. Its two lifts equal the Omniglot
    // copyists' most common count, so no ceiling wording is needed.
    const kannadaConsonantTtha = scripts.kannada!.letters.find(
      (entry) => entry.glyph === "ಠ",
    )!;
    expect(kannadaConsonantTtha.role).toBe("syllable");
    expect(kannadaConsonantTtha.penLifts).toBe(2);
    expect(kannadaConsonantTtha.strokeOrder).toEqual([
      "start at the upper left of the bowl and go down the left side and round the base",
      "without lifting, climb the right side",
      "without lifting, close the bowl leftward along the top",
      "lift, then draw the top bar from left to right",
      "without lifting, curl up into the hook",
      "lift, then set the dot in the middle",
    ]);
    expect(kannadaConsonantTtha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-tta.gif",
    );
    expect(kannadaConsonantTtha.strokeOrderSource?.citation).toMatch(
      /Gopala Krishna A.*Kannada-alphabet-tta\.gif.*consonant ಠ.*41 frames.*4\.1 seconds.*Wikimedia Commons.*25 May 2016/i,
    );
    expect(kannadaConsonantTtha.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*slug "tta".*retroflex ಠ \(U\+0CA0\).*"ta" animates retroflex ಟ.*"thha" dental ಥ.*three pen-down runs.*round bowl.*left side.*base.*right side.*after a lift.*top bar.*hook.*second lift.*dot.*mirror copy.*231×208, 105 KB.*Noto Sans Kannada.*two-lift order/i,
    );
  },
};
