import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B92",
  section: "letters",
  id: "U-B92",
  digest: "33b3102dbff28bf02e275f4506cde7e2b52ae6aa5cc60ab72f3ad107abf510af",
  assert(entry) {
    const tamilIndependentO = entry as Letter;
    expect(tamilIndependentO.sound).toBe("o");
    expect(tamilIndependentO.role).toBe("independent-vowel");
    expect(tamilIndependentO.strokeOrder).toEqual([
      "circle the small left loop and climb into the crown",
      "without lifting, sweep through the large right loop, curl inward, and run out along the tail",
      "without lifting, come back along the tail to the lower bowl, draw it down and around, and return left — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilIndependentO.penLifts).toBe(0);
    for (const step of tamilIndependentO.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilIndependentO.strokeOrderSource?.url).toContain("module-14");
    expect(tamilIndependentO.strokeOrderSource?.citation).toMatch(
      /Module 14.*ஒ.*Appendix I.*Frame 14.*p\. 195/i,
    );
    expect(tamilIndependentO.strokeOrderSource?.variation).toMatch(
      /short o.*three movements.*small left loop.*large right loop.*lower bowl.*one continuous stroke.*in order without lifting.*comes back along the tail.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*99% of the 115 stored prototypes of ஒ are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
