import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC0",
  section: "marks",
  id: "U-BC0",
  digest: "7982eccb6453371e0c73c1859b7ac6d3f787543d9dadb8dd513eecbf678fe347",
  assert(entry) {
    const tamilIi = entry as Mark;
    expect(tamilIi.role).toBe("vowel-sign");
    expect(tamilIi.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "add the ī vowel sign to replace its inherent vowel",
    ]);
    expect(tamilIi.example).toEqual({ base: "ட", combined: "டீ", sound: "ṭī" });
    expect(tamilIi.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(tamilIi.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.6\.3.*Figure 12-21.*U\+0BC0.*ட \+ ீ → டீ.*ல \+ ீ → லீ/i,
    );
    expect(tamilIi.compositionSource?.variation).toMatch(
      /encoded carrier-first composition.*change shape or position.*join cursively.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
  },
});
