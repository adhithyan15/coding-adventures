import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BAF",
  section: "letters",
  id: "U-BAF",
  digest: "2faca9dfe8c48e8b64260e6ce0b3fc47d8b390b5300bec65dfa70c331c0c1b78",
  assert(entry) {
    const tamilYa = entry as Letter;
    expect(tamilYa.strokeOrder).toEqual([
      "start at the top left and descend the left upright",
      "without lifting, round the curved foot and climb into the central upright",
      "without lifting, carry the central upright to the top",
      "without lifting, retrace the central upright back down to the baseline",
      "without lifting, turn right and run along the bottom",
      "without lifting, rise up the right upright — and only now lift",
    ]);
    expect(tamilYa.penLifts).toBe(0);
    expect(tamilYa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilYa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 1.*ய.*p\. 190/i,
    );
    expect(tamilYa.strokeOrderSource?.variation).toMatch(
      /six joined movements.*down the left.*central upright.*across the bottom.*up the right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  },
});
