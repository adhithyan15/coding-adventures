import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B93",
  section: "letters",
  id: "U-B93",
  digest: "eb2601d94bfc39bf1b3f97e323079c5230a2f4750a5ef7f170a20619138a84b2",
  assert(entry) {
    const tamilIndependentLongO = entry as Letter;
    expect(tamilIndependentLongO.sound).toBe("ō");
    expect(tamilIndependentLongO.role).toBe("independent-vowel");
    expect(tamilIndependentLongO.strokeOrder).toEqual([
      "circle the small left loop and climb into the crown",
      "without lifting, sweep through the large right loop, curl inward, and run out along the tail",
      "without lifting, come back along the tail to the lower bowl, sweep around its broad curl, and turn inward into the hook — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilIndependentLongO.penLifts).toBe(0);
    for (const step of tamilIndependentLongO.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilIndependentLongO.strokeOrderSource?.url).toContain("module-15");
    expect(tamilIndependentLongO.strokeOrderSource?.citation).toMatch(
      /Module 15.*ஓ.*Appendix I.*Frame 15.*p\. 196/i,
    );
    expect(tamilIndependentLongO.strokeOrderSource?.variation).toMatch(
      /long o.*three movements.*small left loop.*large right loop.*hooked lower bowl.*one continuous stroke.*in order without lifting.*comes back along the tail.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*97% of the 29 stored prototypes of ஓ are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
