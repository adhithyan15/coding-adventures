import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B8F",
  section: "letters",
  id: "U-B8F",
  digest: "04b364d436f693e90dd5b54a623ad2ec18eaed09f40bff5d038956e7f0730580",
  assert(entry) {
    const tamilIndependentLongE = entry as Letter;
    expect(tamilIndependentLongE.sound).toBe("ē");
    expect(tamilIndependentLongE.role).toBe("independent-vowel");
    expect(tamilIndependentLongE.penLifts).toBe(0);
    expect(tamilIndependentLongE.strokeOrder).toHaveLength(6);
    expect(tamilIndependentLongE.strokeOrderSource?.url).toContain("module-07");
    expect(tamilIndependentLongE.strokeOrderSource?.citation).toMatch(
      /Module 7.*ஏ.*Appendix I.*Frame 7.*p\. 193/i,
    );
    expect(tamilIndependentLongE.strokeOrderSource?.variation).toMatch(
      /long e.*six.*connected movements.*curl.*outer loop.*top bar.*descending upright.*diagonal foot.*continuous.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
