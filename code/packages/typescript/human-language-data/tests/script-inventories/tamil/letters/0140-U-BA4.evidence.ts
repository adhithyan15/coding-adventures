import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BA4",
  section: "letters",
  id: "U-BA4",
  digest: "9ba9360d31e1e9bf39e784269361bfd91cb42f49fcee9e245b3205af0babcc4c",
  assert(entry) {
    const tamilTha = entry as Letter;
    expect(tamilTha.strokeOrder).toEqual([
      "start at the middle left, climb the short upright, and carry the top bar to the right — then lift once",
      "restart at the central crossing, carry the short upper bar right, and curve down around the broad right bowl — then lift a second time",
      "restart at the middle left, turn around the compact left loop, and curl back to the central crossing — then lift a third time",
      "restart at the lower right and sweep the low tail left",
    ]);
    expect(tamilTha.penLifts).toBe(3);
    expect(tamilTha.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilTha.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*த.*p\. 192/i,
    );
    expect(tamilTha.strokeOrderSource?.variation).toMatch(
      /Module 3 identifies.*dental stop.*final Frame 3 row.*four separate pen-down runs.*1–2.*upper frame.*3–4.*right bowl.*5–6.*left loop.*movement 7.*leftward tail.*varies by school.*four-run order.*Noto Sans Tamil/i,
    );
  },
});
