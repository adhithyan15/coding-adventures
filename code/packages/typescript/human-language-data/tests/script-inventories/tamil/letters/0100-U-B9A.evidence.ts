import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B9A",
  section: "letters",
  id: "U-B9A",
  digest: "1f368f38b6baf7b6c17a9dedc8d553d4a6aded2498711906b93c47c568879075",
  assert(entry) {
    const tamilCa = entry as Letter;
    expect(tamilCa.strokeOrder).toEqual([
      "start at the middle left and climb the left upright",
      "without lifting, carry the top bar to the right and return along it to the inner corner",
      "without lifting, drop the inner upright and carry the middle bar right — then lift once",
      "set the pen at the inner crossing, curve down and around the lower-left bowl, return up its outer left side, and close the bowl at the crossing — and only now lift",
    ]);
    expect(tamilCa.penLifts).toBe(1);
    expect(tamilCa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilCa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*ச.*p\. 191/i,
    );
    expect(tamilCa.strokeOrderSource?.variation).toMatch(
      /three joined upper-frame movements.*separate fourth movement.*lower-left bowl.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
  },
});
