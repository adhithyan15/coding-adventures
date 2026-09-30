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
    expect(missingByScript.get("telugu.json")?.has("్") ?? false).toBe(false);
    expect(affected.get("్") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("ం") ?? false).toBe(false);
    expect(affected.get("ం") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("అ") ?? false).toBe(false);
    expect(affected.get("అ") ?? 0).toBe(0);
    const teluguKa = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "క",
    )!;
    expect(teluguKa.sound).toBe("ka");
    expect(teluguKa.penLifts).toBe(1);
    expect(teluguKa.strokeOrder).toEqual([
      "turn down and left around the upper bowl",
      "continue right through the middle shoulder",
      "curve down and left around the lower bowl",
      "finish upward along the left tail",
      "restart and sweep up through the separate headstroke",
    ]);
    expect(teluguKa.strokeOrderNote).toMatch(
      /five numbered movements.*two pen-down runs.*1.?4.*movement 5/i,
    );
    expect(teluguKa.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguKa.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*క.*dot_stroke_c_1_1_ka\.png.*movements 1.?5.*version 2\.6/i,
    );
    expect(teluguKa.strokeOrderSource?.variation).toMatch(
      /five directional movements.*two pen-down starts.*1.?4.*main body.*movement 5.*headstroke.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("క") ?? false).toBe(false);
    expect(affected.get("క") ?? 0).toBe(0);
    const teluguKha = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "ఖ",
    )!;
    expect(teluguKha.sound).toBe("kha");
    expect(teluguKha.penLifts).toBe(1);
    expect(teluguKha.strokeOrder).toEqual([
      "circle up around the upper-left bowl",
      "descend through the central curve",
      "turn up around the left shoulder",
      "sweep right and up around the broad outer bowl",
      "return left along the crown",
      "restart and draw the separate downward stem",
    ]);
    expect(teluguKha.strokeOrderNote).toMatch(
      /six numbered movements.*two pen-down runs.*1.?5.*movement 6/i,
    );
    expect(teluguKha.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguKha.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఖ.*dot_stroke_c_1_2_kha\.png.*movements 1.?6.*version 2\.6/i,
    );
    expect(teluguKha.strokeOrderSource?.variation).toMatch(
      /six directional movements.*1.?5.*main body.*movement 6.*downward stem.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఖ") ?? false).toBe(false);
    expect(affected.get("ఖ") ?? 0).toBe(0);
    const teluguGa = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "గ",
    )!;
    expect(teluguGa.sound).toBe("ga");
    expect(teluguGa.penLifts).toBe(1);
    expect(teluguGa.strokeOrder).toEqual([
      "sweep up and over the broad lower arch",
      "restart and cup through the separate upper flourish",
    ]);
    expect(teluguGa.strokeOrderNote).toMatch(
      /two numbered movements.*two pen-down runs.*movement 1.*lower arch.*movement 2.*upper flourish/i,
    );
    expect(teluguGa.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguGa.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*గ.*dot_stroke_c_1_3_ga\.png.*movements 1.?2.*version 2\.6/i,
    );
    expect(teluguGa.strokeOrderSource?.variation).toMatch(
      /lower arch.*movement 1.*upper flourish.*movement 2.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("గ") ?? false).toBe(false);
    expect(affected.get("గ") ?? 0).toBe(0);
    const teluguGha = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "ఘ",
    )!;
    expect(teluguGha.sound).toBe("gha");
    expect(teluguGha.penLifts).toBe(3);
    expect(teluguGha.strokeOrder).toEqual([
      "sweep left around the upper-left shoulder",
      "continue down and right around the lower-left bowl",
      "turn upward around the broad middle arch",
      "sweep right and up around the outer arch",
      "restart and cup through the upper flourish",
      "restart and draw the separate downward stem",
    ]);
    expect(teluguGha.strokeOrderNote).toMatch(
      /six numbered movements.*four pen-down runs.*1.?3.*movement 4.*movement 5.*movement 6/i,
    );
    expect(teluguGha.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguGha.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఘ.*dot_stroke_c_1_4_gha\.png.*movements 1.?6.*version 2\.6/i,
    );
    expect(teluguGha.strokeOrderSource?.variation).toMatch(
      /three directional movements.*body.*movement 4.*outer arch.*movement 5.*upper flourish.*movement 6.*downward stem.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఘ") ?? false).toBe(false);
    expect(affected.get("ఘ") ?? 0).toBe(0);
    const teluguNga = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "ఙ",
    )!;
    expect(teluguNga.sound).toBe("ṅa");
    expect(teluguNga.penLifts).toBe(2);
    expect(teluguNga.strokeOrder).toEqual([
      "turn around the compact upper-left lobe",
      "continue down and around the broad lower bowl",
      "curl upward around the rounded right lobe",
      "lift and draw the inner horizontal bar from left to right",
      "lift again and draw the short upper headstroke downward",
    ]);
    expect(teluguNga.strokeOrderNote).toMatch(
      /five numbered movements.*three pen-down runs.*1.?3.*movement 4.*movement 5/i,
    );
    expect(teluguNga.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguNga.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఙ.*dot_stroke_c_1_5_nya\.png.*movements 1.?5.*version 2\.6/i,
    );
    expect(teluguNga.strokeOrderSource?.variation).toMatch(
      /three joined directional movements.*main body.*movement 4.*horizontal bar.*movement 5.*headstroke.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఙ") ?? false).toBe(false);
    expect(affected.get("ఙ") ?? 0).toBe(0);
    const teluguCa = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "చ",
    )!;
    expect(teluguCa.sound).toBe("ca");
    expect(teluguCa.penLifts).toBe(1);
    expect(teluguCa.strokeOrder).toEqual([
      "draw the upper bar from left to right",
      "continue down and around the left bowl",
      "sweep right and up around the outer bowl",
      "restart and cup through the upper flourish",
    ]);
    expect(teluguCa.strokeOrderNote).toMatch(
      /four numbered movements.*two pen-down runs.*1.?3.*movement 4/i,
    );
    expect(teluguCa.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguCa.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*చ.*dot_stroke_c_2_1_cha\.png.*movements 1.?4.*version 2\.6/i,
    );
    expect(teluguCa.strokeOrderSource?.variation).toMatch(
      /three directional movements.*main body.*movement 4.*upper flourish.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("చ") ?? false).toBe(false);
    expect(affected.get("చ") ?? 0).toBe(0);
    const teluguChha = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "ఛ",
    )!;
    expect(teluguChha.sound).toBe("cha");
    expect(teluguChha.penLifts).toBe(1);
    expect(teluguChha.strokeOrder).toEqual([
      "draw the upper bar from left to right",
      "continue down and around the left bowl",
      "sweep right and up around the outer bowl",
      "continue through the upper flourish",
      "restart and draw the separate downward stem",
    ]);
    expect(teluguChha.strokeOrderNote).toMatch(
      /five numbered movements.*two pen-down runs.*1.?4.*movement 5/i,
    );
    expect(teluguChha.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguChha.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఛ.*dot_stroke_c_2_2_chha\.png.*movements 1.?5.*version 2\.6/i,
    );
    expect(teluguChha.strokeOrderSource?.variation).toMatch(
      /one pen-down start.*1.?4.*main body.*upper flourish.*second start.*movement 5.*downward stem.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఛ") ?? false).toBe(false);
    expect(affected.get("ఛ") ?? 0).toBe(0);
    const teluguJha = scripts.telugu!.letters.find(
      (entry) => entry.glyph === "ఝ",
    )!;
    expect(teluguJha.sound).toBe("jha");
    expect(teluguJha.penLifts).toBe(4);
    expect(teluguJha.strokeOrder).toEqual([
      "circle clockwise around the left bowl",
      "circle clockwise around the middle bowl",
      "circle clockwise around the right bowl",
      "curl upward through the top flourish",
      "restart and draw the separate downward stem",
    ]);
    expect(teluguJha.strokeOrderNote).toMatch(
      /five separately numbered movements.*five pen-down runs.*1.?3.*movement 4.*movement 5/i,
    );
    expect(teluguJha.strokeOrderSource?.url).toBe(
      "https://play.google.com/store/apps/details?id=com.sathishshanmugam.writetelugualphabets",
    );
    expect(teluguJha.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*Write Telugu Alphabets.*ఝ.*dot_stroke_c_2_4_jha\.png.*line_stroke_c_2_4_jha\.png.*movements 1.?5.*version 2\.6/i,
    );
    expect(teluguJha.strokeOrderSource?.variation).toMatch(
      /three round sections.*upper flourish.*downward stem.*five movements.*not uniform.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఝ") ?? false).toBe(false);
    expect(affected.get("ఝ") ?? 0).toBe(0);
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
    expect(missingByScript.get("telugu.json")?.has("ఆ") ?? false).toBe(false);
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
    expect(missingByScript.get("telugu.json")?.has("ఇ") ?? false).toBe(false);
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
    expect(missingByScript.get("telugu.json")?.has("ఉ") ?? false).toBe(false);
    expect(affected.get("ఉ") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("ఎ") ?? false).toBe(false);
    expect(affected.get("ఎ") ?? 0).toBe(0);
    expect(missingByScript.get("telugu.json")?.has("ఏ") ?? false).toBe(false);
    expect(affected.get("ఏ") ?? 0).toBe(0);
    const teluguO = scripts.telugu!.independentVowels!.find(
      (entry) => entry.glyph === "ఒ",
    )!;
    expect(teluguO.sound).toBe("o");
    expect(teluguO.penLifts).toBe(2);
    expect(teluguO.strokeOrder).toEqual([
      "sweep right across the upper arch",
      "restart and curve down around the left bowl",
      "restart and sweep right around the broad lower bowl",
    ]);
    expect(teluguO.strokeOrderSource?.citation).toMatch(
      /Sathish Shanmugam.*independent vowel ఒ.*dot_stroke_v_12_o\.png.*movements 1–3.*version 2\.6/i,
    );
    expect(teluguO.strokeOrderSource?.variation).toMatch(
      /three disconnected directional movements.*separate pen-down run.*movement 1.*upper arch.*movement 2.*left bowl.*movement 3.*broad lower bowl.*Noto Sans Telugu/i,
    );
    expect(missingByScript.get("telugu.json")?.has("ఒ") ?? false).toBe(false);
    expect(affected.get("ఒ") ?? 0).toBe(0);
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
    expect(missingByScript.get("telugu.json")?.has("ఐ") ?? false).toBe(false);
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
    expect(missingByScript.get("telugu.json")?.has("ఋ") ?? false).toBe(false);
    expect(affected.get("ఋ") ?? 0).toBe(0);
  },
};
