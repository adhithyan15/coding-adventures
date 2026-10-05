import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BA4",
  section: "letters",
  id: "U-BA4",
  digest: "949554a2bccb671026bd837de2b1bf1a1ebf7e83c20311794e9cba46e5f3698f",
  assert(entry) {
    const tamilTha = entry as Letter;
    expect(tamilTha.strokeOrder).toEqual([
      "start at the middle left and climb the short upright",
      "without lifting, carry the top bar to the central upright",
      "without lifting, carry the short upper bar to the right",
      "without lifting, turn back along the bar, drop down the central upright, and curve around the broad right bowl",
      "without lifting, climb back around the right bowl, cross the middle to the left, and turn around the compact left loop",
      "without lifting, curl back to the central crossing",
      "without lifting, follow the right bowl down again and sweep the low tail left — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilTha.penLifts).toBe(0);
    for (const step of tamilTha.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilTha.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilTha.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*த.*p\. 192/i,
    );
    expect(tamilTha.strokeOrderSource?.variation).toMatch(
      /Module 3 identifies.*dental stop.*final Frame 3 row numbers seven hand-movements.*1–2.*upper frame.*3–4.*right bowl.*5–6.*left loop.*7.*leftward tail.*one continuous stroke.*in order without lifting.*retraces the right bowl.*HP Labs India.*88% of the 266 stored prototypes of த are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
