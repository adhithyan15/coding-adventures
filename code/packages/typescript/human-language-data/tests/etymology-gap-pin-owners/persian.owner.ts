import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "persian",
  withoutHook: 1300,
  withoutVisible: 1311,
  rationale: ["Migration baseline from HL24; Persian's remaining etymology debt has not yet been closed."],
} as const satisfies EtymologyGapPinOwner;
