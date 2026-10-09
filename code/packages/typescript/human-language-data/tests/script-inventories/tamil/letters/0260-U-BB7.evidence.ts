import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB7",
  section: "letters",
  id: "U-BB7",
  digest: "f1b0882d6fb7d384c7f45f263279e23740c129d3e7ba8c0a6fdb5b5a4d06aa0f",
  assert(entry) {
    const tamilSha = entry as Letter;
    expect(tamilSha.sound).toBe("ṣa");
    expect(tamilSha.role).toBe("consonant");
    expect(tamilSha.penLifts).toBe(0);
    expect(tamilSha.strokeOrder).toHaveLength(6);
    expect(tamilSha.strokeOrder?.[0]).toMatch(
      /tip inside the small loop.*clockwise/i,
    );
    expect(tamilSha.strokeOrder?.[5]).toMatch(
      /down the tail.*only now lift/i,
    );
    expect(tamilSha.strokeOrderSource?.url).toBe(
      "https://tamilnavarasam.in/Books/Others/Tamil_eng_hindi.pdf",
    );
    expect(tamilSha.strokeOrderSource?.citation).toMatch(
      /Narale.*Learn Tamil Through English\/Hindi.*Third Tamil Granthakshar ஷ.*p\. 13; drawn in one stroke after HP Labs India.*class 31 \(ஷ\)/i,
    );
    expect(tamilSha.strokeOrderSource?.variation).toMatch(
      /four parts.*not evidence of a lift.*175 of the 188 stored prototypes of ஷ \(93%\) are one pen-down stroke.*overrides the four runs.*varies by school/i,
    );
  },
});
