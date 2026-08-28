import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B8E",
  section: "letters",
  id: "U-B8E",
  digest: "e35d476548edfe94b10d12df7e4e7be1c3f5ed61e09008f8c2c90a9da809f535",
  assert(entry) {
    const tamilIndependentE = entry as Letter;
    expect(tamilIndependentE.sound).toBe("e");
    expect(tamilIndependentE.penLifts).toBe(1);
    expect(tamilIndependentE.strokeOrder).toHaveLength(7);
    expect(tamilIndependentE.strokeOrder?.[5]).toMatch(
      /lower foot right.*lift once/i,
    );
    expect(tamilIndependentE.strokeOrder?.[6]).toMatch(
      /separate right upright.*straight up/i,
    );
    expect(tamilIndependentE.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilIndependentE.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 5.*எ.*University of Texas at Austin.*p\. 193/i,
    );
    expect(tamilIndependentE.strokeOrderSource?.variation).toMatch(
      /first six movements.*connected body.*upward right upright.*movement 7.*one lift.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
  },
});
