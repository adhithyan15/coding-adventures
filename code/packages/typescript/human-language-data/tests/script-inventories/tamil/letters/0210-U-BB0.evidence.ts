import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB0",
  section: "letters",
  id: "U-BB0",
  digest: "94b8964efb3c3ca6b9797b4d7becef566f3c8be1a08978b4e5a5f36ba476ad88",
  assert(entry) {
    const tamilRa = entry as Letter;
    expect(tamilRa.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down — then lift once",
      "set the pen at the top left and carry the top bar to the right — then lift a second time",
      "set the pen at the middle top and draw the central upright down",
      "without lifting again, add the short angular tail down-left and hook its tip down-right — and only now lift",
    ]);
    expect(tamilRa.penLifts).toBe(2);
    expect(tamilRa.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilRa.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 3.*ர/i,
    );
    expect(tamilRa.strokeOrderSource?.variation).toMatch(
      /three-movement ஈ frame.*angular short fourth movement.*varies by school.*three-run order.*Noto Sans Tamil/i,
    );
  },
});
