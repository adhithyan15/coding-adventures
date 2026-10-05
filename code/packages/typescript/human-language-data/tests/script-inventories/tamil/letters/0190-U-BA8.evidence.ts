import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BA8",
  section: "letters",
  id: "U-BA8",
  digest: "348b02a9a0b48e89ae3335e9d3edee9349a64b6260dec58efe0f63a00cb52269",
  assert(entry) {
    const tamilDentalNa = entry as Letter;
    expect(tamilDentalNa.strokeOrder).toEqual([
      "start at the lower left and draw the left upright upward",
      "without lifting, carry the top bar to the right",
      "without lifting, make the short return left to the middle upright",
      "without lifting, draw the middle upright straight down",
      "without lifting, climb back up to the junction and descend around the outside of the right bowl",
      "without lifting, turn through the bottom and sweep left into the below-baseline tail — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilDentalNa.sound).toBe("na");
    expect(tamilDentalNa.penLifts).toBe(0);
    for (const step of tamilDentalNa.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilDentalNa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilDentalNa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 5.*ந.*University of Texas at Austin.*p\. 193/i,
    );
    expect(tamilDentalNa.strokeOrderSource?.variation).toMatch(
      /Module 5.*voiced dental nasal.*extended final curve may be omitted.*Frame 5 numbers six hand-movements.*one continuous stroke.*in order without lifting.*HP Labs India.*85% of the 224 stored prototypes of ந are a single pen-down stroke/i,
    );
  },
});
