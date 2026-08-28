import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B9F",
  section: "letters",
  id: "U-B9F",
  digest: "3bb81be34c48d4befeec8acdf4e6fdcc4bc9c6e41221b18cab88be36eb91659f",
  assert(entry) {
    const tamilTta = entry as Letter;
    expect(tamilTta.strokeOrder).toEqual([
      "start at the top left and draw the left upright straight down",
      "without lifting, turn right and carry the long foot to the far edge — and only now lift",
    ]);
    expect(tamilTta.penLifts).toBe(0);
    expect(tamilTta.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilTta.strokeOrderSource?.citation).toMatch(
      /Appendix I.*Frame 1.*ட.*p\. 190/i,
    );
    expect(tamilTta.strokeOrderSource?.variation).toMatch(
      /left descent.*rightward foot.*two joined movements.*Module 1 identifies.*top-to-bottom.*left-to-right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  },
});
