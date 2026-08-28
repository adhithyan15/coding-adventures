import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BAA",
  section: "letters",
  id: "U-BAA",
  digest: "d19e4ffe13b55b7d4d2101ad7ff793b29b263e9b2147a023617da01c2b4118c9",
  assert(entry) {
    const tamilPa = entry as Letter;
    expect(tamilPa.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down to the baseline",
      "without lifting, turn right and run along the bottom to the far right",
      "without lifting, turn upward and finish at the top of the right upright — and only now lift",
    ]);
    expect(tamilPa.penLifts).toBe(0);
    expect(tamilPa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/category/3-moduals/module-01",
    );
    expect(tamilPa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Frame 1.*ப/i,
    );
    expect(tamilPa.strokeOrderSource?.variation).toMatch(
      /left-to-right.*top-to-bottom.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  },
});
