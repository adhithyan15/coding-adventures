import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BA3",
  section: "letters",
  id: "U-BA3",
  digest: "ab0758eca65a7efba1c9fd15b62a17c4cce7543d06a2dea54dc6bac619bd08ce",
  assert(entry) {
    const tamilNna = entry as Letter;
    expect(tamilNna.strokeOrder).toEqual([
      "start in the lower-left curl and spiral outward, climbing the outer left side",
      "without lifting, arch over the top of the left loop to the first junction",
      "without lifting, descend around the outside of the first inner arch",
      "without lifting, turn through its bottom and climb the inside back to the first junction",
      "without lifting, sweep through the extra inner arch — over its top, around its outside and bottom, then up its inside to the right junction",
      "without lifting, carry the top bar to the right edge",
      "without lifting, come back along the bar to the right upright and draw it straight down — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilNna.penLifts).toBe(0);
    for (const step of tamilNna.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilNna.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilNna.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 13.*ண.*p\. 195/i,
    );
    expect(tamilNna.strokeOrderSource?.variation).toMatch(
      /no single national stroke-order standard.*Frame 13 numbers seven hand-movements for ண.*one continuous stroke.*in order without lifting.*comes back along it to the right upright.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*93% of the 212 stored prototypes of ண are a single pen-down stroke/i,
    );
  },
});
