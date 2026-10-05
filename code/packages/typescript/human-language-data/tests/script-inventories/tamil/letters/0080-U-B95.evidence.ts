import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B95",
  section: "letters",
  id: "U-B95",
  digest: "d4992a31bf47dd8689d187f8c24e84e2654b083c5b12b39730f21ec099abba3c",
  assert(entry) {
    const tamilKa = entry as Letter;
    expect(tamilKa.strokeOrder).toEqual([
      "start at the middle left and climb the left upright",
      "without lifting, carry the top bar to the right and return along it to the inner corner",
      "without lifting, drop the inner upright to the inner crossing",
      "without lifting, curve down and around the lower-left bowl",
      "without lifting, return up its outer left side to the middle left",
      "without lifting, cross the middle bar and turn around the lower-right bowl — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilKa.penLifts).toBe(0);
    for (const step of tamilKa.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilKa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilKa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*க.*p\. 191/i,
    );
    expect(tamilKa.strokeOrderSource?.variation).toMatch(
      /Frame 3 numbers six hand-movements.*one continuous stroke.*in order without lifting.*crosses the middle bar.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*88% of the 203 stored prototypes of க are a single pen-down stroke/i,
    );
  },
});
