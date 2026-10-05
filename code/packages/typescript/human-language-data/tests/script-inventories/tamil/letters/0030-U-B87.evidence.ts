import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B87",
  section: "letters",
  id: "U-B87",
  digest: "c94694545c9b991d7a95501a7503e6278f5e6a7422ddec5ec945e5ed96c2f9c6",
  assert(entry) {
    const tamilI = entry as Letter;
    expect(tamilI.strokeOrder).toEqual([
      "start inside the upper curl and sweep around it",
      "without lifting, sweep down the inner right curve",
      "without lifting, carry left down the lower diagonal and turn around the lower-left loop",
      "without lifting, climb the loop's outer side back to the left crossing",
      "without lifting, carry right across the middle and turn around the lower-right loop",
      "without lifting, climb the other diagonal back across the middle and on up the outer left side",
      "without lifting, arch over the top and down the far right — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilI.penLifts).toBe(0);
    for (const step of tamilI.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilI.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilI.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 4.*இ.*p\. 192/i,
    );
    expect(tamilI.strokeOrderSource?.variation).toMatch(
      /no single national stroke-order standard.*Frame 4 numbers seven hand-movements for இ.*one continuous stroke.*in order without lifting.*no movement retraces ink.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*100% of the 30 stored prototypes of இ are a single pen-down stroke/i,
    );
  },
});
