import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB0",
  section: "letters",
  id: "U-BB0",
  digest: "06cbf6ae672a56a1e3c20b846b324072a5f8b40967e02206f5094bfffe40b67c",
  assert(entry) {
    const tamilRa = entry as Letter;
    expect(tamilRa.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down",
      "without lifting, climb back up the upright and carry the top bar to the right",
      "without lifting, return along the bar to the middle and draw the central upright down",
      "without lifting, add the short angular tail down-left and hook its tip down-right — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilRa.penLifts).toBe(0);
    for (const step of tamilRa.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilRa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*ர/i,
    );
    expect(tamilRa.strokeOrderSource?.variation).toMatch(
      /three-movement ஈ frame.*angular short fourth movement.*one continuous stroke.*in order without lifting.*HP Labs India.*90% of the 189 stored prototypes of ர are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
