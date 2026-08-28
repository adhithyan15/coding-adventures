import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC7",
  section: "marks",
  id: "U-BC7",
  digest: "dec38eacb4d87fc69cc9d8c34b957e624d2ec3b1390aed7e3d3d8f2cf7478551",
  assert(entry) {
    const tamilEe = entry as Mark;
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
  },
});
