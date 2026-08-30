import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B90",
  section: "letters",
  id: "U-B90",
  digest: "64938bbf9f567f9c7d830a71b022c8968a238df602f6151aa513bbeaeb2fb74b",
  assert(entry) {
    const tamilIndependentAi = entry as Letter;
    expect(tamilIndependentAi.sound).toBe("ai");
    expect(tamilIndependentAi.role).toBe("independent-vowel");
    expect(tamilIndependentAi.penLifts).toBe(4);
    expect(tamilIndependentAi.strokeOrder).toHaveLength(5);
    expect(tamilIndependentAi.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Writing_Tamil_10.gif",
    );
    expect(tamilIndependentAi.strokeOrderSource?.citation).toMatch(
      /Info-farmer.*Writing Tamil 10.*ஐ.*CC BY-SA 3\.0.*Radhakrishnan.*Frame 11.*p\. 194/i,
    );
    expect(tamilIndependentAi.strokeOrderSource?.variation).toMatch(
      /13-frame.*five separate runs.*upper-left spiral.*central upright.*upper-right loop.*returning left.*lower-left bowl.*lower-right bowl.*seven movements.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
