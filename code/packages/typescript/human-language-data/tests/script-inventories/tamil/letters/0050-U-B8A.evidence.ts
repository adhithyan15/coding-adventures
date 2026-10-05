import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B8A",
  section: "letters",
  id: "U-B8A",
  digest: "a4caf3667de38fde1925d8237d2c1496e318ffc23abed50eab4025a09334a40d",
  assert(entry) {
    const tamilIndependentUu = entry as Letter;
    expect(tamilIndependentUu.sound).toBe("ū");
    expect(tamilIndependentUu.role).toBe("independent-vowel");
    expect(tamilIndependentUu.strokeOrder).toEqual([
      "write உ first: start inside its compact upper spiral and sweep outward around it",
      "without lifting, descend through its broad outer curve and turn left onto the baseline",
      "without lifting, carry the long baseline straight to the right — then lift once",
      "write ள over that base: start inside its loop, curl round its inner bowl, and climb the outer loop",
      "without lifting, curve over the top and into the junction with the middle upright",
      "without lifting, draw the adjoining stem straight down",
      "without lifting, rise back up the same stem to the top",
      "without lifting, carry the top bar to the right edge",
      "without lifting, return along the bar to the right upright and draw it straight down — and only now lift",
    ]);
    // உ in one stroke, one lift, then ள in one stroke (Module 17).
    expect(tamilIndependentUu.penLifts).toBe(1);
    tamilIndependentUu.strokeOrder.forEach((step, index) => {
      if (index === 0 || index === 3) return;
      expect(step).toMatch(/^without lifting, /);
    });
    expect(tamilIndependentUu.strokeOrder[3]).toMatch(/^write ள over that base: /);
    expect(tamilIndependentUu.strokeOrderNote).toMatch(
      /two strokes.*உ's three movements without lifting.*lift once.*ள's six movements as one continuous stroke/i,
    );
    expect(tamilIndependentUu.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/frame-17/92",
    );
    expect(tamilIndependentUu.strokeOrderSource?.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(tamilIndependentUu.strokeOrderSource?.variation).toMatch(
      /write உ first.*then write ள over it.*Frame 16.*three movements.*Frame 12.*six.*one continuous stroke.*lifts once.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*93% of the 73 stored prototypes of ஊ are two pen-down strokes.*two-run learner order.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
