import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "arabic",
  withoutHook: 1187,
  withoutVisible: 1195,
  rationale: ["Migration baseline from HL24; Arabic's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
