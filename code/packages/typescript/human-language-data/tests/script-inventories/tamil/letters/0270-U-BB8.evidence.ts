import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB8",
  section: "letters",
  id: "U-BB8",
  digest: "1816191361fdc577854846cc406026eda7cc71caa0eba516c5c9678c6179e84f",
  assert(entry) {
    const tamilSa = entry as Letter;
    expect(tamilSa.sound).toBe("sa");
    expect(tamilSa.role).toBe("consonant");
    // One continuous stroke, as 150 of the 153 native prototypes draw it.
    expect(tamilSa.penLifts).toBe(0);
    expect(tamilSa.strokeOrder).toHaveLength(7);
    for (const step of tamilSa.strokeOrder.slice(1)) {
      expect(step).toMatch(/^without lifting, /);
    }
    expect(tamilSa.strokeOrder.at(-1)).toMatch(/ — and only now lift$/);
    expect(tamilSa.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilSa.strokeOrderSource?.citation).toBe(
      "HP Labs India, Lipi Toolkit, Lipi Indic Character Recognizers 4.0, Tamil recognizer, class 30 (ஸ, sa): stored online-handwriting prototypes from native writers (MIT licence, 2012)",
    );
    // Counts and shares only, never a copied trace.
    expect(tamilSa.strokeOrderSource?.variation).toMatch(
      /150 of the 153 stored prototypes of ஸ \(98%\) are one pen-down stroke.*only counts and shares are cited here, and no trace was copied/,
    );
  },
});
