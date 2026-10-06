import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC8",
  section: "marks",
  id: "U-BC8",
  digest: "0209083736bef271be0b35c75987f855565b131dbb718816018e09dcb091adc9",
  assert(entry) {
    const tamilAi = entry as Mark;
    // Written position: its own symbol, to the left, BEFORE the
    // consonant. figure-targets.ts' WRITTEN_SIGN_SIDES is held to this record.
    expect(tamilAi.compositionOrder).toEqual(["in handwriting, write the ai sign to the left before the primary consonant", "write the Tamil consonant carrier after it; read the result as consonant plus ai"]);
    expect(tamilAi.compositionSource?.url).toBe(
      "https://lipitk.sourceforge.net/docs/LipiRecognizers/lipi-indic-character-recognizers_4_0_user_manual.pdf",
    );
    expect(tamilAi.compositionSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0 User Manual.*Table 3.*distinct characters to the left or right of the base consonant.*written from left to right/,
    );
    expect(tamilAi.compositionSource?.variation).toMatch(
      /U\+0BBE, U\+0BBF, U\+0BC0, U\+0BC6, U\+0BC7 and U\+0BC8.*written linearly as a sequence of visually discrete symbols.*left of its consonant is written before it.*right of it is written after it/,
    );
    // One continuous stroke of three movements, from native writers' traces.
    expect(tamilAi.penLifts).toBe(0);
    expect(tamilAi.strokeOrder).toEqual([
      "start at the small left loop and circle it",
      "without lifting, climb and arch over to the right",
      "without lifting, rise through the middle and over the second arch — and only now lift",
    ]);
    expect(tamilAi.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilAi.strokeOrderSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 43 \(ை, ai sign\).*native Tamil writers.*MIT licence/,
    );
    expect(tamilAi.strokeOrderSource?.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil/);
  },
});
