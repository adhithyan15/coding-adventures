import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "gujarati",
  withoutHook: 529,
  withoutVisible: 543,
  rationale: ["Migration baseline from HL24; Gujarati's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
