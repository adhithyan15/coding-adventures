import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B99",
  section: "letters",
  id: "U-B99",
  digest: "537d7eaae066d3d52117fac53e6ae3c9c199bc8f3173747e5c85a675d1d49e86",
  assert(entry) {
    const tamilNga = entry as Letter;
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
  },
});
