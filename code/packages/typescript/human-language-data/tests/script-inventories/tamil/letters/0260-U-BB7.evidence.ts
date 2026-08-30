import { expect } from "vitest";
import type { Letter } from "../../../src/types.js";
import { tamilInventoryEvidence } from "../../tamil-inventory-evidence.js";

export const scriptInventoryEvidence = tamilInventoryEvidence({
  name: "Tamil U-BB7",
  section: "letters",
  id: "U-BB7",
  digest: "d2474ea3d6d333d32b4bc72233dbede16383754c574cbbbeb78fa58c36647f7d",
  assert(entry) {
    const tamilSha = entry as Letter;
    expect(tamilSha.sound).toBe("ṣa");
    expect(tamilSha.role).toBe("consonant");
    expect(tamilSha.penLifts).toBe(3);
    expect(tamilSha.strokeOrder).toHaveLength(4);
    expect(tamilSha.strokeOrderSource?.url).toBe(
      "https://tamilnavarasam.in/Books/Others/Tamil_eng_hindi.pdf",
    );
    expect(tamilSha.strokeOrderSource?.citation).toMatch(
      /Narale.*Learn Tamil Through English\/Hindi.*Third Tamil Granthakshar ஷ.*p\. 13/i,
    );
    expect(tamilSha.strokeOrderSource?.variation).toMatch(
      /four separate pen-down runs.*Noto Sans Tamil.*varies by school/i,
    );
  },
});
