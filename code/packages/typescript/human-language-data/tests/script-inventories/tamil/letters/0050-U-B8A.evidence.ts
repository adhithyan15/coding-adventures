import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B8A",
  section: "letters",
  id: "U-B8A",
  digest: "dd6f33cabd2bff4b4f04fb3bdb76a553a60eb8768f9737e63b9f5252b2d7e484",
  assert(entry) {
    const tamilIndependentUu = entry as Letter;
    expect(tamilIndependentUu.sound).toBe("ū");
    expect(tamilIndependentUu.role).toBe("independent-vowel");
    expect(tamilIndependentUu.penLifts).toBe(3);
    expect(tamilIndependentUu.strokeOrder).toHaveLength(9);
    expect(tamilIndependentUu.strokeOrderNote).toMatch(
      /four strokes.*three joined movements of உ.*ள.*three-run order/i,
    );
    expect(tamilIndependentUu.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/frame-17/92",
    );
    expect(tamilIndependentUu.strokeOrderSource?.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(tamilIndependentUu.strokeOrderSource?.variation).toMatch(
      /write உ first.*then write ள over it.*Frame 16.*three movements joined.*Frame 12.*six movements.*three pen-down runs.*four-run learner order.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
