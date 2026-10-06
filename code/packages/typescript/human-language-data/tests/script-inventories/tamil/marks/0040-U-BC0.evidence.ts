import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC0",
  section: "marks",
  id: "U-BC0",
  digest: "3c67f7d92e8f1253d3cf63b185156d1192d5f701a3719d04a3791b5b19eee867",
  assert(entry) {
    const tamilIi = entry as Mark;
    expect(tamilIi.role).toBe("vowel-sign");
    expect(tamilIi.example).toEqual({ base: "ட", combined: "டீ", sound: "ṭī" });
    // Written position: its own symbol, to the right, AFTER the
    // consonant. figure-targets.ts' WRITTEN_SIGN_SIDES is held to this record.
    expect(tamilIi.compositionOrder).toEqual(["write the Tamil consonant carrier first", "write the ii sign after it, as a symbol of its own at the right"]);
    expect(tamilIi.compositionSource?.url).toBe(
      "https://lipitk.sourceforge.net/docs/LipiRecognizers/lipi-indic-character-recognizers_4_0_user_manual.pdf",
    );
    expect(tamilIi.compositionSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0 User Manual.*Table 3.*distinct characters to the left or right of the base consonant.*written from left to right/,
    );
    expect(tamilIi.compositionSource?.variation).toMatch(
      /U\+0BBE, U\+0BBF, U\+0BC0, U\+0BC6, U\+0BC7 and U\+0BC8.*written linearly as a sequence of visually discrete symbols.*left of its consonant is written before it.*right of it is written after it/,
    );
    // One continuous stroke of three movements, from native writers' traces.
    expect(tamilIi.penLifts).toBe(0);
    expect(tamilIi.strokeOrder).toEqual([
      "start at the foot of the tail and curl up to the left",
      "without lifting, arch over the top to the right",
      "without lifting, curl down and back into the small loop — and only now lift",
    ]);
    expect(tamilIi.strokeOrderSource?.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(tamilIi.strokeOrderSource?.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 38 \(ீ, ii sign\).*native Tamil writers.*MIT licence/,
    );
    expect(tamilIi.strokeOrderSource?.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil/);
  },
});
