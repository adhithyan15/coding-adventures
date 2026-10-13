import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B99",
  section: "letters",
  id: "U-B99",
  digest: "23112b2b6f1abdc6686576034a027eebb7e26d837c6bef5e77f108f4eec238e6",
  assert(entry) {
    const tamilNga = entry as Letter;
    expect(tamilNga.sound).toBe("ṅa");
    expect(tamilNga.role).toBe("consonant");
    expect(tamilNga.penLifts).toBe(0);
    expect(tamilNga.strokeOrder).toHaveLength(6);
    expect(tamilNga.strokeOrder?.[0]).toMatch(
      /top of the left upright.*straight down/i,
    );
    expect(tamilNga.strokeOrder?.[5]).toMatch(
      /climb the right upright.*only now lift/i,
    );
    expect(tamilNga.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilNga.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 2.*ங.*University of Texas at Austin.*p\. 191; reordered, and drawn in one stroke, after HP Labs India.*class 13 \(ங\)/i,
    );
    expect(tamilNga.strokeOrderSource?.variation).toMatch(
      /Frame 2.*detached part.*92 of the 108 stored prototypes of ங \(85%\) are one pen-down stroke.*overrides Frame 2's order and its lift.*varies by school/i,
    );
  },
});
