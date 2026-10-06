import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BA9",
  section: "letters",
  id: "U-BA9",
  digest: "a248da446e6ff57ed0c33c24b37745ebf9d2557c96051fc763567903d7622be3",
  assert(entry) {
    const tamilNnna = entry as Letter;
    expect(tamilNnna.strokeOrder).toEqual([
      "start in the lower-left curl and spiral outward, climbing the outer left side",
      "without lifting, arch over the top of the left loop to the middle junction",
      "without lifting, descend around the outside of the single inner arch",
      "without lifting, turn through its bottom and climb the inside back to the top junction",
      "without lifting, carry the top bar to the right edge",
      "without lifting, come back along the bar to the right upright and draw it straight down — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilNnna.penLifts).toBe(0);
    for (const step of tamilNnna.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilNnna.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilNnna.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 13.*ன.*p\. 195/i,
    );
    expect(tamilNnna.strokeOrderSource?.variation).toMatch(
      /no single national stroke-order standard.*Frame 13 numbers six hand-movements for ன.*one continuous stroke.*in order without lifting.*comes back along it to the right upright.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*94% of the 335 stored prototypes of ன are a single pen-down stroke/i,
    );
  },
});
