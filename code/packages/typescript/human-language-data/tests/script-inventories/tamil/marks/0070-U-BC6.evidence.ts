import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC6",
  section: "marks",
  id: "U-BC6",
  digest: "cb41b243b8cd81f84b1bf89597509574dadc1a2448ff3539d0cfbf8d108a9fcc",
  assert(entry) {
    const tamilE = entry as Mark;
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
    // One continuous stroke of three movements, from native writers' traces.
    expect(tamilE.penLifts).toBe(0);
    expect(tamilE.strokeOrder).toEqual([
      "start inside the small curl and circle it",
      "without lifting, climb the outer curve over the top",
      "without lifting, draw the right upright down — and only now lift",
    ]);
    expect(tamilE.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilE.strokeOrderSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 41 \(ெ, e sign\).*native Tamil writers.*MIT licence/,
    );
    expect(tamilE.strokeOrderSource?.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil/);
  },
});
