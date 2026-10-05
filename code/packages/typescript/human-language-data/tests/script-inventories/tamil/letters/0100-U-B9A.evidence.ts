import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B9A",
  section: "letters",
  id: "U-B9A",
  digest: "b960b933e232e60519de4ebd5b8ec1a8405fcff29ff616bfe5d4f6400797db43",
  assert(entry) {
    const tamilCa = entry as Letter;
    expect(tamilCa.strokeOrder).toEqual([
      "start at the middle left and climb the left upright",
      "without lifting, carry the top bar to the right and return along it to the inner corner",
      "without lifting, drop the inner upright and carry the middle bar right",
      "without lifting, come back along the middle bar to the inner crossing, curve down and around the lower-left bowl, return up its outer left side, and close the bowl at the crossing — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilCa.penLifts).toBe(0);
    for (const step of tamilCa.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilCa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilCa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*ச.*p\. 191/i,
    );
    expect(tamilCa.strokeOrderSource?.variation).toMatch(
      /Frame 3 numbers three upper-frame movements.*lower-left bowl.*one continuous stroke.*in order without lifting.*comes back along the bar to the inner crossing.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*88% of the 176 stored prototypes of ச are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
