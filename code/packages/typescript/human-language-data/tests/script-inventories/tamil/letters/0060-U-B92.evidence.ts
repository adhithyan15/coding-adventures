import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B92",
  section: "letters",
  id: "U-B92",
  digest: "9d7a99f10e0e7dfd0c16c656f184e9122a410f37a7b38d854918a0334fed385c",
  assert(entry) {
    const tamilIndependentO = entry as Letter;
    expect(tamilIndependentO.sound).toBe("o");
    expect(tamilIndependentO.role).toBe("independent-vowel");
    expect(tamilIndependentO.penLifts).toBe(1);
    expect(tamilIndependentO.strokeOrder).toHaveLength(3);
    expect(tamilIndependentO.strokeOrderSource?.url).toContain("module-14");
    expect(tamilIndependentO.strokeOrderSource?.citation).toMatch(
      /Module 14.*ஒ.*Appendix I.*Frame 14.*p\. 195/i,
    );
    expect(tamilIndependentO.strokeOrderSource?.variation).toMatch(
      /short o.*three movements.*left loop.*large right loop.*joined.*separate lower bowl.*one lift.*two-run.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
