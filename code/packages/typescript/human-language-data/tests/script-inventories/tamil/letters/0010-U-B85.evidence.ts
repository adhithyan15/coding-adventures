import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B85",
  section: "letters",
  id: "U-B85",
  digest: "198b82cb84c37c6fb835c0c8fac71297b0a14ea39b9d1bb3ba4d9c2dade62cb6",
  assert(entry) {
    const tamilA = entry as Letter;
    expect(tamilA.sound).toBe("a");
    expect(tamilA.role).toBe("independent-vowel");
    expect(tamilA.penLifts).toBe(0);
    expect(tamilA.strokeOrder).toHaveLength(6);
    expect(tamilA.strokeOrder?.[4]).toMatch(
      /climb the right upright/i,
    );
    expect(tamilA.strokeOrder?.[5]).toMatch(
      /straight down over that ink.*only now lift/i,
    );
    expect(tamilA.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilA.strokeOrderSource?.citation).toMatch(
      /Frame 4.*அ.*p\. 192; drawn in one stroke after HP Labs India.*class 0 \(அ\)/i,
    );
    expect(tamilA.strokeOrderSource?.variation).toMatch(
      /not evidence of one.*104 of the 114 stored prototypes of அ \(91%\) are one pen-down stroke.*overrides the old lift/i,
    );
  },
});
