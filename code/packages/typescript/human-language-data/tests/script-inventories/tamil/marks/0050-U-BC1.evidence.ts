import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC1",
  section: "marks",
  id: "U-BC1",
  digest: "e868e2beecec973018bd689d87c161646cc156e18dc74a35040dd078a86fd8cd",
  assert(entry) {
    const tamilU = entry as Mark;
    expect(tamilU.role).toBe("vowel-sign");
    expect(tamilU.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "add the u vowel sign to replace its inherent vowel",
    ]);
    expect(tamilU.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(tamilU.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.6\.3.*U\+0BC1.*க \+ ு → கு/i,
    );
    expect(tamilU.compositionSource?.variation).toMatch(
      /encoded carrier-first composition.*normally ligates.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
  },
});
