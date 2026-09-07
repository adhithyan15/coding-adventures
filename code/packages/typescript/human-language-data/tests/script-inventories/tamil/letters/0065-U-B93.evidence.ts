import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B93",
  section: "letters",
  id: "U-B93",
  digest: "96b0613c1de13faa7daf9268a801e81a000746ad2065598bd9e53a87189abd12",
  assert(entry) {
    const tamilIndependentLongO = entry as Letter;
    expect(tamilIndependentLongO.sound).toBe("ō");
    expect(tamilIndependentLongO.role).toBe("independent-vowel");
    expect(tamilIndependentLongO.penLifts).toBe(1);
    expect(tamilIndependentLongO.strokeOrder).toHaveLength(3);
    expect(tamilIndependentLongO.strokeOrderSource?.url).toContain("module-15");
    expect(tamilIndependentLongO.strokeOrderSource?.citation).toMatch(
      /Module 15.*ஓ.*Appendix I.*Frame 15.*p\. 196/i,
    );
    expect(tamilIndependentLongO.strokeOrderSource?.variation).toMatch(
      /long o.*three movements.*left loop.*large right loop.*joined.*hooked lower bowl.*one lift.*two-run.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
