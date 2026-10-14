import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "bengali",
  withoutHook: 1136,
  withoutVisible: 1144,
  rationale: ["Migration baseline from HL24; Bengali's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
