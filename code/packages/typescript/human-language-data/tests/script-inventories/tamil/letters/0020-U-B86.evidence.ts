import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B86",
  section: "letters",
  id: "U-B86",
  digest: "9f09061e3601959c782cc75be0b60c850dc0bdfc35730fa59714b109c909b8a0",
  assert(entry) {
    const tamilAa = entry as Letter;
    expect(tamilAa.sound).toBe("ā");
    expect(tamilAa.role).toBe("independent-vowel");
    expect(tamilAa.penLifts).toBe(0);
    expect(tamilAa.strokeOrder).toHaveLength(7);
    expect(tamilAa.strokeOrder?.[4]).toMatch(
      /climb the right upright/i,
    );
    expect(tamilAa.strokeOrder?.[6]).toMatch(
      /long-vowel loop.*only now lift/i,
    );
    expect(tamilAa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilAa.strokeOrderSource?.citation).toMatch(
      /Frame 4.*ஆ.*p\. 192; drawn in one stroke after HP Labs India.*class 1 \(ஆ\)/i,
    );
    expect(tamilAa.strokeOrderSource?.variation).toMatch(
      /not evidence of one.*28 of the 29 stored prototypes of ஆ \(97%\) are one pen-down stroke.*overrides the old lift/i,
    );
  },
});
