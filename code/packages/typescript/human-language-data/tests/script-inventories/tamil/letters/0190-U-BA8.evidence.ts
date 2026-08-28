import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BA8",
  section: "letters",
  id: "U-BA8",
  digest: "0fb8730053f097b1597a0dc8395e415d4d689c14e8997b0f89d50a72c403c896",
  assert(entry) {
    const tamilDentalNa = entry as Letter;
    expect(tamilDentalNa.sound).toBe("na");
    expect(tamilDentalNa.penLifts).toBe(2);
    expect(tamilDentalNa.strokeOrder).toHaveLength(6);
    expect(tamilDentalNa.strokeOrder?.[1]).toMatch(
      /top bar to the right.*lift once/i,
    );
    expect(tamilDentalNa.strokeOrder?.[3]).toMatch(
      /middle upright straight down.*lift a second time/i,
    );
    expect(tamilDentalNa.strokeOrder?.[5]).toMatch(
      /sweep left.*below-baseline tail/i,
    );
    expect(tamilDentalNa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilDentalNa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 5.*ந.*University of Texas at Austin.*p\. 193/i,
    );
    expect(tamilDentalNa.strokeOrderSource?.variation).toMatch(
      /Module 5.*voiced dental nasal.*extended final curve may be omitted.*six movements.*three pen-down runs.*1.?2.*3.?4.*5.?6/i,
    );
  },
});
