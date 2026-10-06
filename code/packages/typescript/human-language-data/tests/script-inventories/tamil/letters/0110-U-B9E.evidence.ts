import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-B9E",
  section: "letters",
  id: "U-B9E",
  digest: "e276dc7bf72a84d4b6b8a17187176e1e3cc512998e371ec5f5de572d16a6e329",
  assert(entry) {
    const tamilNya = entry as Letter;
    expect(tamilNya.sound).toBe("ña");
    expect(tamilNya.role).toBe("consonant");
    expect(tamilNya.strokeOrder).toEqual([
      "start inside the compact left body and curl round its small inner loop",
      "without lifting, climb its outer left side and curve over into the long top bar",
      "without lifting, carry the long top bar straight to the right",
      "without lifting, come back along the bar to the central upright and start down it",
      "without lifting, descend the central upright to its lower end",
      "without lifting, climb back up the upright and sweep around the broad outer-right curve",
      "without lifting, continue around the bottom bowl toward the far left",
      "without lifting, return up the outer-left side and finish below the top bar — and only now lift",
    ]);
    // One continuous stroke: the numbered movements are not pen lifts.
    expect(tamilNya.penLifts).toBe(0);
    for (const step of tamilNya.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilNya.strokeOrderNote).toMatch(
      /one stroke.*eight movements.*1–2.*inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*without lifting/i,
    );
    expect(tamilNya.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(tamilNya.strokeOrderSource?.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 8.*ஞ.*University of Texas at Austin.*p\. 194/i,
    );
    expect(tamilNya.strokeOrderSource?.variation).toMatch(
      /eight movements.*1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*one continuous stroke.*in order without lifting.*starts inside the small loop.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*86% of the 140 stored prototypes of ஞ are a single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  },
});
