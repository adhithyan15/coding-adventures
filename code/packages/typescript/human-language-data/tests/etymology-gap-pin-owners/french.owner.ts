import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "french",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,048 / 1,049 baseline fell to 0 / 0. All 1,045 generated word lessons from FR-C49-bras through FR-C257-anniversaire gained a taken-apart block and matching hook, and seven earlier hand-written lessons gained the half they lacked.",
    "Rows were cross-checked against EtymDB 2.1 (Fourrier and Sagot 2020), CC BY-SA 4.0, with Lewis and Short for textbook Latin forms EtymDB lacks. Eighteen lessons state that the origin is uncertain and give the usual account.",
  ],
} as const satisfies EtymologyGapPinOwner;
