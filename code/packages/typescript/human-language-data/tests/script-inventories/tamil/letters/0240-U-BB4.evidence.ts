import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB4",
  section: "letters",
  id: "U-BB4",
  digest: "eb062a2686fe7686c5bcdb677e5f9b7e979583baca0e83b833ee07089efda7b9",
  assert(entry) {
    const tamilZha = entry as Letter;
    expect(tamilZha.strokeOrder).toEqual([
      "start at the lower left and climb the tall upright",
      "without lifting, retrace down the same upright",
      "without lifting, carry the low crossbar to the right, on round the bowl's foot, and up its right side",
      "without lifting, come back left over the top of the bowl into the inner upright",
      "without lifting, descend the inner upright into the stem below the baseline",
      "without lifting, run round the lower hook to its tip and back round it into the short exit — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilZha.penLifts).toBe(0);
    for (const step of tamilZha.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilZha.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilZha.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 7.*ழ.*p\. 193/i,
    );
    expect(tamilZha.strokeOrderSource?.variation).toMatch(
      /Frame 7 numbers six movements.*one continuous stroke.*in order without lifting.*Noto Sans Tamil.*low crossbar.*round the hook to its tip.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*97% of the 182 stored prototypes of ழ are a single pen-down stroke.*varies by school/i,
    );
  },
});
