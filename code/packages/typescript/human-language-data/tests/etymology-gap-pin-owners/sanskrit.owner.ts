import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "sanskrit",
  withoutHook: 1200,
  withoutVisible: 1099,
  rationale: ["Migration baseline from HL24; Sanskrit's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
