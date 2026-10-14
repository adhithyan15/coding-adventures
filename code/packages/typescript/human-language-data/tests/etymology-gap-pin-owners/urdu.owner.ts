import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "urdu",
  withoutHook: 1123,
  withoutVisible: 1137,
  rationale: ["Migration baseline from HL24; Urdu's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
