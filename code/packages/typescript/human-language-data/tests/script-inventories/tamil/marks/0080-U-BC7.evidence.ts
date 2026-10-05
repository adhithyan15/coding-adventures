import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC7",
  section: "marks",
  id: "U-BC7",
  digest: "7e18dddfc503bf417bb8d0f1cfffb9c374c4e12eb881da702368e3d616d1365a",
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
    // One continuous stroke of three movements, from native writers' traces.
    expect(tamilEe.penLifts).toBe(0);
    expect(tamilEe.strokeOrder).toEqual([
      "start inside the small upper loop and circle it",
      "without lifting, sweep left and down the big curve",
      "without lifting, curl up into the lower loop — and only now lift",
    ]);
    expect(tamilEe.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilEe.strokeOrderSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 42 \(ே, ee sign\).*native Tamil writers.*MIT licence/,
    );
    expect(tamilEe.strokeOrderSource?.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil/);
  },
});
