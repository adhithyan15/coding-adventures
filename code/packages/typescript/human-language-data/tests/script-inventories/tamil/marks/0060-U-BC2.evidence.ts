import { expect } from "vitest";
import type { Mark } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BC2",
  section: "marks",
  id: "U-BC2",
  digest: "149c12396c10a1cd97b30313e94832f7e8974a466540a5a6468b6340cabd3a02",
  assert(entry) {
    const tamilUu = entry as Mark;
    expect(tamilUu.role).toBe("vowel-sign");
    expect(tamilUu.compositionOrder).toEqual([
      "write the Tamil consonant carrier first",
      "add the ū vowel sign to replace its inherent vowel",
    ]);
    expect(tamilUu.example).toEqual({ base: "க", combined: "கூ", sound: "kū" });
    expect(tamilUu.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(tamilUu.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.6\.3.*U\+0BC2.*க \+ ூ → கூ/i,
    );
    expect(tamilUu.compositionSource?.variation).toMatch(
      /encoded carrier-first composition.*normally ligates.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
  },
});
