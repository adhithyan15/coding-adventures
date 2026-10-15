import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "portuguese",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,142 / 1,141 baseline fell to 0 / 0. Every Portuguese word lesson now states its origin in a taken-apart block and matching etymology hook.",
    "The review used EtymDB 2.1 (Fourrier and Sagot 2020), CC BY-SA 4.0, with Lewis and Short for certain Latin forms EtymDB lacks. Sixty-two lessons state that the origin is uncertain or disputed; those are genuine open questions rather than placeholders.",
  ],
} as const satisfies EtymologyGapPinOwner;
