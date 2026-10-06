import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BCD",
  section: "marks",
  id: "U-BCD",
  digest: "4b79e980018cd09924cb5731d38476d3ef36fdf98a1e20243baeb6b6c35f42b6",
  assert(entry) {
    const pulli = entry as Mark;
    // Written place: the dot comes AFTER its consonant, once the body is
    // complete. figure-targets.ts' WRITTEN_SIGN_SIDES is held to this record.
    expect(pulli.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "write the pulli sign after it, as a dot above the consonant, once the consonant is complete",
    ]);
    expect(pulli.compositionSource?.url).toBe("https://github.com/abhinayaRajarajan/varai");
    expect(pulli.compositionSource?.citation).toMatch(
      /Abhinaya Rajarajan, Varai.*Swift Student Challenge 2026.*18 Tamil consonants with pulli.*952294fa.*no licence: facts only/,
    );
    expect(pulli.compositionSource?.variation).toMatch(
      /In all 18 recorded drawings.*consonant body is drawn first.*puḷḷi second.*Info-farmer.*confidence medium/,
    );
    // One touch of the pen: the dot by itself has no lift of its own.
    expect(pulli.penLifts).toBe(0);
    expect(pulli.components).toEqual(["a dot"]);
    expect(pulli.strokeOrderSource?.url).toBe("https://github.com/abhinayaRajarajan/varai");
    expect(pulli.strokeOrderSource?.citation).toBe(pulli.compositionSource?.citation);
    expect(pulli.strokeOrderSource?.variation).toMatch(
      /separate second stroke, made after the body.*Confidence is medium.*not a claim.*Noto Sans Tamil/,
    );
  },
});
