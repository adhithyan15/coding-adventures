import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B90",
  section: "letters",
  id: "U-B90",
  digest: "732e4e2d25109e8a885d67bb415840a1b8f476359f2b2fca486dd39f26efa7a3",
  assert(entry) {
    const tamilIndependentAi = entry as Letter;
    expect(tamilIndependentAi.sound).toBe("ai");
    expect(tamilIndependentAi.role).toBe("independent-vowel");
    expect(tamilIndependentAi.strokeOrder).toEqual([
      "start inside the upper-left spiral, curl outward round it, and run down the central upright to its foot",
      "without lifting, draw the central upright back up",
      "without lifting, sweep around the upper-right loop and return left across the middle",
      "without lifting, circle the lower-left bowl back up to the centre",
      "without lifting, come back down the centre stem and circle through the lower-right bowl — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilIndependentAi.penLifts).toBe(0);
    for (const step of tamilIndependentAi.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilIndependentAi.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Writing_Tamil_10.gif",
    );
    expect(tamilIndependentAi.strokeOrderSource?.citation).toMatch(
      /Info-farmer.*Writing Tamil 10.*ஐ.*CC BY-SA 3\.0.*Radhakrishnan.*Frame 11.*p\. 194/i,
    );
    expect(tamilIndependentAi.strokeOrderSource?.variation).toMatch(
      /13-frame.*five parts.*upper-left spiral.*central upright.*upper-right loop.*returning left.*lower-left bowl.*lower-right bowl.*not evidence of pen lifts.*seven movements.*one continuous stroke.*in order without lifting.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*99% of the 169 stored prototypes of ஐ are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
