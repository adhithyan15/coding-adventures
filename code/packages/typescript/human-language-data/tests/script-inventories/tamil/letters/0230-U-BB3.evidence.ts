import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB3",
  section: "letters",
  id: "U-BB3",
  digest: "721c29e7c489331b9ada806c341ab745141bc28234457158e607b66422f9ca2f",
  assert(entry) {
    const tamilRetroflexLa = entry as Letter;
    expect(tamilRetroflexLa.strokeOrder).toEqual([
      "start inside the loop, curl round its inner bowl, and climb the large outer loop",
      "without lifting, curve over the top and into the junction with the middle upright",
      "without lifting, draw the adjoining stem straight down",
      "without lifting, rise back up the same stem to the top",
      "without lifting, carry the top bar to the right edge",
      "without lifting, return along the bar to the right upright and draw it straight down — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilRetroflexLa.sound).toBe("ḷa");
    expect(tamilRetroflexLa.penLifts).toBe(0);
    for (const step of tamilRetroflexLa.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilRetroflexLa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRetroflexLa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 12.*ள.*University of Texas at Austin.*p\. 195/i,
    );
    expect(tamilRetroflexLa.strokeOrderSource?.variation).toMatch(
      /Module 12.*retroflex lateral.*contrasts it with ல.*Frame 12 numbers six hand-movements.*one continuous stroke.*in order without lifting.*HP Labs India.*90% of the 264 stored prototypes of ள are a single pen-down stroke/i,
    );
  },
});
