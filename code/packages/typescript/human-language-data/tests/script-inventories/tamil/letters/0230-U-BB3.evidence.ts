import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB3",
  section: "letters",
  id: "U-BB3",
  digest: "2f20267014c0d18cdb7b194ae8d37559709835e1a141481eacc0e8f3d6d43ff1",
  assert(entry) {
    const tamilRetroflexLa = entry as Letter;
    expect(tamilRetroflexLa.sound).toBe("ḷa");
    expect(tamilRetroflexLa.penLifts).toBe(2);
    expect(tamilRetroflexLa.strokeOrder).toHaveLength(6);
    expect(tamilRetroflexLa.strokeOrder?.[2]).toMatch(
      /adjoining stem straight down.*lift once/i,
    );
    expect(tamilRetroflexLa.strokeOrder?.[4]).toMatch(
      /top bar to the right edge.*lift a second time/i,
    );
    expect(tamilRetroflexLa.strokeOrder?.[5]).toMatch(
      /separate right upright.*straight down/i,
    );
    expect(tamilRetroflexLa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRetroflexLa.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 12.*ள.*University of Texas at Austin.*p\. 195/i,
    );
    expect(tamilRetroflexLa.strokeOrderSource?.variation).toMatch(
      /Module 12.*retroflex lateral.*contrasts it with ல.*six movements.*three pen-down runs.*1.?3.*4.?5.*movement 6/i,
    );
  },
});
