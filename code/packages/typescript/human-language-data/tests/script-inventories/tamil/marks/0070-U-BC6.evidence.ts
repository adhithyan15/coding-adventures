import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC6",
  section: "marks",
  id: "U-BC6",
  digest: "a91238f6eb162131e39b7a4f55ee0438d2401534fd3b3c134c5451b6a35212c7",
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
  },
});
