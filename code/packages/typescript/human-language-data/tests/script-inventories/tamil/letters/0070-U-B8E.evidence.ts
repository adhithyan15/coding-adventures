import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B8E",
  section: "letters",
  id: "U-B8E",
  digest: "aa2f214b385bd626ec809dbc2ba14116796b0d40f9781355601b4c5315f68761",
  assert(entry) {
    const tamilIndependentE = entry as Letter;
    expect(tamilIndependentE.sound).toBe("e");
    expect(tamilIndependentE.role).toBe("independent-vowel");
    expect(tamilIndependentE.penLifts).toBe(0);
    expect(tamilIndependentE.strokeOrder).toHaveLength(4);
    expect(tamilIndependentE.strokeOrder?.[0]).toMatch(
      /inner end of the curl.*clockwise/i,
    );
    expect(tamilIndependentE.strokeOrder?.[3]).toMatch(
      /upright and draw it straight down.*only now lift/i,
    );
    expect(tamilIndependentE.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilIndependentE.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 5.*எ.*University of Texas at Austin.*p\. 193; reordered, and drawn in one stroke, after HP Labs India.*class 6 \(எ\)/i,
    );
    expect(tamilIndependentE.strokeOrderSource?.variation).toMatch(
      /Frame 5.*after a lift.*81 of the 88 stored prototypes of எ \(92%\) are one pen-down stroke.*overrides Frame 5's order and its lift.*varies by school/i,
    );
  },
});
