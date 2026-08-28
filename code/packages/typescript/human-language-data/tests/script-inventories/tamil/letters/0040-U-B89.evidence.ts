import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B89",
  section: "letters",
  id: "U-B89",
  digest: "b95ca4046dbbafe812b47166e975faadb8f80173d7b7b957b5b3f9f4300b9477",
  assert(entry) {
    const tamilIndependentU = entry as Letter;
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
  },
});
