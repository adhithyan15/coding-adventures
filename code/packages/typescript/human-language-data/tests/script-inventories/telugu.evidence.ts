// Exact real-corpus evidence owned by the Telugu inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Telugu",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const teluguVirama = scripts.telugu!.marks!.find(
      (mark) => mark.mark === "్",
    )!;
    expect(teluguVirama.role).toBe("virama");
    expect(teluguVirama.compositionOrder).toEqual([
      "write the Telugu consonant carrier first",
      "add the virama to suppress its inherent vowel or prepare the following consonant cluster",
    ]);
    expect(teluguVirama.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(teluguVirama.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.7\.1.*Rendering Behavior.*U\+0C4D/i,
    );
    expect(teluguVirama.compositionSource?.variation).toMatch(
      /headstroke.*encoded composition.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const teluguA = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "అ",
    )!;
    expect(teluguA.sound).toBe("a");
    expect(teluguA.penLifts).toBe(1);
    expect(teluguA.strokeOrder).toEqual([
      "turn around the left lobe",
      "sweep around the broad lower bowl",
      "turn around the right lobe",
      "return left along the inner bar",
    ]);
    expect(teluguA.strokeOrderNote).toMatch(
      /four numbered movements.*two pen-down runs.*1.?2.*3.?4/i,
    );
    expect(teluguA.strokeOrderSource?.url).toBe(
      "https://write-telugu-alphabets.en.aptoide.com/app",
    );
    expect(teluguA.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*అ.*movements 1.?4.*version 2\.6/i,
    );
    expect(teluguA.strokeOrderSource?.variation).toMatch(
      /four directional movements.*two pen-down starts.*1.?2.*3.?4.*not uniform.*Noto Sans Telugu/i,
    );
    const teluguE = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఎ",
    )!;
    expect(teluguE.sound).toBe("e");
    expect(teluguE.penLifts).toBe(1);
    expect(teluguE.strokeOrder).toEqual([
      "turn down and left around the compact lower loop",
      "continue around its base and return to the central junction",
      "restart at the junction and sweep up through the broad outer arch",
    ]);
    expect(teluguE.strokeOrderNote).toMatch(
      /three numbered movements.*two pen-down runs.*1.?2.*movement 3/i,
    );
    expect(teluguE.strokeOrderSource?.url).toBe(
      "https://write-telugu-alphabets.en.aptoide.com/app",
    );
    expect(teluguE.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఎ.*dot_stroke_v_9_e\.png.*movements 1.?3.*version 2\.6/i,
    );
    expect(teluguE.strokeOrderSource?.variation).toMatch(
      /three directional movements.*two pen-down runs.*1.?2.*movement 3.*not uniform.*Noto Sans Telugu/i,
    );
    const teluguEe = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఏ",
    )!;
    expect(teluguEe.sound).toBe("ē");
    expect(teluguEe.penLifts).toBe(2);
    expect(teluguEe.strokeOrder).toEqual([
      "turn down and left around the compact lower loop",
      "continue around its base and return to the central junction",
      "restart at the lower-right tail and sweep up through the broad outer arch",
      "restart below the upper-left hook and sweep upward to its tip",
    ]);
    expect(teluguEe.strokeOrderNote).toMatch(
      /four numbered movements.*three pen-down runs.*1.?2.*movement 3.*movement 4/i,
    );
    expect(teluguEe.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఏ.*dot_stroke_v_10_ae\.png.*movements 1.?4.*version 2\.6/i,
    );
    expect(teluguEe.strokeOrderSource?.variation).toMatch(
      /four directional movements.*three pen-down runs.*1.?2.*movement 3.*movement 4.*not uniform.*Noto Sans Telugu/i,
    );
    const teluguAnusvara = scripts.telugu!.marks!.find(
      (mark) => mark.mark === "ం",
    )!;
    expect(teluguAnusvara.role).toBe("anusvara");
    expect(teluguAnusvara.compositionOrder).toEqual([
      "write the Telugu carrier first",
      "add the sunna to mark consonant nasalization",
    ]);
    expect(teluguAnusvara.compositionSource?.url).toBe(
      "https://www.unicode.org/L2/L2012/12289-index-cnvrt.pdf",
    );
    expect(teluguAnusvara.compositionSource?.citation).toMatch(
      /Indic Scripts in Unicode.*Telugu.*352.*sunna.*U\+0C02.*ANUSVARA/i,
    );
    expect(teluguAnusvara.compositionSource?.variation).toMatch(
      /consonant-nasalization role.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    expect(missingByScript.get("telugu.json")?.has("్")).toBe(false);
    expect(affected.get("్") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("ం")).toBe(false);
    expect(affected.get("ం") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("అ")).toBe(false);
    expect(affected.get("అ") ?? 0).toBe(0);
    const teluguAa = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఆ",
    )!;
    expect(teluguAa.sound).toBe("ā");
    expect(teluguAa.penLifts).toBe(1);
    expect(teluguAa.strokeOrder).toEqual([
      "turn around the hooked left lobe and sweep through the broad lower bowl",
      "after lifting, turn around the rounded right lobe and return left along the inner bar",
    ]);
    expect(teluguAa.strokeOrderSource?.citation).toMatch(
      /Hojaswani LUCIDA and Physics classes.*ఆ letter.*00:00–00:10.*15 September 2024/i,
    );
    expect(teluguAa.strokeOrderSource?.variation).toMatch(
      /hooked bowl.*rounded right lobe.*recombined as ఆ.*Noto Sans Telugu.*handwriting may vary/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఆ")).toBe(false);
    expect(affected.get("ఆ") ?? 0).toBe(0);
    const teluguI = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఇ",
    )!;
    expect(teluguI.sound).toBe("i");
    expect(teluguI.penLifts).toBe(2);
    expect(teluguI.strokeOrder).toEqual([
      "turn around the broad outer bowl",
      "lift and form the compact upper-left lobe",
      "lift again and form the angled upper-right shoulder",
    ]);
    expect(teluguI.strokeOrderSource?.citation).toMatch(
      /Hojaswani LUCIDA and Physics classes.*ఇ decomposition.*00:00–00:05.*14 September 2024/i,
    );
    expect(teluguI.strokeOrderSource?.variation).toMatch(
      /three separated components.*recombined as ఇ.*Noto Sans Telugu.*handwriting may vary/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఇ")).toBe(false);
    expect(affected.get("ఇ") ?? 0).toBe(0);
    const teluguU = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఉ",
    )!;
    expect(teluguU.sound).toBe("u");
    expect(teluguU.penLifts).toBe(2);
    expect(teluguU.strokeOrder).toEqual([
      "sweep left across the rounded upper arch",
      "continue down and around the broad lower bowl",
      "curl upward around the rounded right lobe without lifting",
      "lift and draw the inner horizontal bar from left to right",
      "lift again and draw the short upper headstroke downward",
    ]);
    expect(teluguU.strokeOrderNote).toMatch(
      /five numbered movements.*three pen-down runs.*1.?3.*movement 4.*movement 5/i,
    );
    expect(teluguU.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*independent vowel ఉ.*dot_stroke_v_5_u\.png.*movements 1–5.*version 2\.6/i,
    );
    expect(teluguU.strokeOrderSource?.variation).toMatch(
      /five directional movements.*visible joins.*disconnected printed components.*movements 1.?3.*main body.*movements 4 and 5.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఉ")).toBe(false);
    expect(affected.get("ఉ") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("ఎ")).toBe(false);
    expect(affected.get("ఎ") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("ఏ")).toBe(false);
    expect(affected.get("ఏ") ?? 0).toBe(0);
    const teluguAi = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఐ",
    )!;
    expect(teluguAi.sound).toBe("ai");
    expect(teluguAi.penLifts).toBe(4);
    expect(teluguAi.strokeOrder).toEqual([
      "sweep left across the compact upper arch",
      "restart and curve down around the left bowl",
      "restart and sweep right around the broad lower bowl",
      "restart and sweep left across the upper-right arch",
      "restart and sweep left across the upper-left arch",
    ]);
    expect(teluguAi.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*independent vowel ఐ.*dot_stroke_v_11_ai\.png.*movements 1–5.*version 2\.6/i,
    );
    expect(teluguAi.strokeOrderSource?.variation).toMatch(
      /five disconnected directional movements.*separate pen-down run.*1.?3.*central and lower body.*4.?5.*two upper arches.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఐ")).toBe(false);
    expect(affected.get("ఐ") ?? 0).toBe(0);
    const teluguVocalicR = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఋ",
    )!;
    expect(teluguVocalicR.sound).toBe("r̥");
    expect(teluguVocalicR.penLifts).toBe(5);
    expect(teluguVocalicR.strokeOrder).toEqual([
      "sweep right across the upper shoulder",
      "restart and curve down around the left bowl",
      "restart and sweep right around the lower bowl",
      "restart and curl up around the first right lobe",
      "restart and curl up around the middle lobe",
      "restart and curl up around the final lobe",
    ]);
    expect(teluguVocalicR.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*independent vowel ఋ.*dot_stroke_v_7_ru\.png.*movements 1–6.*version 2\.6/i,
    );
    expect(teluguVocalicR.strokeOrderSource?.variation).toMatch(
      /six disconnected directional movements.*separate pen-down run.*1.?3.*broad left body.*4.?6.*three successive right-side curls.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఋ")).toBe(false);
    expect(affected.get("ఋ") ?? 0).toBe(0);
  },
};
