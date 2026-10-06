import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BBF",
  section: "marks",
  id: "U-BBF",
  digest: "434d16c7f524399216d887641db47cd5f85d50df6fda1ca1f84f9c71752e5448",
  assert(entry) {
    const tamilI = entry as Mark;
    // Written position: its own symbol, to the right, AFTER the
    // consonant. figure-targets.ts' WRITTEN_SIGN_SIDES is held to this record.
    expect(tamilI.compositionOrder).toEqual(["write the Tamil consonant carrier first", "write the i sign after it, as a symbol of its own at the right"]);
    expect(tamilI.compositionSource?.url).toBe(
      "https://lipitk.sourceforge.net/docs/LipiRecognizers/lipi-indic-character-recognizers_4_0_user_manual.pdf",
    );
    expect(tamilI.compositionSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0 User Manual.*Table 3.*distinct characters to the left or right of the base consonant.*written from left to right/,
    );
    expect(tamilI.compositionSource?.variation).toMatch(
      /U\+0BBE, U\+0BBF, U\+0BC0, U\+0BC6, U\+0BC7 and U\+0BC8.*written linearly as a sequence of visually discrete symbols.*left of its consonant is written before it.*right of it is written after it/,
    );
    // One continuous stroke of three movements, from native writers' traces.
    expect(tamilI.penLifts).toBe(0);
    expect(tamilI.strokeOrder).toEqual([
      "start at the tip of the hook and curl up to the left",
      "without lifting, arch over the top to the right",
      "without lifting, draw the stem down to the line — and only now lift",
    ]);
    expect(tamilI.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilI.strokeOrderSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 37 \(ி, i sign\).*native Tamil writers.*MIT licence/,
    );
    expect(tamilI.strokeOrderSource?.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil/);
  },
});
