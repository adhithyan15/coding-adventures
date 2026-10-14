import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "japanese",
  withoutHook: 601,
  withoutVisible: 688,
  rationale: ["Migration baseline from HL24; Japanese's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
