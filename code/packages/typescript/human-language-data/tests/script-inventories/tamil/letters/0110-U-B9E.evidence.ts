import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B9E",
  section: "letters",
  id: "U-B9E",
  digest: "49535e6a33e21d17719a82dc75acd7c2d1b4ceac8631de193064de6752c62199",
  assert(entry) {
    const tamilNya = entry as Letter;
    expect(tamilNya.sound).toBe("ña");
    expect(tamilNya.role).toBe("consonant");
    expect(tamilNya.penLifts).toBe(3);
    expect(tamilNya.strokeOrder).toHaveLength(8);
    expect(tamilNya.strokeOrderNote).toMatch(
      /four strokes.*1–2.*inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl/i,
    );
    expect(tamilNya.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 8.*ஞ.*University of Texas at Austin.*p\. 194/i,
    );
    expect(tamilNya.strokeOrderSource?.variation).toMatch(
      /eight movements.*1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
  },
});
