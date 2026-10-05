import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB1",
  section: "letters",
  id: "U-BB1",
  digest: "8704d07716eddf73a3f6db04ec321cd384197842a908ff5016b0ddf9525d2660",
  assert(entry) {
    const tamilRra = entry as Letter;
    expect(tamilRra.strokeOrder).toEqual([
      "start at the lower left, climb the left side, and arch to the middle",
      "without lifting, descend the middle upright",
      "without lifting, climb back up the same upright",
      "without lifting, arch over and descend the right side",
      "without lifting, sweep left below the baseline and drop into the long descender — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilRra.penLifts).toBe(0);
    for (const step of tamilRra.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilRra.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRra.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 10.*ற.*p\. 194/i,
    );
    expect(tamilRra.strokeOrderSource?.variation).toMatch(
      /Frame 10 numbers five hand-movements.*one continuous stroke.*in order without lifting.*HP Labs India.*99% of the 378 stored prototypes of ற are a single pen-down stroke/i,
    );
  },
});
