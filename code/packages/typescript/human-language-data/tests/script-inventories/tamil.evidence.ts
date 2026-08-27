// Exact real-corpus evidence owned by the Tamil inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Tamil",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const tamilU = scripts.tamil!.marks!.find((mark) => mark.mark === "ு")!;
    expect(tamilU.role).toBe("vowel-sign");
    expect(tamilU.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "add the u vowel sign to replace its inherent vowel",
    ]);
    expect(tamilU.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(tamilU.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.6\.3.*U\+0BC1.*க \+ ு → கு/i,
    );
    expect(tamilU.compositionSource?.variation).toMatch(
      /encoded carrier-first composition.*normally ligates.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const tamilUu = scripts.tamil!.marks!.find((mark) => mark.mark === "ூ")!;
    expect(tamilUu.role).toBe("vowel-sign");
    expect(tamilUu.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "add the ū vowel sign to replace its inherent vowel",
    ]);
    expect(tamilUu.example).toEqual({ base: "க", combined: "கூ", sound: "kū" });
    expect(tamilUu.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(tamilUu.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.6\.3.*U\+0BC2.*க \+ ூ → கூ/i,
    );
    expect(tamilUu.compositionSource?.variation).toMatch(
      /encoded carrier-first composition.*normally ligates.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const tamilIi = scripts.tamil!.marks!.find((mark) => mark.mark === "ீ")!;
    expect(tamilIi.role).toBe("vowel-sign");
    expect(tamilIi.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "add the ī vowel sign to replace its inherent vowel",
    ]);
    expect(tamilIi.example).toEqual({ base: "ட", combined: "டீ", sound: "ṭī" });
    expect(tamilIi.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(tamilIi.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.6\.3.*Figure 12-21.*U\+0BC0.*ட \+ ீ → டீ.*ல \+ ீ → லீ/i,
    );
    expect(tamilIi.compositionSource?.variation).toMatch(
      /encoded carrier-first composition.*change shape or position.*join cursively.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const tamilIndependentE = scripts.tamil!.letters.find(
      (entry) => entry.glyph === "எ",
    )!;
    expect(tamilIndependentE.sound).toBe("e");
    expect(tamilIndependentE.penLifts).toBe(1);
    expect(tamilIndependentE.strokeOrder).toHaveLength(7);
    expect(tamilIndependentE.strokeOrder?.[5]).toMatch(
      /lower foot right.*lift once/i,
    );
    expect(tamilIndependentE.strokeOrder?.[6]).toMatch(
      /separate right upright.*straight up/i,
    );
    expect(tamilIndependentE.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilIndependentE.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 5.*எ.*University of Texas at Austin.*p\. 193/i,
    );
    expect(tamilIndependentE.strokeOrderSource?.variation).toMatch(
      /first six movements.*connected body.*upward right upright.*movement 7.*one lift.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
    const tamilIndependentU = scripts.tamil!.letters.find(
      (entry) => entry.glyph === "உ",
    )!;
    expect(tamilIndependentU.sound).toBe("u");
    expect(tamilIndependentU.role).toBe("independent-vowel");
    expect(tamilIndependentU.penLifts).toBe(0);
    expect(tamilIndependentU.strokeOrder).toEqual([
      "start inside the upper spiral and sweep outward around it",
      "without lifting, descend through the broad outer curve and turn left onto the baseline",
      "without lifting, carry the long baseline straight to the right",
    ]);
    expect(tamilIndependentU.strokeOrderNote).toMatch(
      /one unbroken stroke.*three joined movements.*no pen lift/i,
    );
    expect(tamilIndependentU.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilIndependentU.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 16.*உ.*University of Texas at Austin.*p\. 196/i,
    );
    expect(tamilIndependentU.strokeOrderSource?.variation).toMatch(
      /Frame 16.*upper spiral.*descending outer curve.*rightward baseline.*three joined movements.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
    const tamilIndependentUu = scripts.tamil!.letters.find(
      (entry) => entry.glyph === "ஊ",
    )!;
    expect(tamilIndependentUu.sound).toBe("ū");
    expect(tamilIndependentUu.role).toBe("independent-vowel");
    expect(tamilIndependentUu.penLifts).toBe(3);
    expect(tamilIndependentUu.strokeOrder).toHaveLength(9);
    expect(tamilIndependentUu.strokeOrderNote).toMatch(/four strokes.*three joined movements of உ.*ள.*three-run order/i);
    expect(tamilIndependentUu.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/frame-17/92",
    );
    expect(tamilIndependentUu.strokeOrderSource?.citation).toMatch(/Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i);
    expect(tamilIndependentUu.strokeOrderSource?.variation).toMatch(/write உ first.*then write ள over it.*Frame 16.*three movements joined.*Frame 12.*six movements.*three pen-down runs.*four-run learner order.*Noto Sans Tamil.*varies by school/i);
    const tamilIndependentO = scripts.tamil!.letters.find((entry) => entry.glyph === "ஒ")!;
    expect(tamilIndependentO.sound).toBe("o");
    expect(tamilIndependentO.role).toBe("independent-vowel");
    expect(tamilIndependentO.penLifts).toBe(1);
    expect(tamilIndependentO.strokeOrder).toHaveLength(3);
    expect(tamilIndependentO.strokeOrderSource?.url).toContain("module-14");
    expect(tamilIndependentO.strokeOrderSource?.citation).toMatch(/Module 14.*ஒ.*Appendix I.*Frame 14.*p\. 195/i);
    expect(tamilIndependentO.strokeOrderSource?.variation).toMatch(/short o.*three movements.*left loop.*large right loop.*joined.*separate lower bowl.*one lift.*two-run.*Noto Sans Tamil.*varies by school/i);
    expect(missingByScript.get("tamil.json")?.has("ஒ")).toBe(false);
    expect(affected.get("ஒ") ?? 0).toBe(0);
    const tamilNga = scripts.tamil!.letters.find(
      (entry) => entry.glyph === "ங",
    )!;
    expect(tamilNga.sound).toBe("ṅa");
    expect(tamilNga.role).toBe("consonant");
    expect(tamilNga.penLifts).toBe(1);
    expect(tamilNga.strokeOrder).toEqual([
      "draw the detached upright straight down — then lift once",
      "set the pen low on the left and climb the tall body",
      "without lifting, carry the top bar right and return to the inner upright",
      "without lifting, descend into the rounded inner turn",
      "without lifting, carry the low bar to the right",
      "without lifting, return along the low bar to the left and finish up the inner stem — and only now lift",
    ]);
    expect(tamilNga.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 2.*ங.*University of Texas at Austin.*p\. 191/i,
    );
    expect(tamilNga.strokeOrderSource?.variation).toMatch(
      /detached descending upright.*five joined movements.*Noto Sans Tamil.*detached upright on the right.*varies by school.*two-run order/i,
    );
    const tamilNya = scripts.tamil!.letters.find((entry) => entry.glyph === "ஞ")!;
    expect(tamilNya.sound).toBe("ña");
    expect(tamilNya.role).toBe("consonant");
    expect(tamilNya.penLifts).toBe(3);
    expect(tamilNya.strokeOrder).toHaveLength(8);
    expect(tamilNya.strokeOrderNote).toMatch(/four strokes.*1–2.*inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl/i);
    expect(tamilNya.strokeOrderSource?.citation).toMatch(/Tamil Script Learners Manual.*Appendix I.*Frame 8.*ஞ.*University of Texas at Austin.*p\. 194/i);
    expect(tamilNya.strokeOrderSource?.variation).toMatch(/eight movements.*1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*varies by school.*four-run order.*Noto Sans Tamil/i);
    const tamilRetroflexLa = scripts.tamil!.letters.find(
      (entry) => entry.glyph === "ள",
    )!;
    expect(tamilRetroflexLa.sound).toBe("ḷa");
    expect(tamilRetroflexLa.penLifts).toBe(2);
    expect(tamilRetroflexLa.strokeOrder).toHaveLength(6);
    expect(tamilRetroflexLa.strokeOrder?.[2]).toMatch(
      /adjoining stem straight down.*lift once/i,
    );
    expect(tamilRetroflexLa.strokeOrder?.[4]).toMatch(
      /top bar to the right edge.*lift a second time/i,
    );
    expect(tamilRetroflexLa.strokeOrder?.[5]).toMatch(
      /separate right upright.*straight down/i,
    );
    expect(tamilRetroflexLa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRetroflexLa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 12.*ள.*University of Texas at Austin.*p\. 195/i,
    );
    expect(tamilRetroflexLa.strokeOrderSource?.variation).toMatch(
      /Module 12.*retroflex lateral.*contrasts it with ல.*six movements.*three pen-down runs.*1.?3.*4.?5.*movement 6/i,
    );
    const tamilDentalNa = scripts.tamil!.letters.find(
      (entry) => entry.glyph === "ந",
    )!;
    expect(tamilDentalNa.sound).toBe("na");
    expect(tamilDentalNa.penLifts).toBe(2);
    expect(tamilDentalNa.strokeOrder).toHaveLength(6);
    expect(tamilDentalNa.strokeOrder?.[1]).toMatch(
      /top bar to the right.*lift once/i,
    );
    expect(tamilDentalNa.strokeOrder?.[3]).toMatch(
      /middle upright straight down.*lift a second time/i,
    );
    expect(tamilDentalNa.strokeOrder?.[5]).toMatch(
      /sweep left.*below-baseline tail/i,
    );
    expect(tamilDentalNa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilDentalNa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 5.*ந.*University of Texas at Austin.*p\. 193/i,
    );
    expect(tamilDentalNa.strokeOrderSource?.variation).toMatch(
      /Module 5.*voiced dental nasal.*extended final curve may be omitted.*six movements.*three pen-down runs.*1.?2.*3.?4.*5.?6/i,
    );
    const tamilE = scripts.tamil!.marks!.find((mark) => mark.mark === "ெ")!;
    expect(tamilE.compositionOrder).toEqual([
      "in handwriting, write the e vowel sign to the left before the primary consonant",
      "write the Tamil consonant carrier after it; read the result as consonant plus e",
    ]);
    expect(tamilE.example).toEqual({ base: "க", combined: "கெ", sound: "ke" });
    expect(tamilE.compositionSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/category/3-moduals/module-06",
    );
    expect(tamilE.compositionSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Module 6.*Frame 6.*secondary symbol for short e.*always placed before the primary letter.*University of Texas at Austin.*2009/i,
    );
    expect(tamilE.compositionSource?.variation).toMatch(
      /handwritten sign-before-carrier order.*left-side placement.*does not supply a standalone directional path or pen-lift count.*no ductus is inferred/i,
    );
    const tamilEe = scripts.tamil!.marks!.find((mark) => mark.mark === "ே")!;
    expect(tamilEe.compositionOrder).toEqual([
      "in handwriting, write the ē vowel sign to the left before the primary consonant",
      "write the Tamil consonant carrier after it; read the result as consonant plus ē",
    ]);
    expect(tamilEe.example).toEqual({ base: "க", combined: "கே", sound: "kē" });
    expect(tamilEe.compositionSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/category/3-moduals/module-07",
    );
    expect(tamilEe.compositionSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Module 7.*Frame 7.*secondary symbol for ē.*written before the primary consonant.*University of Texas at Austin.*2009/i,
    );
    expect(tamilEe.compositionSource?.variation).toMatch(
      /handwritten sign-before-carrier order.*left-side placement.*does not supply a standalone directional path or pen-lift count.*no ductus is inferred/i,
    );
    const tamilPa = scripts.tamil!.letters.find(
      (letter) => letter.glyph === "ப",
    )!;
    expect(tamilPa.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down to the baseline",
      "without lifting, turn right and run along the bottom to the far right",
      "without lifting, turn upward and finish at the top of the right upright — and only now lift",
    ]);
    expect(tamilPa.penLifts).toBe(0);
    expect(tamilPa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/category/3-moduals/module-01",
    );
    expect(tamilPa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Frame 1.*ப/i,
    );
    expect(tamilPa.strokeOrderSource?.variation).toMatch(
      /left-to-right.*top-to-bottom.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
    const tamilTta = scripts.tamil!.letters.find(
      (letter) => letter.glyph === "ட",
    )!;
    expect(tamilTta.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down",
      "without lifting, turn right and carry the long foot to the far edge — and only now lift",
    ]);
    expect(tamilTta.penLifts).toBe(0);
    expect(tamilTta.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilTta.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 1.*ட.*p\. 190/i,
    );
    expect(tamilTta.strokeOrderSource?.variation).toMatch(
      /left descent.*rightward foot.*two joined movements.*Module 1 identifies.*top-to-bottom.*left-to-right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
    const tamilTha = scripts.tamil!.letters.find(
      (letter) => letter.glyph === "த",
    )!;
    expect(tamilTha.strokeOrder).toEqual([
      "start at the middle left, climb the short upright, and carry the top bar to the right — then lift once",
      "restart at the central crossing, carry the short upper bar right, and curve down around the broad right bowl — then lift a second time",
      "restart at the middle left, turn around the compact left loop, and curl back to the central crossing — then lift a third time",
      "restart at the lower right and sweep the low tail left",
    ]);
    expect(tamilTha.penLifts).toBe(3);
    expect(tamilTha.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilTha.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*த.*p\. 192/i,
    );
    expect(tamilTha.strokeOrderSource?.variation).toMatch(
      /Module 3 identifies.*dental stop.*final Frame 3 row.*four separate pen-down runs.*1–2.*upper frame.*3–4.*right bowl.*5–6.*left loop.*movement 7.*leftward tail.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
    const tamilRa = scripts.tamil!.letters.find(
      (letter) => letter.glyph === "ர",
    )!;
    expect(tamilRa.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down — then lift once",
      "set the pen at the top left and carry the top bar to the right — then lift a second time",
      "set the pen at the middle top and draw the central upright down",
      "without lifting again, add the short angular tail down-left and hook its tip down-right — and only now lift",
    ]);
    expect(tamilRa.penLifts).toBe(2);
    expect(tamilRa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*ர/i,
    );
    expect(tamilRa.strokeOrderSource?.variation).toMatch(
      /three-movement ஈ frame.*angular short fourth movement.*varies by school.*three-run order.*Noto Sans Tamil/i,
    );
    const tamilCa = scripts.tamil!.letters.find(
      (letter) => letter.glyph === "ச",
    )!;
    expect(tamilCa.strokeOrder).toEqual([
      "start at the middle left and climb the left upright",
      "without lifting, carry the top bar to the right and return along it to the inner corner",
      "without lifting, drop the inner upright and carry the middle bar right — then lift once",
      "set the pen at the inner crossing, curve down and around the lower-left bowl, return up its outer left side, and close the bowl at the crossing — and only now lift",
    ]);
    expect(tamilCa.penLifts).toBe(1);
    expect(tamilCa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilCa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*ச.*p\. 191/i,
    );
    expect(tamilCa.strokeOrderSource?.variation).toMatch(
      /three joined upper-frame movements.*separate fourth movement.*lower-left bowl.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
    const tamilYa = scripts.tamil!.letters.find(
      (letter) => letter.glyph === "ய",
    )!;
    expect(tamilYa.strokeOrder).toEqual([
      "start at the top left and descend the left upright",
      "without lifting, round the curved foot and climb into the central upright",
      "without lifting, carry the central upright to the top",
      "without lifting, retrace the central upright back down to the baseline",
      "without lifting, turn right and run along the bottom",
      "without lifting, rise up the right upright — and only now lift",
    ]);
    expect(tamilYa.penLifts).toBe(0);
    expect(tamilYa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilYa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 1.*ய.*p\. 190/i,
    );
    expect(tamilYa.strokeOrderSource?.variation).toMatch(
      /six joined movements.*down the left.*central upright.*across the bottom.*up the right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
    expect(missingByScript.get("tamil.json")?.has("ு")).toBe(false);
    expect(affected.get("ு") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ூ")).toBe(false);
    expect(affected.get("ூ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ீ")).toBe(false);
    expect(affected.get("ீ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ப")).toBe(false);
    expect(affected.get("ப") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("த")).toBe(false);
    expect(affected.get("த") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ர")).toBe(false);
    expect(affected.get("ர") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ய")).toBe(false);
    expect(affected.get("ய") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ச")).toBe(false);
    expect(affected.get("ச") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ட")).toBe(false);
    expect(affected.get("ட") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ே")).toBe(false);
    expect(affected.get("ே") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ெ")).toBe(false);
    expect(affected.get("ெ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ள")).toBe(false);
    expect(affected.get("ள") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("எ")).toBe(false);
    expect(affected.get("எ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ழ")).toBe(false);
    expect(affected.get("ழ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("உ")).toBe(false);
    expect(affected.get("உ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ஊ")).toBe(false);
    expect(affected.get("ஊ") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ங")).toBe(false);
    expect(affected.get("ங") ?? 0).toBe(0);
    expect(missingByScript.get("tamil.json")?.has("ஞ")).toBe(false);
    expect(affected.get("ஞ") ?? 0).toBe(0);
  },
};
