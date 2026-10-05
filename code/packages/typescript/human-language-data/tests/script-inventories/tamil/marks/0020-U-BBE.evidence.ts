import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BBE",
  section: "marks",
  id: "U-BBE",
  digest: "041e66559b7ddfa20c9cbce2d39cbc1d70e905ba5b6001871d4923e741c91120",
  assert(entry) {
    const tamilAa = entry as Mark;
    // Written position: its own symbol, to the right, AFTER the
    // consonant. figure-targets.ts' WRITTEN_SIGN_SIDES is held to this record.
    expect(tamilAa.compositionOrder).toEqual(["write the Tamil consonant carrier first", "write the aa sign after it, as a symbol of its own to the right"]);
    expect(tamilAa.compositionSource?.url).toBe(
      "https://lipitk.sourceforge.net/docs/LipiRecognizers/lipi-indic-character-recognizers_4_0_user_manual.pdf",
    );
    expect(tamilAa.compositionSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0 User Manual.*Table 3.*distinct characters to the left or right of the base consonant.*written from left to right/,
    );
    expect(tamilAa.compositionSource?.variation).toMatch(
      /U\+0BBE, U\+0BBF, U\+0BC0, U\+0BC6, U\+0BC7 and U\+0BC8.*written linearly as a sequence of visually discrete symbols.*left of its consonant is written before it.*right of it is written after it/,
    );
    // One continuous stroke of three movements, from native writers' traces.
    expect(tamilAa.penLifts).toBe(0);
    expect(tamilAa.strokeOrder).toEqual([
      "start partway down the left upright and draw down to its foot",
      "without lifting, climb back up and carry the top bar to the right",
      "without lifting, come back to the right upright and draw it down — and only now lift",
    ]);
    expect(tamilAa.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilAa.strokeOrderSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 36 \(ா, aa sign\).*native Tamil writers.*MIT licence/,
    );
    expect(tamilAa.strokeOrderSource?.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil/);
  },
});
